//! A chart cut open along its seams as flat arrays: a mesh vertex is copied once per fan of chart
//! faces between cuts, so a seam through a chart separates its two sides while a dart tip stays
//! one vertex. Each expected copy count is worked out from the grid by hand.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use common::{Grid, Octahedron, ROW_1, Setup};
use openplush_mesh::{FaceId, ManifoldMesh, VertexId};
use openplush_seams::{ChartMesh, SeamKind, SeamSet, charts};

/// Every chart of `seams`, cut open and checked by `well_formed`.
fn cut_open(mesh: &ManifoldMesh, seams: &SeamSet) -> Result<Vec<ChartMesh>, Setup> {
    Ok(charts(mesh, seams)?
        .iter()
        .map(|chart| {
            let cut = ChartMesh::new(mesh, seams, chart);
            assert_eq!(cut.chart(), chart.id());
            let faces: Vec<FaceId> = cut.faces().iter().map(|&(f, _)| f).collect();
            assert_eq!(faces, chart.faces(), "every chart face, in ascending order");
            well_formed(mesh, seams, &cut);
            cut
        })
        .collect())
}

/// Each local corner is its face's global corner, in the face's order; each local vertex sits
/// where its global vertex does; local vertices are numbered in order of first appearance; and
/// two faces across an edge with no seam share that edge's local vertices.
fn well_formed(mesh: &ManifoldMesh, seams: &SeamSet, cut: &ChartMesh) {
    let n = cut.xyz().len();
    assert_eq!(cut.vtx_to_global().len(), n);
    let mut next = 0;
    let mut local_of = std::collections::BTreeMap::new();
    for &(f, corners) in cut.faces() {
        let h = mesh.face_half_edge(f);
        let sides = [h, mesh.next(h), mesh.prev(h)];
        for (&l, &side) in corners.iter().zip(&sides) {
            let i = l.index() as usize;
            assert!(i <= next, "{l:?} numbered before first use");
            next = next.max(i + 1);
            assert_eq!(cut.vtx_to_global()[i], mesh.origin(side));
            assert_eq!(cut.xyz()[i], mesh.position(mesh.origin(side)));
            local_of.insert(side, l);
        }
    }
    assert_eq!(next, n, "every local vertex is used");
    for (&side, &l) in &local_of {
        let across = side.twin();
        if seams.get(side.edge()).is_some() {
            continue;
        }
        // The far face's corner at this side's origin starts the half-edge after `across`.
        if let (Some(_), Some(&far)) = (
            local_of.get(&across),
            local_of.get(&mesh.next(across)),
        ) {
            assert_eq!(l, far, "no seam on {:?}, so one vertex", side.edge());
        }
    }
}

/// How many local vertices, over every chart, copy `v`.
fn copies(cuts: &[ChartMesh], v: VertexId) -> usize {
    cuts.iter()
        .flat_map(|c| c.vtx_to_global())
        .filter(|&&g| g == v)
        .count()
}

fn total(cuts: &[ChartMesh]) -> usize {
    cuts.iter().map(|c| c.xyz().len()).sum()
}

#[test]
fn a_sheet_with_no_seams_keeps_every_vertex_once() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let cuts = cut_open(&grid.mesh, &SeamSet::new())?;
    assert_eq!(total(&cuts), grid.mesh.vertex_count());
    Ok(())
}

#[test]
fn a_seam_across_a_sheet_copies_its_vertices_into_both_charts() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    let cuts = cut_open(&grid.mesh, &seams)?;
    assert_eq!(cuts.len(), 2);
    assert_eq!(total(&cuts), grid.mesh.vertex_count() + ROW_1.len());
    for &(c, r) in &ROW_1 {
        assert_eq!(copies(&cuts, grid.at(c, r)?), 2, "({c}, {r})");
    }
    Ok(())
}

#[test]
fn a_dart_from_the_edge_splits_its_mouth_and_keeps_its_tip() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Dart, &[(2, 0), (2, 1)])?;
    let cuts = cut_open(&grid.mesh, &seams)?;
    assert_eq!(copies(&cuts, grid.at(2, 0)?), 2, "the mouth, on the boundary");
    assert_eq!(copies(&cuts, grid.at(2, 1)?), 1, "the tip");
    assert_eq!(total(&cuts), grid.mesh.vertex_count() + 1);
    Ok(())
}

#[test]
fn an_interior_slit_splits_its_middle_and_keeps_both_ends() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(1, 1), (2, 1), (3, 1)])?;
    let cuts = cut_open(&grid.mesh, &seams)?;
    assert_eq!(copies(&cuts, grid.at(1, 1)?), 1);
    assert_eq!(copies(&cuts, grid.at(2, 1)?), 2);
    assert_eq!(copies(&cuts, grid.at(3, 1)?), 1);
    Ok(())
}

#[test]
fn a_tube_cut_lengthwise_opens_into_a_sheet() -> Result<(), Setup> {
    let grid = Grid::tube(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(0, 0), (0, 1), (0, 2)])?;
    let cuts = cut_open(&grid.mesh, &seams)?;
    assert_eq!(cuts.len(), 1);
    // A 4 by 2 sheet's 5 by 3 vertices.
    assert_eq!(total(&cuts), 15);
    Ok(())
}

#[test]
fn a_seam_on_the_boundary_copies_nothing() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Mandatory, &[(0, 0), (1, 0), (2, 0)])?;
    let cuts = cut_open(&grid.mesh, &seams)?;
    assert_eq!(total(&cuts), grid.mesh.vertex_count());
    Ok(())
}

#[test]
fn a_slit_octahedron_keeps_six_vertices_and_its_equator_cut_copies_four() -> Result<(), Setup> {
    let solid = Octahedron::new()?;
    let mut seams = SeamSet::new();
    seams.insert(solid.edge(0, 4)?, SeamKind::Cut)?;
    assert_eq!(total(&cut_open(&solid.mesh, &seams)?), 6);
    let mut seams = SeamSet::new();
    for i in 0..4 {
        seams.insert(solid.edge(i, (i + 1) % 4)?, SeamKind::Cut)?;
    }
    let cuts = cut_open(&solid.mesh, &seams)?;
    assert_eq!(cuts.iter().map(|c| c.xyz().len()).collect::<Vec<_>>(), [5, 5]);
    Ok(())
}
