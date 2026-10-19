//! Small generated meshes for the seams tests: a grid of quads laid flat, rolled into a tube, or
//! closed into a torus, with seams placed by grid coordinates; and an octahedron.

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
    /// The last column joins the first, and the last row the first.
    Torus,
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
            Shape::Torus => (self.cols, self.rows),
        }
    }

    fn point(self, c: u32, r: u32) -> DVec3 {
        let (u, v) = (f64::from(c), f64::from(r));
        let around = TAU * u / f64::from(self.cols);
        match self.shape {
            Shape::Sheet => DVec3::new(10.0 * u, 10.0 * v, 0.0),
            Shape::Tube => DVec3::new(10.0 * around.cos(), 10.0 * around.sin(), 10.0 * v),
            Shape::Torus => {
                let tube = TAU * v / f64::from(self.rows);
                let ring = 20.0 + 8.0 * tube.cos();
                DVec3::new(ring * around.cos(), ring * around.sin(), 8.0 * tube.sin())
            }
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
        let id = |c: u32, r: u32| (r % down) * across + c % across;
        let triangles: Vec<[u32; 3]> = (0..self.rows)
            .flat_map(|r| (0..self.cols).map(move |c| (c, r)))
            .flat_map(|(c, r)| {
                let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
                [[a, b, d], [a, d, e]]
            })
            .collect();
        repaired(positions, triangles)
    }
}

fn repaired(positions: Vec<DVec3>, triangles: Vec<[u32; 3]>) -> Result<ManifoldMesh, Setup> {
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

/// The closed octahedron on the unit points of the axes: +x, +y, -x, -y round the equator, then
/// +z and -z.
pub(crate) struct Octahedron {
    pub(crate) mesh: ManifoldMesh,
}

impl Octahedron {
    const POINTS: [(f64, f64, f64); 6] = [
        (10.0, 0.0, 0.0),
        (0.0, 10.0, 0.0),
        (-10.0, 0.0, 0.0),
        (0.0, -10.0, 0.0),
        (0.0, 0.0, 10.0),
        (0.0, 0.0, -10.0),
    ];

    pub(crate) fn new() -> Result<Self, Setup> {
        let positions = Self::POINTS
            .iter()
            .map(|&(x, y, z)| DVec3::new(x, y, z))
            .collect();
        let triangles = (0..4)
            .flat_map(|i| [[i, (i + 1) % 4, 4], [(i + 1) % 4, i, 5]])
            .collect();
        Ok(Self {
            mesh: repaired(positions, triangles)?,
        })
    }

    /// The edge between points `a` and `b` of `POINTS`.
    pub(crate) fn edge(&self, a: usize, b: usize) -> Result<EdgeId, Setup> {
        let find = |i: usize| {
            let (x, y, z) = Self::POINTS[i];
            (0..u32::try_from(self.mesh.vertex_count())?)
                .map(VertexId::new)
                .find(|&v| self.mesh.position(v) == DVec3::new(x, y, z))
                .ok_or_else(|| Setup::from(format!("no point {i}")))
        };
        let (from, to) = (find(a)?, find(b)?);
        self.mesh
            .outgoing(from)
            .find(|&h| self.mesh.dest(h) == to)
            .map(|h| h.edge())
            .ok_or_else(|| format!("no edge {a} to {b}").into())
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

    pub(crate) fn torus(cols: u32, rows: u32) -> Result<Self, Setup> {
        Self::build(cols, rows, Shape::Torus)
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
            Shape::Torus => (c % across, r % down),
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
