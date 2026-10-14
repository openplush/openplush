//! Property tests for `ManifoldMesh` (T027): every mesh the builder accepts
//! satisfies `debug_validate`, and every mesh it refuses gets a typed error.
//!
//! In-crate rather than `tests/invariants.rs` (as tasks.md names it) because the
//! builder is crate-private: only repair may construct a `ManifoldMesh`.

use openplush_geom::DVec3;
use proptest::prelude::*;

use super::ManifoldMesh;
use super::tests::{p, soup, tol};
use crate::MeshError;

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

    /// Deleting arbitrary faces can create bowties and isolated vertices. The builder
    /// must either produce a mesh that passes every invariant or return a typed error;
    /// never a panic, never an invalid mesh.
    #[test]
    fn damaged_grids_build_valid_or_refuse(w in 3usize..8, h in 2usize..6, keep in proptest::collection::vec(any::<bool>(), 1..40),
                                           seed in any::<u64>()) {
        let (pos, tris) = grid(w, h, &[true, false], false);
        let mut kept: Vec<[u32; 3]> = tris.into_iter().enumerate().filter(|(i, _)| keep[i % keep.len()]).map(|(_, t)| t).collect();
        prop_assume!(!kept.is_empty());
        permute(&mut kept, seed);
        let s = soup(pos, kept);
        let t = tol(&s);
        match ManifoldMesh::from_oriented_triangles(&s, &t) {
            Ok(m) => prop_assert!(m.debug_validate(&t).is_ok()),
            Err(e) => prop_assert!(
                matches!(e, MeshError::IsolatedVertex { .. } | MeshError::NonManifoldVertex { .. }),
                "unexpected refusal: {:?}",
                e
            ),
        }
    }
}
