//! The density rule and the edge swaps, on hand-made patches.

use openplush_geom::DVec3;
use openplush_geom::epsilon::ModelTolerances;

use super::{Patch, relax, split, swappable};

fn tol(diagonal: f64) -> ModelTolerances {
    ModelTolerances::for_diagonal(diagonal).unwrap()
}

/// One triangle with the given corners and per-corner σ.
fn triangle(pos: [DVec3; 3], sigma: [f64; 3]) -> Patch {
    Patch {
        pos: pos.to_vec(),
        sigma: sigma.to_vec(),
        rim: 3,
        tris: vec![[0, 1, 2]],
    }
}

/// The pair `(a, b, c)`, `(b, a, d)` sharing edge a-b, all four on the rim.
fn pair(a: DVec3, b: DVec3, c: DVec3, d: DVec3) -> Patch {
    Patch {
        pos: vec![a, b, c, d],
        sigma: vec![1.0; 4],
        rim: 4,
        tris: vec![[0, 1, 2], [1, 0, 3]],
    }
}

/// Edge (-1,0,0)-(1,0,0) with its two opposite corners 0.2 off it: far past Delaunay.
fn flat_pair() -> Patch {
    pair(
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, -0.2, 0.0),
        DVec3::new(0.0, 0.2, 0.0),
    )
}

/// An equilateral triangle of side `side` in z = 0; its centroid is `side / sqrt(3)` from
/// each corner.
fn equilateral(side: f64) -> [DVec3; 3] {
    let h = side * 3f64.sqrt() / 2.0;
    [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(side, 0.0, 0.0),
        DVec3::new(side / 2.0, h, 0.0),
    ]
}

#[test]
fn a_centroid_closer_than_sigma_over_root_two_is_not_split() {
    // The centroid is 1/sqrt(3) = 0.577 from each corner: past σ/2 but short of σ/√2.
    let mut patch = triangle(equilateral(1.0), [1.0; 3]);
    assert!(!split(&mut patch));
    assert_eq!(patch.tris, vec![[0, 1, 2]]);
    assert_eq!(patch.pos.len(), 3);
}

#[test]
fn a_triangle_small_against_one_corner_s_own_sigma_is_not_split() {
    // √2 times the centroid distance is 1.633 at every corner: past the mean σ of 1.5 and
    // past corners 0 and 1, but not corner 2's own 2.5.
    let mut patch = triangle(equilateral(2.0), [1.0, 1.0, 2.5]);
    assert!(!split(&mut patch));
    assert_eq!(patch.tris.len(), 1);
}

#[test]
fn a_triangle_small_against_the_mean_sigma_is_not_split() {
    // The apex at (2, 1) is 2/3 from the centroid, 0.943 after √2: past its own σ of 0.5
    // but not the mean σ of 1.83; the base corners pass both.
    let pos = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(4.0, 0.0, 0.0),
        DVec3::new(2.0, 1.0, 0.0),
    ];
    let mut patch = triangle(pos, [2.5, 2.5, 0.5]);
    assert!(!split(&mut patch));
    assert_eq!(patch.tris.len(), 1);
}

#[test]
fn a_triangle_large_at_every_corner_is_split_at_its_centroid() {
    let mut patch = triangle(equilateral(2.0), [1.0; 3]);
    assert!(split(&mut patch));
    assert_eq!(patch.tris, vec![[0, 1, 3], [1, 2, 3], [2, 0, 3]]);
    assert_eq!(patch.sigma[3], 1.0);
}

#[test]
fn an_edge_past_delaunay_is_swapped() {
    assert_eq!(
        swappable(&flat_pair(), &|_, _| false, &tol(2.0)),
        Some((0, 1, 2, 3))
    );
}

#[test]
fn a_swap_onto_a_patch_edge_is_refused() {
    let mut patch = flat_pair();
    patch.pos.push(DVec3::new(0.0, 0.0, 1.0));
    patch.tris.push([2, 3, 4]);
    assert_eq!(swappable(&patch, &|_, _| false, &tol(2.0)), None);
}

#[test]
fn a_swap_onto_an_edge_the_mesh_has_between_rim_vertices_is_refused() {
    let joined = |i: usize, k: usize| (i, k) == (2, 3);
    assert_eq!(swappable(&flat_pair(), &joined, &tol(2.0)), None);
}

#[test]
fn a_cocircular_pair_is_left_as_it_is() {
    // A square's diagonal: both opposite angles are right angles, cotangents exactly 0.
    let patch = pair(
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 1.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    );
    assert_eq!(swappable(&patch, &|_, _| false, &tol(2.0)), None);
}

#[test]
fn a_swap_that_would_leave_a_sliver_is_refused() {
    // Edge a-b is 1e5 long with c and d 1 off it, and a 1e-4 behind chord c-d, so the
    // angles at c and d pass π. Swapped, triangle a-d-c has area 1e-4, below the 1e-2 a
    // diagonal of 1e5 allows.
    let patch = pair(
        DVec3::new(-1e-4, 0.0, 0.0),
        DVec3::new(1e5, 0.0, 0.0),
        DVec3::new(0.0, -1.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    );
    assert_eq!(swappable(&patch, &|_, _| false, &tol(1e5)), None);
}

#[test]
fn a_swap_that_would_fold_the_pair_over_is_refused() {
    // Found by search: the pair bends by about 86 degrees and fails the Delaunay test,
    // but the swapped pair's normals point apart.
    let patch = pair(
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(-1.08, -0.08, -0.06),
        DVec3::new(0.08, -0.19, 0.29),
    );
    assert_eq!(swappable(&patch, &|_, _| false, &tol(2.0)), None);
}

#[test]
fn a_pair_that_keeps_swapping_is_refused() {
    // An isosceles trapezoid on the unit circle. Cocircular, so in exact arithmetic both
    // diagonals sit on the Delaunay boundary; rounding puts both cotangent sums at -5.6e-17,
    // so each swap undoes the last until the bound.
    let mut patch = pair(
        DVec3::new(-0.8, -0.6, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.8, -0.6, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
    );
    assert_eq!(relax(&mut patch, &|_, _| false, &tol(2.0)), None);
}
