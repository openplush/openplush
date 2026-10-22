//! Meshes and seams for the session tests: grids of 10-unit quads written as OBJ and repaired,
//! as a shell would, and seams named by grid point. The session's unit tests include it too, so
//! it names the crates below the pipeline rather than the pipeline itself.

use std::fmt::Write as _;

use openplush_io::obj;
use openplush_mesh::{EdgeId, ManifoldMesh, RepairPlan, VertexId, into_manifold};
use openplush_seams::{SeamKind, SeamSet};

pub(crate) type Setup = Box<dyn std::error::Error>;

pub(crate) fn mesh_of(obj: &str) -> Result<ManifoldMesh, Setup> {
    let soup = obj::read(obj.as_bytes())?;
    Ok(into_manifold(&soup, &RepairPlan::default())?.0)
}

/// The OBJ vertex lines and face lines of a `cols` by `rows` grid, each quad split along its
/// rising diagonal, grid point `(c, r)` at `(10c, 10r, height(c, r))`, faces numbered from
/// vertex `first`.
pub(crate) fn grid_lines(
    cols: u32,
    rows: u32,
    first: u32,
    height: impl Fn(u32, u32) -> f64,
) -> (String, String) {
    let (mut v, mut f) = (String::new(), String::new());
    for r in 0..=rows {
        for c in 0..=cols {
            let _ = writeln!(v, "v {} {} {}", 10 * c, 10 * r, height(c, r));
        }
    }
    let id = |c: u32, r: u32| first + r * (cols + 1) + c;
    for r in 0..rows {
        for c in 0..cols {
            let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
            let _ = writeln!(f, "f {a} {b} {d}\nf {a} {d} {e}");
        }
    }
    (v, f)
}

/// A grid whose faces are numbered row by row from the bottom, quad `(c, r)` holding faces
/// `2(r·cols + c)` and the one after.
pub(crate) fn grid(cols: u32, rows: u32, height: impl Fn(u32, u32) -> f64) -> String {
    let (v, f) = grid_lines(cols, rows, 1, height);
    v + &f
}

/// A `cols` by `rows` grid raised `rise` at every interior point, so it takes several steps to
/// lay flat.
pub(crate) fn bumpy(cols: u32, rows: u32, rise: f64) -> String {
    grid(cols, rows, bumps(cols, rows, rise))
}

/// Heights of `rise` and twice it, alternating over the interior points, and 0 on the border.
pub(crate) fn bumps(cols: u32, rows: u32, rise: f64) -> impl Fn(u32, u32) -> f64 {
    move |c, r| {
        if c == 0 || r == 0 || c == cols || r == rows {
            0.0
        } else {
            rise * f64::from(1 + (c + r) % 2)
        }
    }
}

/// The mesh vertex over grid point `(c, r)`.
pub(crate) fn vertex_at(mesh: &ManifoldMesh, c: u32, r: u32) -> Result<VertexId, Setup> {
    let (x, y) = (f64::from(10 * c), f64::from(10 * r));
    (0..u32::try_from(mesh.vertex_count())?)
        .map(VertexId::new)
        .find(|&v| (mesh.position(v).x, mesh.position(v).y) == (x, y))
        .ok_or_else(|| format!("no vertex at ({c}, {r})").into())
}

pub(crate) fn edge_between(mesh: &ManifoldMesh, a: VertexId, b: VertexId) -> Result<EdgeId, Setup> {
    mesh.outgoing(a)
        .find(|&h| mesh.dest(h) == b)
        .map(|h| h.edge())
        .ok_or_else(|| "no such edge".into())
}

/// Adds seams of `kind` along a walk through the grid points `(column, row)`.
pub(crate) fn seam(
    mesh: &ManifoldMesh,
    seams: &mut SeamSet,
    kind: SeamKind,
    points: &[(u32, u32)],
) -> Result<(), Setup> {
    for pair in points.windows(2) {
        let (a, b) = (
            vertex_at(mesh, pair[0].0, pair[0].1)?,
            vertex_at(mesh, pair[1].0, pair[1].1)?,
        );
        seams.insert(edge_between(mesh, a, b)?, kind)?;
    }
    Ok(())
}

/// A cut from the bottom edge to the top of a grid `rows` high, up column `c`.
pub(crate) fn column_cut(
    mesh: &ManifoldMesh,
    seams: &mut SeamSet,
    c: u32,
    rows: u32,
) -> Result<(), Setup> {
    let points: Vec<(u32, u32)> = (0..=rows).map(|r| (c, r)).collect();
    seam(mesh, seams, SeamKind::Cut, &points)
}

/// A tetrahedron, then a bumpy 3 by 3 grid: chart 0 is the tetrahedron, chart 4 the grid.
pub(crate) fn tetrahedron_then_grid() -> Result<ManifoldMesh, Setup> {
    let tetra_v = "v 100 0 0\nv 110 0 0\nv 100 10 0\nv 100 0 10\n";
    let tetra_f = "f 1 3 2\nf 1 2 4\nf 1 4 3\nf 2 3 4\n";
    let (grid_v, grid_f) = grid_lines(3, 3, 5, bumps(3, 3, 3.0));
    mesh_of(&format!("{tetra_v}{grid_v}{tetra_f}{grid_f}"))
}

/// Seams along the tetrahedron's edges between the points named by their `x, y, z`.
pub(crate) fn tetra_cut(mesh: &ManifoldMesh, path: &[[f64; 3]]) -> Result<SeamSet, Setup> {
    let at = |p: [f64; 3]| {
        (0..u32::try_from(mesh.vertex_count())?)
            .map(VertexId::new)
            .find(|&v| {
                let q = mesh.position(v);
                [q.x, q.y, q.z] == p
            })
            .ok_or_else(|| Setup::from(format!("no vertex at {p:?}")))
    };
    let mut seams = SeamSet::new();
    for pair in path.windows(2) {
        let edge = edge_between(mesh, at(pair[0])?, at(pair[1])?)?;
        seams.insert(edge, SeamKind::Cut)?;
    }
    Ok(seams)
}
