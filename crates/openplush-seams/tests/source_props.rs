//! UV cuts over random corner UVs and random existing seams on a tube, checked against cuts
//! recomputed from the triangles by vertex index, and for the proposal's shape and determinism.
//!
//! Native only: proptest does not build for wasm32 (see Cargo.toml).
#![cfg(not(target_arch = "wasm32"))]

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::TAU;

use common::{Setup, repaired_with_uvs};
use openplush_geom::epsilon::UV_CUT;
use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{EdgeId, ManifoldMesh, VertexId};
use openplush_seams::{ProposalKind, SeamKind, SeamReason, SeamSet, uv_cuts};
use proptest::prelude::*;

const COLS: u32 = 4;
const ROWS: u32 = 2;
const CORNERS: usize = (6 * COLS * ROWS) as usize;

/// A tube `COLS` quads round and `ROWS` along, open at both ends, with welded vertices.
fn tube() -> (Vec<DVec3>, Vec<[u32; 3]>) {
    let positions = (0..=ROWS)
        .flat_map(|r| (0..COLS).map(move |c| (c, r)))
        .map(|(c, r)| {
            let around = TAU * f64::from(c) / f64::from(COLS);
            DVec3::new(
                10.0 * around.cos(),
                10.0 * around.sin(),
                10.0 * f64::from(r),
            )
        })
        .collect();
    let id = |c: u32, r: u32| r * COLS + c % COLS;
    let triangles = (0..ROWS)
        .flat_map(|r| (0..COLS).map(move |c| (c, r)))
        .flat_map(|(c, r)| {
            let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
            [[a, b, d], [a, d, e]]
        })
        .collect();
    (positions, triangles)
}

/// A corner's UV: its vertex's shared UV, a UV from a coarse grid, one within `UV_CUT` of the
/// shared one, or none.
#[derive(Debug, Clone, Copy)]
enum Corner {
    Shared,
    Own(u8, u8),
    Near(i8, i8),
    Bare,
}

fn corner() -> impl Strategy<Value = Corner> {
    prop_oneof![
        4 => Just(Corner::Shared),
        2 => (0u8..3, 0u8..3).prop_map(|(u, v)| Corner::Own(u, v)),
        1 => (-1i8..=1, -1i8..=1).prop_map(|(u, v)| Corner::Near(u, v)),
        1 => Just(Corner::Bare),
    ]
}

/// Every corner's UV, in triangle order then corner order.
fn corner_uvs(triangles: &[[u32; 3]], corners: &[Corner]) -> Vec<Option<DVec2>> {
    let shared = |v: u32| DVec2::new(f64::from(v % COLS) * 0.25, f64::from(v / COLS) * 0.25);
    triangles
        .iter()
        .flatten()
        .zip(corners)
        .map(|(&v, corner)| match *corner {
            Corner::Shared => Some(shared(v)),
            Corner::Own(u, w) => Some(DVec2::new(f64::from(u) * 0.5, f64::from(w) * 0.5)),
            Corner::Near(u, w) => {
                Some(shared(v) + DVec2::new(f64::from(u), f64::from(w)) * (0.5 * UV_CUT))
            }
            Corner::Bare => None,
        })
        .collect()
}

/// The cut edges by vertex index, recomputed from the triangles alone: an edge with two faces,
/// both with UVs at every corner, whose UVs differ beyond `UV_CUT` in a coordinate at an end.
fn oracle(triangles: &[[u32; 3]], uvs: &[Option<DVec2>]) -> (BTreeSet<(u32, u32)>, u32) {
    let mut sides: BTreeMap<(u32, u32), Vec<BTreeMap<u32, DVec2>>> = BTreeMap::new();
    let mut bare = 0;
    for (tri, corner_uvs) in triangles.iter().zip(uvs.chunks(3)) {
        let Some(at): Option<BTreeMap<u32, DVec2>> = tri
            .iter()
            .zip(corner_uvs)
            .map(|(&v, uv)| uv.map(|uv| (v, uv)))
            .collect()
        else {
            bare += 1;
            continue;
        };
        for i in 0..3 {
            let (a, b) = (tri[i], tri[(i + 1) % 3]);
            sides
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push(at.clone());
        }
    }
    let differ = |p: DVec2, q: DVec2| (p.x - q.x).abs() > UV_CUT || (p.y - q.y).abs() > UV_CUT;
    let cuts = sides
        .into_iter()
        .filter(|(edge, both)| {
            both.len() == 2
                && [edge.0, edge.1]
                    .iter()
                    .any(|v| differ(both[0][v], both[1][v]))
        })
        .map(|(edge, _)| edge)
        .collect();
    (cuts, bare)
}

/// The mesh edge between the tube's vertices `a` and `b`, found by position.
fn mesh_edge(mesh: &ManifoldMesh, positions: &[DVec3], a: u32, b: u32) -> Result<EdgeId, Setup> {
    let at = |i: u32| {
        (0..u32::try_from(mesh.vertex_count())?)
            .map(VertexId::new)
            .find(|&v| mesh.position(v) == positions[i as usize])
            .ok_or_else(|| Setup::from(format!("no vertex {i}")))
    };
    let (from, to) = (at(a)?, at(b)?);
    mesh.outgoing(from)
        .find(|&h| mesh.dest(h) == to)
        .map(|h| h.edge())
        .ok_or_else(|| format!("no edge {a} to {b}").into())
}

const KINDS: [SeamKind; 4] = [
    SeamKind::Cut,
    SeamKind::Dart,
    SeamKind::Opening,
    SeamKind::FabricBoundary,
];

fn check(corners: &[Corner], existing: &[u8]) -> Result<(), Setup> {
    let (positions, triangles) = tube();
    let uvs = corner_uvs(&triangles, corners);
    let (mesh, _) = repaired_with_uvs(positions.clone(), triangles.clone(), Some(uvs.clone()))?;
    let mut seams = SeamSet::new();
    for (e, &k) in existing.iter().enumerate().take(mesh.edge_count()) {
        if let Some(&kind) = KINDS.get(usize::from(k)) {
            seams.insert(EdgeId::new(u32::try_from(e)?), kind)?;
        }
    }

    let cuts = uv_cuts(&mesh, &seams)?;
    let (expected, bare) = oracle(&triangles, &uvs);
    let mut want = Vec::new();
    for (a, b) in expected {
        let edge = mesh_edge(&mesh, &positions, a, b)?;
        if seams.get(edge).is_none() {
            want.push(edge);
        }
    }
    want.sort();
    assert_eq!(cuts.edges(), want.as_slice());
    assert_eq!(cuts.faces_without_uv(), bare);
    assert!(cuts.edges().is_sorted_by(|a, b| a < b));
    assert!(cuts.edges().iter().all(|&e| seams.get(e).is_none()));

    let proposal = cuts.proposal()?;
    assert_eq!(proposal.is_some(), !cuts.edges().is_empty());
    if let Some(p) = &proposal {
        let kind = ProposalKind::Seam {
            reason: SeamReason::SourceModel,
        };
        assert_eq!(p.kind(), &kind);
        assert!(p.add().keys().eq(cuts.edges()));
        assert!(p.add().values().all(|&k| k == SeamKind::Cut));
    }

    let again = uv_cuts(&mesh, &seams)?;
    assert_eq!(again, cuts);
    assert_eq!(again.proposal()?, proposal);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, rng_seed: proptest::test_runner::RngSeed::Fixed(20261021), ..ProptestConfig::default() })]

    #[test]
    fn uv_cuts_match_the_oracle_skip_existing_seams_and_repeat(
        corners in prop::collection::vec(corner(), CORNERS),
        existing in prop::collection::vec(0u8..12, 64),
    ) {
        check(&corners, &existing).map_err(|e| TestCaseError::fail(e.to_string()))?;
    }
}
