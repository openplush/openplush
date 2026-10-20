//! Joining the copies of a split edge into one bundle: the chains of every seam copy repair
//! records in a group sew together, with k sides between them, each side's half-edges running with
//! the lead chain's walk (checked by position, since copies keep their vertices' positions).

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use common::{Setup, repaired_with_report};
use openplush_geom::DVec3;
use openplush_mesh::{EdgeId, ManifoldMesh, RepairReport};
use openplush_seams::{SeamBundle, SeamChainId, SeamGraph, SeamKind, SeamSet, SeamsError, SewingMap};

fn p(x: f64, y: f64, z: f64) -> DVec3 {
    DVec3::new(x, y, z)
}

/// Outward-wound tetrahedron with a fin on its edge 0-1.
fn fin() -> Result<(ManifoldMesh, RepairReport), Setup> {
    repaired_with_report(
        vec![
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(0., 1., 0.),
            p(0., 0., 1.),
            p(0.5, -1., 0.),
        ],
        vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2], [0, 1, 4]],
    )
}

/// Five triangles round a ring whose last one joins back twisted.
fn twisted_ring() -> Result<(ManifoldMesh, RepairReport), Setup> {
    let radii = [3.0, 1.0, 2.0, 1.0, 1.0];
    let positions = radii
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let a = std::f64::consts::TAU * i as f64 / 5.0;
            p(r * a.cos(), r * a.sin(), 0.0)
        })
        .collect();
    repaired_with_report(
        positions,
        vec![[0, 1, 2], [1, 2, 3], [2, 3, 4], [3, 4, 0], [4, 0, 1]],
    )
}

/// A unit square, both sides: the diagonal 0-2 has four faces.
fn sheet() -> Result<(ManifoldMesh, RepairReport), Setup> {
    repaired_with_report(
        vec![p(0., 0., 0.), p(1., 0., 0.), p(1., 1., 0.), p(0., 1., 0.)],
        vec![[0, 1, 2], [0, 2, 3], [0, 3, 2], [2, 1, 0]],
    )
}

/// Every edge of every copy group marked `kind`.
fn mark_groups(report: &RepairReport, seams: &mut SeamSet, kind: SeamKind) -> Result<(), Setup> {
    for group in report.edge_copies() {
        for &e in group {
            seams.insert(e, kind)?;
        }
    }
    Ok(())
}

fn map(mesh: &ManifoldMesh, seams: &SeamSet, report: &RepairReport) -> Result<Result<SewingMap, SeamsError>, Setup> {
    let graph = SeamGraph::new(mesh, seams)?;
    Ok(SewingMap::new(mesh, seams, &graph, report.edge_copies()))
}

fn is_boundary(mesh: &ManifoldMesh, e: EdgeId) -> bool {
    let h = e.half_edge();
    mesh.face(h).is_none() || mesh.face(h.twin()).is_none()
}

/// Every side of a joined bundle runs with the lead chain's walk: at each position, the origins
/// of all sides' half-edges sit at one point, and so do the destinations.
fn sides_run_together(mesh: &ManifoldMesh, bundle: &SeamBundle) {
    let lead = &bundle.sides()[0];
    for side in bundle.sides() {
        assert_eq!(side.half_edges().len(), lead.half_edges().len(), "{bundle:?}");
        for (&h, &g) in side.half_edges().iter().zip(lead.half_edges()) {
            let at = |v| mesh.position(v);
            assert_eq!(at(mesh.origin(h)), at(mesh.origin(g)), "{bundle:?}");
            assert_eq!(at(mesh.dest(h)), at(mesh.dest(g)), "{bundle:?}");
            assert!(mesh.face(h).is_some(), "{bundle:?}: a side has a face");
        }
    }
}

#[test]
fn a_fin_sews_into_the_body_seam_it_was_split_from_with_three_sides() -> Result<(), Setup> {
    let (mesh, report) = fin()?;
    let mut seams = SeamSet::new();
    mark_groups(&report, &mut seams, SeamKind::Mandatory)?;
    let map = map(&mesh, &seams, &report)??;
    let [bundle] = map.bundles() else {
        panic!("one bundle: {:?}", map.bundles());
    };
    assert_eq!(bundle.sides().len(), 3, "{bundle:?}");
    assert_eq!(bundle.chains().len(), 2);
    assert_eq!(bundle.chain(), bundle.chains()[0]);
    sides_run_together(&mesh, bundle);
    // Both chains find the one bundle.
    for &chain in bundle.chains() {
        assert_eq!(map.bundle(chain), Some(bundle));
    }
    Ok(())
}

#[test]
fn a_fin_whose_body_edge_is_not_a_seam_is_still_an_unpaired_boundary() -> Result<(), Setup> {
    let (mesh, report) = fin()?;
    let mut seams = SeamSet::new();
    let [group] = report.edge_copies() else {
        panic!("{:?}", report.edge_copies());
    };
    let rim = *group.iter().find(|&&e| is_boundary(&mesh, e)).ok_or("a boundary copy")?;
    seams.insert(rim, SeamKind::Mandatory)?;
    let graph = SeamGraph::new(&mesh, &seams)?;
    let chain = graph.chains()[0].id();
    assert_eq!(
        map(&mesh, &seams, &report)?,
        Err(SeamsError::UnpairedBoundarySeam { chain })
    );
    Ok(())
}

#[test]
fn a_twisted_rings_two_cut_copies_sew_back_together() -> Result<(), Setup> {
    let (mesh, report) = twisted_ring()?;
    let mut seams = SeamSet::new();
    mark_groups(&report, &mut seams, SeamKind::Mandatory)?;
    let map = map(&mesh, &seams, &report)??;
    let [bundle] = map.bundles() else {
        panic!("one bundle: {:?}", map.bundles());
    };
    assert_eq!(bundle.sides().len(), 2, "{bundle:?}");
    assert_eq!(bundle.chains().len(), 2);
    sides_run_together(&mesh, bundle);
    Ok(())
}

#[test]
fn a_sheets_diagonal_copies_make_a_four_sided_bundle() -> Result<(), Setup> {
    let (mesh, report) = sheet()?;
    let mut seams = SeamSet::new();
    mark_groups(&report, &mut seams, SeamKind::Cut)?;
    let map = map(&mesh, &seams, &report)??;
    let [bundle] = map.bundles() else {
        panic!("one bundle: {:?}", map.bundles());
    };
    assert_eq!(bundle.sides().len(), 4, "{bundle:?}");
    sides_run_together(&mesh, bundle);
    Ok(())
}

#[test]
fn copies_whose_chains_differ_in_length_are_a_mismatch_not_a_guess() -> Result<(), Setup> {
    let (mesh, report) = fin()?;
    let mut seams = SeamSet::new();
    mark_groups(&report, &mut seams, SeamKind::Mandatory)?;
    // Extend the fin's boundary chain by one of the fin's other rim edges, so it is two edges
    // long while the body's copy is one.
    let [group] = report.edge_copies() else {
        panic!("{:?}", report.edge_copies());
    };
    let rim = *group.iter().find(|&&e| is_boundary(&mesh, e)).ok_or("a boundary copy")?;
    let h = rim.half_edge();
    let b = if mesh.face(h).is_none() { h } else { h.twin() };
    let next = mesh.next(b).edge();
    seams.insert(next, SeamKind::Mandatory)?;
    let graph = SeamGraph::new(&mesh, &seams)?;
    let body = graph
        .chains()
        .iter()
        .find(|c| c.edges().len() == 1 && !is_boundary(&mesh, c.edges()[0]))
        .ok_or("the body chain")?
        .id();
    assert_eq!(
        map(&mesh, &seams, &report)?,
        Err(SeamsError::CopyGroupMismatch { chain: body })
    );
    Ok(())
}

#[test]
fn chain_ids_are_the_lowest_edge_whatever_the_join() -> Result<(), Setup> {
    let (mesh, report) = fin()?;
    let mut seams = SeamSet::new();
    mark_groups(&report, &mut seams, SeamKind::Mandatory)?;
    let map = map(&mesh, &seams, &report)??;
    let lead: SeamChainId = map.bundles()[0].chain();
    assert!(map.bundles()[0].chains().iter().all(|&c| c >= lead));
    Ok(())
}
