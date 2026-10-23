//! Binary FBX 7.x: the models under the document's root, as a soup.
//!
//! Models are walked depth first from the root, children in connection order, each with
//! FBX's full transform stack composed with its parent's in f64; a model's geometric
//! transform moves its own geometry only. The file's axis system is turned to Y up,
//! right-handed; units stay the file's. A transform that mirrors space reverses its
//! triangles' winding. Each (model, material slot with polygons) pair of a polygon mesh
//! is one source primitive, numbered from 0 in walk order and then slot order, and
//! `vertex` is the control-point index; every primitive carries all of its mesh's control
//! points. A slot's material is the model's connected material of that index; the names
//! are the file's materials in order, an unnamed one called `material N`. Polygons are
//! triangulated as OBJ's are. The first UV layer gives the corner UVs; `extra_uv_sets` is
//! the most further layers on any drawn mesh. Normals, textures and the templates in
//! `Definitions` are not read. Deformers, animation stacks and geometry that is not a
//! polygon mesh are not applied: they are listed in [`Imported::ignored`].

mod document;
mod mesh;
mod transform;
mod tree;

use std::collections::BTreeMap;

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{MaterialIndex, SourceCorner, SourceMaterials, TriangleSoup};

use crate::affine::{self, Mat};
use crate::triangulate::triangulate;
use crate::{Format, Ignored, Imported, IoError};
use document::{Document, Object};
use mesh::Mesh;
use transform::Properties;

/// Reads binary FBX 7.x bytes into a soup.
///
/// Errors: [`IoError::Parse`] for ASCII FBX (its detail says so), bytes that are not
/// binary FBX 7.x or that end early, an object or connection the reader cannot read, a
/// model reached twice, a transform property that is not numbers, a rotation order or
/// axis system outside FBX's, a mesh index out of range, a polygon of under three or over
/// 1000 corners or one that crosses itself, a material slot with no material, a UV layer
/// it cannot read or a non-finite UV; [`IoError::NonFiniteInput`] naming every drawn
/// control point with a non-finite coordinate.
pub fn read(bytes: &[u8]) -> Result<Imported, IoError> {
    let top = tree::parse(bytes).map_err(parse)?;
    let doc = Document::new(&top).map_err(parse)?;
    let mut builder = Builder::default();
    for (model, world) in doc.models().map_err(parse)? {
        builder.model(&doc, &model, &world).map_err(parse)?;
    }
    builder
        .ignored
        .extend(doc.animation_stacks().map(|s| Ignored::AnimationStack {
            stack: s.name.into(),
        }));
    builder.finish(doc.material_names())
}

fn parse(detail: String) -> IoError {
    IoError::Parse {
        format: Format::Fbx,
        detail,
    }
}

/// The soup's arrays as primitives are added, and what was not applied.
#[derive(Default)]
struct Builder {
    positions: Vec<DVec3>,
    triangles: Vec<[u32; 3]>,
    sources: Vec<SourceCorner>,
    uvs: Vec<Option<DVec2>>,
    has_uvs: bool,
    extra_uv_sets: u32,
    materials: Vec<Option<MaterialIndex>>,
    non_finite: Vec<SourceCorner>,
    ignored: Vec<Ignored>,
}

impl Builder {
    /// Adds the model's polygon meshes and lists its deformers and other geometry.
    fn model(&mut self, doc: &Document<'_>, model: &Object<'_>, world: &Mat) -> Result<(), String> {
        let slots = doc.slots(model);
        let geometric = transform::geometric(Properties::of(Some(model.node)))
            .map_err(|e| format!("model \"{}\": {e}", model.name))?;
        let at = affine::mul(world, &geometric);
        for geometry in doc.connected(model.id, "Geometry") {
            let (m, g) = (model.name.to_owned(), geometry.name.to_owned());
            if geometry.subclass != "Mesh" {
                self.ignored.push(Ignored::Geometry {
                    model: m,
                    geometry: g,
                });
                continue;
            }
            self.ignored
                .extend(
                    doc.connected(geometry.id, "Deformer")
                        .map(|d| Ignored::Deformer {
                            model: m.clone(),
                            deformer: d.name.into(),
                        }),
                );
            let mesh = mesh::read(geometry.node).map_err(|e| format!("geometry \"{g}\": {e}"))?;
            self.mesh(&mesh, &at, &slots)
                .map_err(|e| format!("geometry \"{g}\" on model \"{m}\": {e}"))?;
        }
        Ok(())
    }

    /// One primitive per material slot that has polygons, in slot order. Without a
    /// material layer every polygon is in slot 0.
    fn mesh(&mut self, mesh: &Mesh, at: &Mat, slots: &[MaterialIndex]) -> Result<(), String> {
        let mut by_slot: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for p in 0..mesh.polygon_count() {
            // INVARIANT: the mesh reader gives every polygon a slot.
            let slot = mesh.slots.as_ref().map_or(0, |s| s[p]);
            by_slot.entry(slot).or_default().push(p);
        }
        let world: Vec<DVec3> = mesh.points.iter().map(|&p| affine::apply(at, p)).collect();
        let mirrored = affine::mirrors(at);
        for (slot, polygons) in by_slot {
            let material = match slots.get(slot as usize) {
                Some(&m) => Some(m),
                None if mesh.slots.is_none() => None,
                None => return Err(format!("material slot {slot} has no material connected")),
            };
            self.primitive(mesh, &world, &polygons, mirrored)?;
            self.materials.push(material);
        }
        self.has_uvs |= mesh.uvs.is_some();
        self.extra_uv_sets = self.extra_uv_sets.max(mesh.extra_uv_sets);
        Ok(())
    }

    /// The polygons' triangles over a copy of all of `world`, the mesh's control points
    /// moved into place.
    fn primitive(
        &mut self,
        mesh: &Mesh,
        world: &[DVec3],
        polygons: &[usize],
        mirrored: bool,
    ) -> Result<(), String> {
        let id = u32::try_from(self.materials.len()).map_err(|_| "too many primitives")?;
        let total = self.positions.len().checked_add(world.len());
        if total.and_then(|t| u32::try_from(t).ok()).is_none() {
            return Err("too many vertices".into());
        }
        // Lossless: the total fits in u32 (checked above).
        let base = self.positions.len() as u32;
        for (v, p) in world.iter().enumerate() {
            if !p.is_finite() {
                // Lossless: the mesh reader checked the control points fit in u32.
                self.non_finite.push(SourceCorner {
                    primitive: id,
                    vertex: v as u32,
                });
            }
        }
        self.positions.extend_from_slice(world);
        if !self.non_finite.is_empty() {
            // The read fails with `NonFiniteInput`, and a non-finite polygon has no split.
            return Ok(());
        }
        for &p in polygons {
            // INVARIANT: the mesh reader checked every polygon's range, every control point
            // index and the UV count against `corners`.
            let range = mesh.starts[p]..mesh.starts[p + 1];
            let corners = &mesh.corners[range.clone()];
            let points: Vec<DVec3> = corners.iter().map(|&c| world[c as usize]).collect();
            let split = triangulate(&points)
                .ok_or_else(|| format!("polygon {p} crosses itself, so it has no triangulation"))?;
            for mut t in split {
                if mirrored {
                    t.swap(1, 2);
                }
                self.triangles.push(t.map(|k| base + corners[k]));
                for k in t {
                    self.sources.push(SourceCorner {
                        primitive: id,
                        vertex: corners[k],
                    });
                    self.uvs.push(mesh.uvs.as_ref().map(|u| u[range.start + k]));
                }
            }
        }
        Ok(())
    }

    fn finish(self, names: Vec<String>) -> Result<Imported, IoError> {
        if !self.non_finite.is_empty() {
            return Err(IoError::NonFiniteInput {
                vertices: self.non_finite,
            });
        }
        let uvs = self.has_uvs.then_some(self.uvs);
        let soup = TriangleSoup::new(self.positions, self.triangles, self.sources, uvs)?
            .with_materials(
                SourceMaterials::new(names, self.materials),
                self.extra_uv_sets,
            )?;
        Ok(Imported {
            soup,
            ignored: self.ignored,
        })
    }
}
