use openplush_geom::{DVec2, DVec3};

use crate::{CornerArray, Counted, FaceId, MeshError, VertexId};

/// Where a triangle corner came from in the source file. Provenance survives
/// repair, so a saved project can find the same corner again after re-import
/// The importer owns the numbering and must keep it stable for
/// the same file:
/// - `primitive` is unique across the whole file. For glTF it counts each drawn
///   instance (node × mesh × primitive) separately, because one mesh instanced by
///   two nodes gives two different pieces of surface.
/// - `vertex` indexes that primitive's vertex array. For OBJ, whose indices are
///   file-global and separate per attribute, it is the position (`v`) index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceCorner {
    /// Source primitive (drawn instance) index, unique within the file.
    pub primitive: u32,
    /// Source vertex index within the primitive (OBJ: the position index).
    pub vertex: u32,
}

/// Indexed triangles as imported, before any repair.
///
/// **Guarantees**: every coordinate is finite, every index is
/// in range, and every corner has a provenance record (and optionally a UV).
/// **Does not guarantee** anything about mesh quality: repeated indices, duplicate
/// or zero-area faces, non-manifold edges, and unwelded seams pass through untouched,
/// because detecting and reporting them is validation's job.
#[derive(Debug, Clone, PartialEq)]
pub struct TriangleSoup {
    positions: Vec<DVec3>,
    triangles: Vec<[u32; 3]>,
    corner_src: Vec<SourceCorner>,
    corner_uv: Option<Vec<Option<DVec2>>>,
}

impl TriangleSoup {
    /// Builds a soup. `corner_src` (and `corner_uv`, if given) hold three entries per
    /// triangle, in triangle order then corner order.
    ///
    /// Errors, in the order they are checked: [`MeshError::TooLarge`] (vertices,
    /// then triangles), [`MeshError::CornerCountMismatch`] (sources, then UVs),
    /// [`MeshError::NonFiniteVertex`], [`MeshError::IndexOutOfRange`]. Within each,
    /// the lowest-indexed offender is reported.
    ///
    /// The triangle cap is `u32`, so a soup can hold more corners than a
    /// `ManifoldMesh` has half-edge IDs for; building the mesh checks its own
    /// half-edge count.
    pub fn new(
        positions: Vec<DVec3>,
        triangles: Vec<[u32; 3]>,
        corner_src: Vec<SourceCorner>,
        corner_uv: Option<Vec<Option<DVec2>>>,
    ) -> Result<Self, MeshError> {
        // Checked in this order, so the same input always reports the same error:
        // sizes and array lengths (O(1)), then coordinates, then indices.
        let vertex_count = positions.len();
        if u32::try_from(vertex_count).is_err() {
            return Err(MeshError::TooLarge {
                what: Counted::Vertices,
                count: vertex_count,
            });
        }
        if u32::try_from(triangles.len()).is_err() {
            return Err(MeshError::TooLarge {
                what: Counted::Triangles,
                count: triangles.len(),
            });
        }
        let expected = triangles.len() * 3;
        if corner_src.len() != expected {
            return Err(MeshError::CornerCountMismatch {
                array: CornerArray::Sources,
                expected,
                found: corner_src.len(),
            });
        }
        if let Some(uv) = &corner_uv
            && uv.len() != expected
        {
            return Err(MeshError::CornerCountMismatch {
                array: CornerArray::Uvs,
                expected,
                found: uv.len(),
            });
        }
        for (i, p) in positions.iter().enumerate() {
            if !p.is_finite() {
                // Lossless: vertex_count fits in u32 (checked above).
                return Err(MeshError::NonFiniteVertex {
                    vertex: VertexId::new(i as u32),
                });
            }
        }
        for (triangle, tri) in triangles.iter().enumerate() {
            if let Some(&index) = tri.iter().find(|&&i| i as usize >= vertex_count) {
                return Err(MeshError::IndexOutOfRange {
                    // Lossless: the triangle count fits in u32 (checked above).
                    triangle: FaceId::new(triangle as u32),
                    index,
                    vertex_count,
                });
            }
        }
        Ok(Self {
            positions,
            triangles,
            corner_src,
            corner_uv,
        })
    }

    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// The position of `v`, or `None` if it does not exist.
    pub fn position(&self, v: VertexId) -> Option<DVec3> {
        self.positions.get(v.index() as usize).copied()
    }

    /// All positions, in vertex order.
    pub fn positions(&self) -> &[DVec3] {
        &self.positions
    }

    /// The vertices of triangle `f`, or `None` if it does not exist.
    pub fn triangle(&self, f: FaceId) -> Option<[VertexId; 3]> {
        let t = usize::try_from(f.index()).ok()?;
        self.triangles.get(t).map(|tri| tri.map(VertexId::new))
    }

    /// Provenance of corner `corner` (0..3, in the triangle's vertex order) of
    /// triangle `f`; `None` if either is out of range.
    pub fn corner_source(&self, f: FaceId, corner: usize) -> Option<SourceCorner> {
        self.corner_src.get(corner_slot(f, corner)?).copied()
    }

    /// UV of corner `corner` of triangle `f`; `None` if absent or out of range.
    pub fn corner_uv(&self, f: FaceId, corner: usize) -> Option<DVec2> {
        self.corner_uv
            .as_ref()?
            .get(corner_slot(f, corner)?)
            .copied()
            .flatten()
    }

    /// Raw triangle indices, all in range (checked in [`TriangleSoup::new`]).
    pub(crate) fn triangle_indices(&self) -> &[[u32; 3]] {
        &self.triangles
    }

    /// Per-corner provenance, three entries per triangle.
    pub(crate) fn corner_sources(&self) -> &[SourceCorner] {
        &self.corner_src
    }

    /// Per-corner UVs, three entries per triangle, if any were supplied.
    pub(crate) fn corner_uvs(&self) -> Option<&[Option<DVec2>]> {
        self.corner_uv.as_deref()
    }

    /// This soup with each vertex index `i` replaced by `rep[i]`.
    // INVARIANT: `rep` has one in-range entry per vertex (the weld's representatives).
    pub(crate) fn reindexed(&self, rep: &[u32]) -> Self {
        Self {
            positions: self.positions.clone(),
            triangles: self
                .triangles
                .iter()
                .map(|t| t.map(|v| rep[v as usize]))
                .collect(),
            corner_src: self.corner_src.clone(),
            corner_uv: self.corner_uv.clone(),
        }
    }

    /// This soup with only `faces`, in that order, each with its corner records.
    // INVARIANT: every entry of `faces` is a face of this soup.
    pub(crate) fn retaining(&self, faces: &[u32]) -> Self {
        let slots = || {
            faces
                .iter()
                .flat_map(|&f| (0..3).map(move |c| f as usize * 3 + c))
        };
        Self {
            positions: self.positions.clone(),
            triangles: faces.iter().map(|&f| self.triangles[f as usize]).collect(),
            corner_src: slots().map(|i| self.corner_src[i]).collect(),
            corner_uv: self
                .corner_uv
                .as_ref()
                .map(|uv| slots().map(|i| uv[i]).collect()),
        }
    }

    /// The bounding-box diagonal of the vertices triangles use: the scale every
    /// model-space tolerance derives from (`openplush_geom::epsilon`). Unused
    /// vertices are left out, so one stray point cannot inflate the weld and area
    /// tolerances. `None` if no triangle exists. The result can be 0 (all corners
    /// at one point) or overflow to +inf; `ModelTolerances::for_diagonal` refuses
    /// both.
    pub fn bbox_diagonal(&self) -> Option<f64> {
        let mut used = self
            .triangles
            .iter()
            .flatten()
            .filter_map(|&v| self.positions.get(v as usize));
        let first = *used.next()?;
        let (lo, hi) = used.fold((first, first), |(lo, hi), p| {
            (
                DVec3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z)),
                DVec3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z)),
            )
        });
        Some((hi - lo).length())
    }
}

/// Index of a corner in the per-corner arrays. Checked, because on wasm32 `usize`
/// is 32 bits and `face * 3` can overflow for a large (out-of-range) face index.
fn corner_slot(f: FaceId, corner: usize) -> Option<usize> {
    if corner >= 3 {
        return None;
    }
    usize::try_from(f.index())
        .ok()?
        .checked_mul(3)?
        .checked_add(corner)
}
