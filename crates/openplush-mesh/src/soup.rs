use openplush_geom::{DVec2, DVec3};

use crate::{MeshError, VertexId};

/// Where a triangle corner came from in the source file: which primitive (glTF
/// primitive, OBJ group) and which source vertex index. Provenance survives repair,
/// so a saved project can find the same corner again after re-import (T125).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceCorner {
    /// Source primitive or group index.
    pub primitive: u32,
    /// Vertex index within the source primitive.
    pub vertex: u32,
}

/// Indexed triangles as imported, before any repair.
///
/// **Guarantees** (data-model section 1): every coordinate is finite, every index is
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
    /// Errors: [`MeshError::NonFiniteVertex`], [`MeshError::IndexOutOfRange`],
    /// [`MeshError::CornerCountMismatch`], [`MeshError::TooLarge`].
    pub fn new(
        positions: Vec<DVec3>,
        triangles: Vec<[u32; 3]>,
        corner_src: Vec<SourceCorner>,
        corner_uv: Option<Vec<Option<DVec2>>>,
    ) -> Result<Self, MeshError> {
        let vertex_count = positions.len();
        if u32::try_from(vertex_count).is_err() {
            return Err(MeshError::TooLarge {
                count: vertex_count,
            });
        }
        if u32::try_from(triangles.len()).is_err() {
            return Err(MeshError::TooLarge {
                count: triangles.len(),
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
                    triangle,
                    index,
                    vertex_count,
                });
            }
        }
        let expected = triangles.len() * 3;
        if corner_src.len() != expected {
            return Err(MeshError::CornerCountMismatch {
                expected,
                found: corner_src.len(),
            });
        }
        if let Some(uv) = &corner_uv
            && uv.len() != expected
        {
            return Err(MeshError::CornerCountMismatch {
                expected,
                found: uv.len(),
            });
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

    /// The vertices of triangle `t`, or `None` if it does not exist.
    pub fn triangle(&self, t: usize) -> Option<[VertexId; 3]> {
        self.triangles.get(t).map(|tri| tri.map(VertexId::new))
    }

    /// Provenance of corner `corner` (0..3) of triangle `t`.
    pub fn corner_source(&self, t: usize, corner: usize) -> Option<SourceCorner> {
        if corner >= 3 {
            return None;
        }
        self.corner_src.get(t * 3 + corner).copied()
    }

    /// UV of corner `corner` of triangle `t`; `None` if absent or out of range.
    pub fn corner_uv(&self, t: usize, corner: usize) -> Option<DVec2> {
        if corner >= 3 {
            return None;
        }
        self.corner_uv
            .as_ref()?
            .get(t * 3 + corner)
            .copied()
            .flatten()
    }

    /// The bounding-box diagonal: the scale every model-space tolerance derives from
    /// (`openplush_geom::epsilon`). `None` for an empty soup.
    pub fn bbox_diagonal(&self) -> Option<f64> {
        let first = *self.positions.first()?;
        let (lo, hi) = self.positions.iter().fold((first, first), |(lo, hi), p| {
            (
                DVec3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z)),
                DVec3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z)),
            )
        });
        Some((hi - lo).length())
    }
}
