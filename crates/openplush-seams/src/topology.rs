//! The disk check: whether a chart, cut open along its seams, is a topological disk.
//!
//! The chart is never built cut open. A half-edge of the chart is *cut* when a seam runs along it
//! or the face across it is not in the chart; cutting duplicates each vertex once per wedge of
//! chart faces between cuts, and each cut half-edge ends at exactly one such wedge. So with `F`
//! faces, `B` cut half-edges and `I` vertices that no cut half-edge ends at, the cut-open chart
//! has `I + B` vertices and `(3F + B) / 2` edges, and χ = V − E + F = I + (B − F) / 2.

use std::collections::BTreeSet;

use openplush_mesh::{HalfEdgeId, ManifoldMesh};

use crate::{Chart, ChartFailure, SeamSet};

/// `Ok` when `chart`, from `charts(mesh, seams)`, cut open along `seams` is a disk: χ = 2 − 2g − b
/// with genus g = 0 and b = 1 boundary loop.
///
/// # Errors
/// `PositiveGenus` when it has handles, whatever its boundaries, since `Closed` and
/// `MultipleBoundaries` describe genus-0 regions; otherwise `Closed` with no boundary, or
/// `MultipleBoundaries` with more than one.
pub fn check_disk(mesh: &ManifoldMesh, seams: &SeamSet, chart: &Chart) -> Result<(), ChartFailure> {
    let mut inside = vec![false; mesh.face_count()];
    for f in chart.faces() {
        inside[f.index() as usize] = true;
    }
    let cut = |h: HalfEdgeId| {
        seams.get(h.edge()).is_some()
            || !mesh
                .face(h.twin())
                .is_some_and(|g| inside[g.index() as usize])
    };
    let sides: Vec<HalfEdgeId> = chart
        .faces()
        .iter()
        .flat_map(|&f| {
            let h = mesh.face_half_edge(f);
            [h, mesh.next(h), mesh.prev(h)]
        })
        .collect();
    let cuts: Vec<HalfEdgeId> = sides.iter().copied().filter(|&h| cut(h)).collect();
    let vertices: BTreeSet<_> = sides.iter().map(|&h| mesh.origin(h)).collect();
    let on_cut: BTreeSet<_> = cuts.iter().map(|&h| mesh.dest(h)).collect();
    let (faces, cut_count) = (count(chart.faces().len()), count(cuts.len()));
    let uncut = count(vertices.len() - on_cut.len());
    // INVARIANT: B ≡ 3F ≡ F (mod 2), since the uncut half-edges pair up.
    let euler = uncut + (cut_count - faces) / 2;
    let boundaries = loops(mesh, &cuts, cut);
    // INVARIANT: a connected surface has χ = 2 − 2g − b with g ≥ 0, and g and b are at most
    // the face count, which fits in u32.
    let genus = (2 - count(boundaries) - euler) / 2;
    if genus > 0 {
        Err(ChartFailure::PositiveGenus {
            genus: genus as u32,
        })
    } else if boundaries == 0 {
        Err(ChartFailure::Closed)
    } else if boundaries > 1 {
        Err(ChartFailure::MultipleBoundaries {
            count: boundaries as u32,
        })
    } else {
        Ok(())
    }
}

/// The boundary loops of the cut-open chart: from each cut half-edge, the next one along the
/// boundary is found by turning about its end through the wedge of chart faces it bounds.
fn loops(mesh: &ManifoldMesh, cuts: &[HalfEdgeId], cut: impl Fn(HalfEdgeId) -> bool) -> usize {
    let mut seen = BTreeSet::new();
    let mut loops = 0;
    for &start in cuts {
        if !seen.insert(start) {
            continue;
        }
        loops += 1;
        let mut h = start;
        loop {
            let mut n = mesh.next(h);
            // INVARIANT: an uncut half-edge has a chart face across it, so the turn stays in
            // the chart and reaches a cut within one fan.
            while !cut(n) {
                n = mesh.next(n.twin());
            }
            if n == start {
                break;
            }
            seen.insert(n);
            h = n;
        }
    }
    loops
}

/// A count as a signed number for the Euler arithmetic; mesh counts fit in u32.
fn count(n: usize) -> i64 {
    n as i64
}
