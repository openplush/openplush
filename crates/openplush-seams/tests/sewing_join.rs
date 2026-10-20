//! Joining the copies of a split edge into one bundle: the chains of every seam copy repair
//! records in a group sew together, with k sides between them, each side's half-edges running with
//! the lead chain's walk (checked by position, since copies keep their vertices' positions).

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use common::{Grid, Setup, repaired_with_report};
use openplush_geom::DVec3;
use openplush_mesh::{EdgeId, HalfEdgeId, ManifoldMesh, RepairReport};
use openplush_seams::{
    SeamBundle, SeamChainId, SeamGraph, SeamKind, SeamNode, SeamSet, SeamsError, SewingMap,
};

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

fn map(
    mesh: &ManifoldMesh,
    seams: &SeamSet,
    report: &RepairReport,
) -> Result<Result<SewingMap, SeamsError>, Setup> {
    let graph = SeamGraph::with_copies(mesh, seams, report.edge_copies())?;
    Ok(SewingMap::new(mesh, seams, &graph, report.edge_copies()))
}

fn is_boundary(mesh: &ManifoldMesh, e: EdgeId) -> bool {
    let h = e.half_edge();
    mesh.face(h).is_none() || mesh.face(h.twin()).is_none()
}

/// Every side of a joined bundle follows the lead chain's walk: at each position, every side's
/// half-edge joins the same two points as the lead's (with the walk or, for a twin side, against
/// it), and every side has faces.
fn sides_run_together(mesh: &ManifoldMesh, bundle: &SeamBundle) {
    let lead = &bundle.sides()[0];
    let at = |h: HalfEdgeId| {
        let mut ends = [mesh.position(mesh.origin(h)), mesh.position(mesh.dest(h))];
        ends.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        ends
    };
    for side in bundle.sides() {
        assert_eq!(
            side.half_edges().len(),
            lead.half_edges().len(),
            "{bundle:?}"
        );
        for (&h, &g) in side.half_edges().iter().zip(lead.half_edges()) {
            assert_eq!(at(h), at(g), "{bundle:?}");
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
    let rim = *group
        .iter()
        .find(|&&e| is_boundary(&mesh, e))
        .ok_or("a boundary copy")?;
    seams.insert(rim, SeamKind::Mandatory)?;
    let graph = SeamGraph::with_copies(&mesh, &seams, report.edge_copies())?;
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
    let rim = *group
        .iter()
        .find(|&&e| is_boundary(&mesh, e))
        .ok_or("a boundary copy")?;
    let h = rim.half_edge();
    let b = if mesh.face(h).is_none() { h } else { h.twin() };
    let next = mesh.next(b).edge();
    seams.insert(next, SeamKind::Mandatory)?;
    let graph = SeamGraph::with_copies(&mesh, &seams, report.edge_copies())?;
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

#[test]
fn two_copies_meeting_at_a_shared_vertex_are_separate_chains_ending_at_copy_ends()
-> Result<(), Setup> {
    // The pillow's diagonal copies join the same two vertices; without the groups they would
    // chain into one two-edge ring.
    let (mesh, report) = sheet()?;
    let mut seams = SeamSet::new();
    mark_groups(&report, &mut seams, SeamKind::Cut)?;
    let ring = SeamGraph::new(&mesh, &seams)?;
    assert_eq!(ring.chains().len(), 1);
    let graph = SeamGraph::with_copies(&mesh, &seams, report.edge_copies())?;
    assert_eq!(graph.chains().len(), 2);
    assert!(graph.nodes().all(|(_, n)| n == SeamNode::CopyEnd));
    assert_eq!(graph.nodes().count(), 2);
    Ok(())
}

/// The tetrahedron with a fin along two consecutive body edges, 0-1 and 1-3: each spine edge
/// is a copy group, and the body's copies and the fin's each form a two-edge chain.
fn two_edge_fin() -> Result<(ManifoldMesh, RepairReport), Setup> {
    repaired_with_report(
        vec![
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(0., 1., 0.),
            p(0., 0., 1.),
            p(0.5, -1., 0.5),
        ],
        vec![
            [0, 2, 1],
            [0, 1, 3],
            [1, 2, 3],
            [0, 3, 2],
            [0, 1, 4],
            [1, 3, 4],
        ],
    )
}

/// Six pages on one spine, three coincident copies of it, each pair of pages a manifold edge.
fn triple_book() -> Result<(ManifoldMesh, RepairReport), Setup> {
    let mut positions = vec![p(0., 0., 0.), p(1., 0., 0.)];
    let mut triangles = Vec::new();
    for k in 0..6u32 {
        let t = f64::from(k) * std::f64::consts::FRAC_PI_3;
        positions.push(p(0.5, t.cos(), t.sin()));
        triangles.push(if k % 2 == 0 {
            [0, 1, k + 2]
        } else {
            [1, 0, k + 2]
        });
    }
    repaired_with_report(positions, triangles)
}

fn map_with(
    mesh: &ManifoldMesh,
    seams: &SeamSet,
    groups: &[Vec<EdgeId>],
) -> Result<(Result<SewingMap, SeamsError>, SeamChainId), Setup> {
    let graph = SeamGraph::with_copies(mesh, seams, groups)?;
    let first = graph.chains().first().ok_or("a chain")?.id();
    Ok((SewingMap::new(mesh, seams, &graph, groups), first))
}

#[test]
fn a_two_edge_fin_sews_chain_to_chain_with_three_sides_of_two_edges() -> Result<(), Setup> {
    let (mesh, report) = two_edge_fin()?;
    let mut seams = SeamSet::new();
    mark_groups(&report, &mut seams, SeamKind::Mandatory)?;
    let map = map(&mesh, &seams, &report)??;
    let [bundle] = map.bundles() else {
        panic!("one bundle: {:?}", map.bundles());
    };
    assert_eq!(bundle.chains().len(), 2);
    assert_eq!(bundle.sides().len(), 3, "{bundle:?}");
    assert!(bundle.sides().iter().all(|s| s.half_edges().len() == 2));
    sides_run_together(&mesh, bundle);
    Ok(())
}

#[test]
fn a_group_covering_only_the_start_of_the_chain_is_a_mismatch() -> Result<(), Setup> {
    let (mesh, report) = two_edge_fin()?;
    let mut seams = SeamSet::new();
    mark_groups(&report, &mut seams, SeamKind::Mandatory)?;
    // Only the first spine edge's copies are grouped: the copy chain has two edges, the lead
    // finds a partner for one of them.
    let first_only = vec![report.edge_copies()[0].clone()];
    let (map, lead) = map_with(&mesh, &seams, &first_only)?;
    assert_eq!(map, Err(SeamsError::CopyGroupMismatch { chain: lead }));
    Ok(())
}

#[test]
fn a_copy_that_shares_only_one_end_with_the_lead_edge_is_a_mismatch() -> Result<(), Setup> {
    // Every pair of edges meeting at one interior vertex, grouped as if they were copies. For
    // some pair the lead's walk starts at the shared vertex, where a match on starting
    // positions alone would sew the pair; none is a copy, so each is a mismatch.
    let grid = Grid::sheet(4, 3)?;
    let at = grid.at(2, 1)?;
    let around: Vec<EdgeId> = grid.mesh.outgoing(at).map(|h| h.edge()).collect();
    assert!(around.len() >= 4, "{around:?}");
    for (i, &a) in around.iter().enumerate() {
        for &b in &around[i + 1..] {
            let mut seams = SeamSet::new();
            seams.insert(a, SeamKind::Cut)?;
            seams.insert(b, SeamKind::Cut)?;
            let (map, lead) = map_with(&grid.mesh, &seams, &[vec![a, b]])?;
            assert_eq!(
                map,
                Err(SeamsError::CopyGroupMismatch { chain: lead }),
                "{a:?} {b:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn three_coincident_copies_sew_into_one_bundle_of_six_sides() -> Result<(), Setup> {
    let (mesh, report) = triple_book()?;
    let mut seams = SeamSet::new();
    mark_groups(&report, &mut seams, SeamKind::Cut)?;
    let map = map(&mesh, &seams, &report)??;
    let [bundle] = map.bundles() else {
        panic!("one bundle: {:?}", map.bundles());
    };
    assert_eq!(bundle.chains().len(), 3);
    assert_eq!(bundle.sides().len(), 6, "{bundle:?}");
    sides_run_together(&mesh, bundle);
    Ok(())
}

#[test]
fn overlapping_groups_that_claim_one_chain_twice_are_a_mismatch() -> Result<(), Setup> {
    let (mesh, report) = triple_book()?;
    let mut seams = SeamSet::new();
    mark_groups(&report, &mut seams, SeamKind::Cut)?;
    let [x, y, z] = report.edge_copies()[0][..] else {
        panic!("{:?}", report.edge_copies());
    };
    // `y` is in both groups; the first lead joins it, so the second cannot.
    let (map, _) = map_with(&mesh, &seams, &[vec![x, y], vec![y, z]])?;
    // The refused chain is the later lead, `z`'s, whose copy `y` was already taken.
    let late = SeamChainId::new(z.index());
    assert_eq!(map, Err(SeamsError::CopyGroupMismatch { chain: late }));
    Ok(())
}
