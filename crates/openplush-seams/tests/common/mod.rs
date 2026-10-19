//! Small generated meshes for the seams tests: a grid of quads laid flat or rolled into a tube,
//! with seams placed by grid coordinates.

use std::f64::consts::TAU;

use openplush_geom::DVec3;
use openplush_mesh::{EdgeId, ManifoldMesh, RepairPlan, SourceCorner, TriangleSoup, VertexId};
use openplush_seams::{SeamKind, SeamSet};

pub(crate) type Setup = Box<dyn std::error::Error>;

/// Across a 4-column sheet's middle row, or once round a 4-column tube.
pub(crate) const ROW_1: [(u32, u32); 5] = [(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Sheet,
    /// The last column joins the first.
    Tube,
}

/// Where a grid's vertices are: `cols` by `rows` quads in some shape.
#[derive(Debug, Clone, Copy)]
struct Layout {
    cols: u32,
    rows: u32,
    shape: Shape,
}

impl Layout {
    /// Distinct vertex columns and rows.
    fn counts(self) -> (u32, u32) {
        match self.shape {
            Shape::Sheet => (self.cols + 1, self.rows + 1),
            Shape::Tube => (self.cols, self.rows + 1),
        }
    }

    fn point(self, c: u32, r: u32) -> DVec3 {
        let (u, v) = (f64::from(c), f64::from(r));
        let around = TAU * u / f64::from(self.cols);
        match self.shape {
            Shape::Sheet => DVec3::new(10.0 * u, 10.0 * v, 0.0),
            Shape::Tube => DVec3::new(10.0 * around.cos(), 10.0 * around.sin(), 10.0 * v),
        }
    }

    /// Two counter-clockwise triangles per quad, split along the diagonal from `(c, r)` to
    /// `(c + 1, r + 1)`.
    fn mesh(self) -> Result<ManifoldMesh, Setup> {
        let (across, down) = self.counts();
        let positions = (0..down)
            .flat_map(|r| (0..across).map(move |c| (c, r)))
            .map(|(c, r)| self.point(c, r))
            .collect();
        let id = |c: u32, r: u32| r * across + c % across;
        let triangles: Vec<[u32; 3]> = (0..self.rows)
            .flat_map(|r| (0..self.cols).map(move |c| (c, r)))
            .flat_map(|(c, r)| {
                let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
                [[a, b, d], [a, d, e]]
            })
            .collect();
        let corners = (0..)
            .take(3 * triangles.len())
            .map(|vertex| SourceCorner {
                primitive: 0,
                vertex,
            })
            .collect();
        let soup = TriangleSoup::new(positions, triangles, corners, None)?;
        Ok(openplush_mesh::into_manifold(&soup, &RepairPlan::default())?.0)
    }
}

/// A repaired grid mesh, with seams placed by `(column, row)`.
pub(crate) struct Grid {
    pub(crate) mesh: ManifoldMesh,
    layout: Layout,
}

impl Grid {
    pub(crate) fn sheet(cols: u32, rows: u32) -> Result<Self, Setup> {
        Self::build(cols, rows, Shape::Sheet)
    }

    pub(crate) fn tube(cols: u32, rows: u32) -> Result<Self, Setup> {
        Self::build(cols, rows, Shape::Tube)
    }

    fn build(cols: u32, rows: u32, shape: Shape) -> Result<Self, Setup> {
        let layout = Layout { cols, rows, shape };
        Ok(Self {
            mesh: layout.mesh()?,
            layout,
        })
    }

    /// The vertex at column `c`, row `r`, found by position so repair's numbering is not assumed.
    pub(crate) fn at(&self, c: u32, r: u32) -> Result<VertexId, Setup> {
        let (across, down) = self.layout.counts();
        let (c, r) = match self.layout.shape {
            Shape::Sheet => (c, r),
            Shape::Tube => (c % across, r),
        };
        if c >= across || r >= down {
            return Err(format!("({c}, {r}) is off the grid").into());
        }
        let want = self.layout.point(c, r);
        (0..u32::try_from(self.mesh.vertex_count())?)
            .map(VertexId::new)
            .find(|&v| self.mesh.position(v) == want)
            .ok_or_else(|| format!("no vertex at ({c}, {r})").into())
    }

    /// The edge from `a` to `b`, each a `(column, row)`.
    pub(crate) fn edge(&self, a: (u32, u32), b: (u32, u32)) -> Result<EdgeId, Setup> {
        let (from, to) = (self.at(a.0, a.1)?, self.at(b.0, b.1)?);
        self.mesh
            .outgoing(from)
            .find(|&h| self.mesh.dest(h) == to)
            .map(|h| h.edge())
            .ok_or_else(|| format!("no edge {a:?} to {b:?}").into())
    }

    /// Seams of `kind` along the path through `points`.
    pub(crate) fn seam(
        &self,
        seams: &mut SeamSet,
        kind: SeamKind,
        points: &[(u32, u32)],
    ) -> Result<(), Setup> {
        for pair in points.windows(2) {
            seams.insert(self.edge(pair[0], pair[1])?, kind)?;
        }
        Ok(())
    }
}
