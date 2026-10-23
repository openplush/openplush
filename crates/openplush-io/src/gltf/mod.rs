//! glTF 2.0 and GLB: the scene a renderer would draw, as a soup.
//!
//! The file's `scene` is drawn, else scene 0, with every node's world transform applied to
//! positions in f64; a node that mirrors space has its triangles' winding reversed. Each
//! drawn (node, triangle primitive) pair is one source primitive, numbered from 0 in walk
//! order, so a mesh drawn twice is two. `vertex` is the index into that primitive's
//! accessors. A primitive's material is its own `material`, none for glTF's default
//! material; the names are the file's materials list in order, an unnamed one called
//! `material N`. `TEXCOORD_0` gives the corner UVs; `extra_uv_sets` is the most sets
//! beyond it on any drawn primitive.
//! Normals, tangents and images are not read. Skins, morph targets, animations and point or
//! line primitives are not applied: they are listed in [`Imported::ignored`].

mod accessor;
mod buffers;
mod scene;

use std::borrow::Cow;

use ::gltf::mesh::Mode;
use ::gltf::{Document, Node, Primitive, Semantic};
use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{MaterialIndex, SourceCorner, SourceMaterials, TriangleSoup};

use crate::affine::{self, Mat};
use crate::{Format, Ignored, Imported, IoError};
use accessor::Use;

/// Reads glTF or GLB bytes (GLB when they start with its magic) into a soup. `name` is the
/// file's name in the import set; `sidecar` looks up the files beside it by name.
///
/// Errors: [`IoError::Parse`] for a file the `gltf` crate refuses (including a required
/// extension it does not implement), no scene, a node reached twice, a node rotation that is
/// not a unit quaternion, a sparse accessor, an accessor of the wrong type, out of its view
/// or with a stride shorter than its element, an index past its primitive's last vertex, a
/// non-finite `TEXCOORD_0` value, a short buffer, or a reference outside the supplied files;
/// [`IoError::MissingResource`] for a referenced file that was not supplied;
/// [`IoError::NonFiniteInput`] naming every drawn vertex with a non-finite coordinate.
pub fn read<'a>(
    name: &str,
    bytes: &[u8],
    sidecar: &dyn Fn(&str) -> Option<&'a [u8]>,
) -> Result<Imported, IoError> {
    let format = if bytes.starts_with(b"glTF") {
        Format::Glb
    } else {
        Format::Gltf
    };
    let parse = |detail: String| IoError::Parse { format, detail };
    let file = ::gltf::Gltf::from_slice(bytes).map_err(|e| parse(e.to_string()))?;
    let beside = |n: &str| -> Option<&[u8]> { sidecar(n) };
    let buffers = buffers::resolve(&file.document, file.blob.as_deref(), name, &beside, format)?;
    let mut soup = Builder::default();
    let mut ignored = Vec::new();
    for (node, world) in scene::walk(&file.document).map_err(parse)? {
        soup.node(&node, &world, &buffers, &mut ignored)
            .map_err(parse)?;
    }
    ignored.extend(file.document.animations().map(|a| Ignored::Animation {
        animation: index(a.index()),
    }));
    Ok(Imported {
        soup: soup.finish(&file.document)?,
        ignored,
    })
}

/// The soup's arrays as primitives are added.
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
}

impl Builder {
    fn node(
        &mut self,
        node: &Node<'_>,
        world: &Mat,
        buffers: &[Cow<'_, [u8]>],
        ignored: &mut Vec<Ignored>,
    ) -> Result<(), String> {
        let Some(mesh) = node.mesh() else {
            return Ok(());
        };
        let (n, m) = (index(node.index()), index(mesh.index()));
        if node.skin().is_some() {
            ignored.push(Ignored::Skin { node: n });
        }
        if mesh
            .primitives()
            .any(|p| p.morph_targets().next().is_some())
        {
            ignored.push(Ignored::MorphTargets { node: n, mesh: m });
        }
        for primitive in mesh.primitives() {
            if matches!(
                primitive.mode(),
                Mode::Triangles | Mode::TriangleStrip | Mode::TriangleFan
            ) {
                self.primitive(&primitive, world, buffers)
                    .map_err(|e| format!("mesh {m} primitive {}: {e}", primitive.index()))?;
            } else {
                ignored.push(Ignored::Primitive {
                    node: n,
                    mesh: m,
                    primitive: index(primitive.index()),
                });
            }
        }
        Ok(())
    }

    fn primitive(
        &mut self,
        primitive: &Primitive<'_>,
        world: &Mat,
        buffers: &[Cow<'_, [u8]>],
    ) -> Result<(), String> {
        let id = u32::try_from(self.materials.len()).map_err(|_| "too many primitives")?;
        let positions = primitive
            .get(&Semantic::Positions)
            .ok_or("no POSITION attribute")?;
        let points = accessor::floats(&positions, buffers, Use::Position)?;
        let count = points.len() / 3;
        let uvs = texcoords(primitive, buffers, count)?;
        let indices = match primitive.indices() {
            Some(a) => accessor::indices(&a, buffers)?,
            None => (0..u32::try_from(count).map_err(|_| "too many vertices")?).collect(),
        };
        if let Some(&i) = indices.iter().find(|&&i| i as usize >= count) {
            return Err(format!("index {i} is past its last vertex"));
        }
        let mut triangles = triangles(primitive.mode(), &indices)?;
        if affine::mirrors(world) {
            for t in &mut triangles {
                t.swap(1, 2);
            }
        }
        let base = u32::try_from(self.positions.len()).map_err(|_| "too many vertices")?;
        for (v, p) in points.chunks_exact(3).enumerate() {
            let at = affine::apply(world, [p[0], p[1], p[2]]);
            if !(at.x.is_finite() && at.y.is_finite() && at.z.is_finite()) {
                self.non_finite.push(corner(id, v));
            }
            self.positions.push(at);
        }
        for t in triangles {
            self.triangles.push(t.map(|i| base + i));
            for i in t {
                self.sources.push(SourceCorner {
                    primitive: id,
                    vertex: i,
                });
                let i = i as usize;
                self.uvs
                    .push(uvs.as_ref().map(|t| DVec2::new(t[2 * i], t[2 * i + 1])));
            }
        }
        self.has_uvs |= uvs.is_some();
        let sets = primitive
            .attributes()
            .filter(|(s, _)| matches!(s, Semantic::TexCoords(_)))
            .count();
        // Every set but the one read counts, TEXCOORD_0's absence included.
        let extra = u32::try_from(sets - usize::from(uvs.is_some())).unwrap_or(u32::MAX);
        self.extra_uv_sets = self.extra_uv_sets.max(extra);
        self.materials.push(
            primitive
                .material()
                .index()
                .map(|i| MaterialIndex::new(index(i))),
        );
        Ok(())
    }

    fn finish(self, doc: &Document) -> Result<TriangleSoup, IoError> {
        if !self.non_finite.is_empty() {
            return Err(IoError::NonFiniteInput {
                vertices: self.non_finite,
            });
        }
        let names = doc
            .materials()
            .enumerate()
            .map(|(i, m)| {
                m.name()
                    .map_or_else(|| format!("material {i}"), str::to_owned)
            })
            .collect();
        let uvs = self.has_uvs.then_some(self.uvs);
        let soup = TriangleSoup::new(self.positions, self.triangles, self.sources, uvs)?;
        Ok(soup.with_materials(
            SourceMaterials::new(names, self.materials),
            self.extra_uv_sets,
        )?)
    }
}

/// The triangles `mode` makes of `indices`, as indices into the primitive's vertices.
fn triangles(mode: Mode, indices: &[u32]) -> Result<Vec<[u32; 3]>, String> {
    let n = indices.len();
    Ok(match mode {
        Mode::TriangleStrip => (2..n)
            .map(|k| {
                let [a, b, c] = [indices[k - 2], indices[k - 1], indices[k]];
                // The spec flips every other triangle so the strip keeps one winding.
                if k % 2 == 0 { [a, b, c] } else { [a, c, b] }
            })
            .collect(),
        Mode::TriangleFan => (2..n)
            .map(|k| [indices[0], indices[k - 1], indices[k]])
            .collect(),
        _ if !n.is_multiple_of(3) => {
            return Err(format!(
                "a triangle list of {n} indices, which is not a multiple of 3"
            ));
        }
        _ => indices
            .chunks_exact(3)
            .map(|t| [t[0], t[1], t[2]])
            .collect(),
    })
}

fn corner(primitive: u32, vertex: usize) -> SourceCorner {
    SourceCorner {
        primitive,
        // Lossless: the vertex count fits in u32 (checked when the indices were built).
        vertex: vertex as u32,
    }
}

/// A file index as u32; the `gltf` crate reads every index from a u32 in the JSON.
fn index(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

/// The primitive's `TEXCOORD_0`, two values per vertex, if it has one.
fn texcoords(
    primitive: &Primitive<'_>,
    buffers: &[Cow<'_, [u8]>],
    count: usize,
) -> Result<Option<Vec<f64>>, String> {
    let Some(a) = primitive.get(&Semantic::TexCoords(0)) else {
        return Ok(None);
    };
    let uvs = accessor::floats(&a, buffers, Use::TexCoord)?;
    if uvs.len() / 2 != count {
        return Err("TEXCOORD_0 and POSITION differ in length".into());
    }
    if let Some(v) = uvs
        .chunks_exact(2)
        .position(|uv| !(uv[0].is_finite() && uv[1].is_finite()))
    {
        return Err(format!("vertex {v} has a non-finite texture coordinate"));
    }
    Ok(Some(uvs))
}
