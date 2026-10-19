//! The shortest surface path between two picked vertices, for seam drawing: an edge path whose
//! 3D length an independent all-pairs oracle confirms is the shortest, walked from one pick to
//! the other, the same every time.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use common::{Grid, Setup, repaired};
use openplush_geom::DVec3;
use openplush_mesh::{EdgeId, ManifoldMesh, VertexId};
use openplush_seams::{SeamsError, shortest_path};

fn length(mesh: &ManifoldMesh, e: EdgeId) -> f64 {
    let h = e.half_edge();
    (mesh.position(mesh.dest(h)) - mesh.position(mesh.origin(h))).length()
}

/// Oracle: Floyd–Warshall over every vertex from positions alone.
fn all_pairs(mesh: &ManifoldMesh) -> Result<Vec<Vec<f64>>, Setup> {
    let n = mesh.vertex_count();
    let mut d = vec![vec![f64::INFINITY; n]; n];
    for (i, row) in d.iter_mut().enumerate() {
        row[i] = 0.0;
    }
    for e in 0..u32::try_from(mesh.edge_count())? {
        let h = EdgeId::new(e).half_edge();
        let (a, b) = (
            mesh.origin(h).index() as usize,
            mesh.dest(h).index() as usize,
        );
        let l = length(mesh, EdgeId::new(e));
        d[a][b] = d[a][b].min(l);
        d[b][a] = d[b][a].min(l);
    }
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                let via = d[i][k] + d[k][j];
                if via < d[i][j] {
                    d[i][j] = via;
                }
            }
        }
    }
    Ok(d)
}

/// A path's edges join end to end from `from` to `to`; returns its length.
fn walked(mesh: &ManifoldMesh, path: &[EdgeId], from: VertexId, to: VertexId) -> f64 {
    let mut at = from;
    let mut total = 0.0;
    for &e in path {
        let h = e.half_edge();
        let (a, b) = (mesh.origin(h), mesh.dest(h));
        assert!(a == at || b == at, "{e:?} does not leave {at:?}");
        at = if a == at { b } else { a };
        total += length(mesh, e);
    }
    assert_eq!(at, to, "the path ends at the second pick");
    total
}

#[test]
fn every_pair_on_a_sheet_gets_a_shortest_walk_from_pick_to_pick() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mesh = &grid.mesh;
    let oracle = all_pairs(mesh)?;
    let n = u32::try_from(mesh.vertex_count())?;
    for a in (0..n).map(VertexId::new) {
        for b in (0..n).map(VertexId::new) {
            let path = shortest_path(mesh, a, b)?;
            let total = walked(mesh, &path, a, b);
            let want = oracle[a.index() as usize][b.index() as usize];
            assert!((total - want).abs() < 1e-9, "{a:?} to {b:?}: {total} vs {want}");
        }
    }
    Ok(())
}

#[test]
fn a_straight_run_along_a_row_is_its_edges_in_order() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let (from, to) = (grid.at(0, 1)?, grid.at(4, 1)?);
    let want: Vec<EdgeId> = (0..4)
        .map(|c| grid.edge((c, 1), (c + 1, 1)))
        .collect::<Result<_, _>>()?;
    assert_eq!(shortest_path(&grid.mesh, from, to)?, want);
    Ok(())
}

#[test]
fn the_same_picks_give_the_same_path_among_equal_choices() -> Result<(), Setup> {
    // From a corner to a point two across and one up there are several equal-length walks.
    let grid = Grid::sheet(4, 2)?;
    let (from, to) = (grid.at(0, 0)?, grid.at(2, 1)?);
    let first = shortest_path(&grid.mesh, from, to)?;
    for _ in 0..3 {
        assert_eq!(shortest_path(&grid.mesh, from, to)?, first);
    }
    assert_eq!(first.len(), 2);
    Ok(())
}

#[test]
fn the_same_pick_twice_is_an_empty_path() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let v = grid.at(1, 1)?;
    assert_eq!(shortest_path(&grid.mesh, v, v)?, []);
    Ok(())
}

#[test]
fn a_pick_the_mesh_does_not_have_is_refused() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let beyond = VertexId::new(u32::try_from(grid.mesh.vertex_count())?);
    let v = grid.at(0, 0)?;
    assert_eq!(
        shortest_path(&grid.mesh, v, beyond),
        Err(SeamsError::UnknownVertex { vertex: beyond })
    );
    assert_eq!(
        shortest_path(&grid.mesh, beyond, v),
        Err(SeamsError::UnknownVertex { vertex: beyond })
    );
    Ok(())
}

#[test]
fn picks_on_separate_pieces_of_mesh_have_no_path() -> Result<(), Setup> {
    let mesh = repaired(
        vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(10.0, 0.0, 0.0),
            DVec3::new(0.0, 10.0, 0.0),
            DVec3::new(50.0, 0.0, 0.0),
            DVec3::new(60.0, 0.0, 0.0),
            DVec3::new(50.0, 10.0, 0.0),
        ],
        vec![[0, 1, 2], [3, 4, 5]],
    )?;
    let at = |p: DVec3| -> Result<VertexId, Setup> {
        (0..u32::try_from(mesh.vertex_count())?)
            .map(VertexId::new)
            .find(|&v| mesh.position(v) == p)
            .ok_or_else(|| "no vertex there".into())
    };
    let (from, to) = (at(DVec3::new(0.0, 0.0, 0.0))?, at(DVec3::new(60.0, 0.0, 0.0))?);
    assert_eq!(
        shortest_path(&mesh, from, to),
        Err(SeamsError::NoPath { from, to })
    );
    Ok(())
}
