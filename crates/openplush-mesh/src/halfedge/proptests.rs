//! Property tests for `ManifoldMesh`: every mesh the builder accepts
//! satisfies `debug_validate`, and every mesh it refuses gets a typed error.
//!
//! In-crate rather than `tests/invariants.rs` (as tasks.md names it) because the
//! builder is crate-private: only repair may construct a `ManifoldMesh`.

use openplush_geom::DVec3;
use proptest::prelude::*;

use super::ManifoldMesh;
use super::tests::{p, soup, tol};
use crate::{MeshError, VertexId};

/// A w x h grid of quads, each split along a chosen diagonal; optionally wrapped
/// around into a cylinder. Returns positions and triangles.
fn grid(w: usize, h: usize, diagonals: &[bool], cylinder: bool) -> (Vec<DVec3>, Vec<[u32; 3]>) {
    let cols = if cylinder { w } else { w + 1 };
    let mut pos = Vec::new();
    for j in 0..=h {
        for i in 0..cols {
            if cylinder {
                let a = std::f64::consts::TAU * i as f64 / w as f64;
                pos.push(p(a.cos(), a.sin(), j as f64 * 0.5));
            } else {
                pos.push(p(i as f64, j as f64, ((i * 7 + j * 3) % 5) as f64 * 0.1));
            }
        }
    }
    let id = |i: usize, j: usize| (j * cols + i % cols) as u32;
    let mut tris = Vec::new();
    for j in 0..h {
        for i in 0..w {
            let (a, b, c, d) = (id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1));
            if diagonals[(j * w + i) % diagonals.len()] {
                tris.push([a, b, c]);
                tris.push([a, c, d]);
            } else {
                tris.push([a, b, d]);
                tris.push([b, c, d]);
            }
        }
    }
    (pos, tris)
}

fn permute(tris: &mut [[u32; 3]], seed: u64) {
    // Deterministic Fisher-Yates and corner rotation, so a failure reproduces.
    let mut s = seed | 1;
    for i in (1..tris.len()).rev() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        tris.swap(i, (s % (i as u64 + 1)) as usize);
        tris[i].rotate_left((s % 3) as usize);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 300, rng_seed: proptest::test_runner::RngSeed::Fixed(20261014), ..ProptestConfig::default() })]

    /// Every clean disk or cylinder builds, validates, and has the right Euler
    /// characteristic (disk 1, cylinder 0), whatever the face order and corner rotation.
    #[test]
    fn clean_grids_build_and_validate(w in 3usize..9, h in 1usize..7, diagonals in proptest::collection::vec(any::<bool>(), 1..20),
                                      cylinder in any::<bool>(), seed in any::<u64>()) {
        let (pos, mut tris) = grid(w, h, &diagonals, cylinder);
        permute(&mut tris, seed);
        let s = soup(pos, tris);
        let t = tol(&s);
        let m = ManifoldMesh::from_oriented_triangles(&s, &t).unwrap();
        prop_assert!(m.debug_validate(&t).is_ok());
        let chi = m.vertex_count() as i64 - m.edge_count() as i64 + m.face_count() as i64;
        prop_assert_eq!(chi, if cylinder { 0 } else { 1 });
        prop_assert_eq!(m.boundary_loops().len(), if cylinder { 2 } else { 1 });
    }

    /// Deleting arbitrary faces from a grid (and the vertices left unused) leaves
    /// a surface that is manifold or not at each vertex. An independent oracle,
    /// the connectivity of each vertex's link, decides which: the builder must
    /// build exactly when every link is connected, and otherwise name the lowest
    /// vertex whose link is not.
    #[test]
    fn damaged_grids_build_exactly_when_every_vertex_link_is_connected(
        w in 3usize..8, h in 2usize..6, delete in proptest::collection::vec(any::<usize>(), 1..4), seed in any::<u64>()) {
        let (pos, kept) = damaged_grid(w, h, &delete);
        prop_assume!(!kept.is_empty());
        let bad = non_manifold_vertices(pos.len(), &kept);
        let mut kept = kept;
        permute(&mut kept, seed);
        let s = soup(pos, kept);
        let t = tol(&s);
        match ManifoldMesh::from_oriented_triangles(&s, &t) {
            Ok(m) => {
                prop_assert!(bad.is_empty(), "built, but the oracle says {:?} are non-manifold", bad);
                prop_assert!(m.debug_validate(&t).is_ok());
            }
            Err(e) => prop_assert_eq!(
                Some(e),
                bad.first().map(|&v| MeshError::NonManifoldVertex { vertex: VertexId::new(v) })
            ),
        }
    }
}

/// A `w` x `h` grid with the faces at `delete` (modulo the face count) removed,
/// and the vertices no remaining face uses removed too, so the result can only
/// fail by being non-manifold. A few deletions give a mix of holes (still
/// manifold) and faces meeting at a single vertex (bowties).
fn damaged_grid(w: usize, h: usize, delete: &[usize]) -> (Vec<DVec3>, Vec<[u32; 3]>) {
    let (pos, tris) = grid(w, h, &[true, false], false);
    let n = tris.len();
    let kept: Vec<[u32; 3]> = tris
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !delete.iter().any(|d| d % n == *i))
        .map(|(_, t)| t)
        .collect();
    let mut new_index = vec![None; pos.len()];
    let mut out = Vec::new();
    for &v in kept.iter().flatten() {
        if new_index[v as usize].is_none() {
            new_index[v as usize] = Some(out.len() as u32);
            out.push(v);
        }
    }
    // Renumber in original order, so vertex order does not depend on face order.
    out.sort();
    for (n, &v) in out.iter().enumerate() {
        new_index[v as usize] = Some(n as u32);
    }
    let pos = out.iter().map(|&v| pos[v as usize]).collect();
    let tris = kept
        .iter()
        .map(|t| t.map(|v| new_index[v as usize].unwrap()))
        .collect();
    (pos, tris)
}

/// The oracle: a vertex is manifold when its link (the edges opposite it in its
/// triangles) is connected: one path for a boundary vertex, one cycle for an
/// interior one. Shares no code with the builder.
fn non_manifold_vertices(n: usize, tris: &[[u32; 3]]) -> Vec<u32> {
    let mut links: Vec<Vec<(u32, u32)>> = vec![Vec::new(); n];
    for t in tris {
        for k in 0..3 {
            links[t[k] as usize].push((t[(k + 1) % 3], t[(k + 2) % 3]));
        }
    }
    let mut bad = Vec::new();
    for (v, link) in links.iter().enumerate() {
        // Union-find over the link's endpoints.
        let mut parent: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
        fn find(p: &mut std::collections::BTreeMap<u32, u32>, x: u32) -> u32 {
            let up = p[&x];
            if up == x {
                x
            } else {
                let r = find(p, up);
                p.insert(x, r);
                r
            }
        }
        for &(a, b) in link {
            parent.entry(a).or_insert(a);
            parent.entry(b).or_insert(b);
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            parent.insert(ra, rb);
        }
        let keys: Vec<u32> = parent.keys().copied().collect();
        let roots: std::collections::BTreeSet<u32> =
            keys.into_iter().map(|k| find(&mut parent, k)).collect();
        if roots.len() > 1 {
            bad.push(v as u32);
        }
    }
    bad
}

/// The property above is only meaningful if both outcomes are common. Over a
/// fixed set of damage patterns, at least a fifth must build and a fifth must be
/// refused as non-manifold.
#[test]
fn damaged_grids_produce_both_outcomes() {
    let (mut built, mut refused, mut s) = (0, 0, 0x9e37_79b9_7f4a_7c15u64);
    for _ in 0..300 {
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s as usize
        };
        let count = 1 + next() % 3;
        let delete: Vec<usize> = (0..count).map(|_| next()).collect();
        let (pos, tris) = damaged_grid(5, 4, &delete);
        if tris.is_empty() {
            continue;
        }
        let so = soup(pos, tris);
        match ManifoldMesh::from_oriented_triangles(&so, &tol(&so)) {
            Ok(_) => built += 1,
            Err(MeshError::NonManifoldVertex { .. }) => refused += 1,
            Err(e) => panic!("unexpected refusal {e:?}"),
        }
    }
    assert!(
        built >= 60 && refused >= 60,
        "built {built}, refused {refused}"
    );
}
