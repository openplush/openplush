//! Grouping pieces into templates: by shape as printed or flipped, only when their seams sew to
//! the same templates edge for edge, and near mirror images reported, never merged.
#![allow(clippy::expect_used, clippy::panic, reason = "test setup")]

use std::collections::BTreeSet;

use openplush_geom::DVec2;
use openplush_pattern::Outline;
use openplush_pattern::dedupe::Facing;
use openplush_seams::SeamChainId;

use super::{EdgeSeam, group};

fn outline(points: &[(f64, f64)]) -> Outline {
    Outline::new(points.iter().map(|&(x, y)| DVec2::new(x, y)).collect()).expect("valid")
}

const ELL: [(f64, f64); 6] = [
    (0.0, 0.0),
    (90.0, 0.0),
    (90.0, 30.0),
    (30.0, 30.0),
    (30.0, 60.0),
    (0.0, 60.0),
];

/// The L mirrored left to right and reversed, so corner `i` of the L is its corner `5 - i`, and
/// edge `i` of the L its edge `4 - i` (mod 6).
fn mirror(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    points.iter().rev().map(|&(x, y)| (-x, y)).collect()
}

fn square(side: f64) -> Outline {
    outline(&[(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)])
}

/// Every edge unsewn.
fn bare(n: usize) -> Vec<EdgeSeam> {
    (0..n).map(|_| EdgeSeam::default()).collect()
}

/// Edge `edge` sewn along `chain` to the pieces `partners`, the rest unsewn.
fn sewn(n: usize, edge: usize, chain: u32, partners: &[usize]) -> Vec<EdgeSeam> {
    let mut edges = bare(n);
    edges[edge] = EdgeSeam {
        chain: Some(SeamChainId::new(chain)),
        partners: partners.iter().copied().collect::<BTreeSet<_>>(),
    };
    edges
}

#[test]
fn two_identical_pieces_are_one_template_cut_as_printed() {
    let (a, b) = (outline(&ELL), outline(&ELL));
    let g = group(&[&a, &b], &[bare(6), bare(6)]);
    assert_eq!(g.members[0].template, 0);
    assert!(g.members[0].fit.is_none());
    assert_eq!(g.members[1].template, 0);
    let fit = g.members[1].fit.expect("a fit");
    assert_eq!(fit.facing(), Facing::AsPrinted);
    assert!(g.near.is_empty());
}

#[test]
fn a_mirror_image_joins_its_original_flipped() {
    let (a, b) = (outline(&ELL), outline(&mirror(&ELL)));
    let g = group(&[&a, &b], &[bare(6), bare(6)]);
    assert_eq!(g.members[1].template, 0);
    assert_eq!(g.members[1].fit.expect("a fit").facing(), Facing::Flipped);
}

#[test]
fn a_different_shape_stays_its_own_template() {
    let (a, b) = (outline(&ELL), square(40.0));
    let g = group(&[&a, &b], &[bare(6), bare(4)]);
    assert_eq!((g.members[0].template, g.members[1].template), (0, 1));
}

#[test]
fn a_mirror_image_3_mm_off_is_a_near_match_and_stays_apart() {
    let mut off = mirror(&ELL);
    off[3].0 -= 3.0;
    let (a, b) = (outline(&ELL), outline(&off));
    let g = group(&[&a, &b], &[bare(6), bare(6)]);
    assert_eq!(g.members[1].template, 1);
    let [near] = &g.near[..] else {
        panic!("one near match: {:?}", g.near);
    };
    assert_eq!((near.a, near.b), (0, 1));
    assert!(near.gap_mm > 1.0 && near.gap_mm <= 3.0, "{}", near.gap_mm);
    assert!(near.seams_mirror);
}

#[test]
fn a_mirror_image_6_mm_off_is_simply_another_piece() {
    let mut off = mirror(&ELL);
    off[3].0 -= 12.0;
    let (a, b) = (outline(&ELL), outline(&off));
    let g = group(&[&a, &b], &[bare(6), bare(6)]);
    assert_eq!(g.members[1].template, 1);
    assert!(g.near.is_empty(), "{:?}", g.near);
}

#[test]
fn matching_shapes_sewn_to_different_pieces_stay_apart() {
    // L and its mirror sew along matching edges, to a square and to a differently sized one.
    let (a, b, s, t) = (
        outline(&ELL),
        outline(&mirror(&ELL)),
        square(40.0),
        square(50.0),
    );
    let edges = [sewn(6, 0, 1, &[2]), sewn(6, 4, 2, &[3]), bare(4), bare(4)];
    let g = group(&[&a, &b, &s, &t], &edges);
    assert_eq!(g.members[1].template, 1);
}

#[test]
fn partners_that_are_one_template_let_their_pieces_merge() {
    let (a, b, s, t) = (
        outline(&ELL),
        outline(&mirror(&ELL)),
        square(40.0),
        square(40.0),
    );
    let edges = [sewn(6, 0, 1, &[2]), sewn(6, 4, 2, &[3]), bare(4), bare(4)];
    let g = group(&[&a, &b, &s, &t], &edges);
    assert_eq!(g.members[3].template, 2);
    assert_eq!(g.members[1].template, 0);
}

#[test]
fn a_seam_on_an_edge_that_does_not_correspond_blocks_the_merge() {
    // Edge 0 of the L lands on edge 4 of its mirror, not edge 0.
    let (a, b, s) = (outline(&ELL), outline(&mirror(&ELL)), square(40.0));
    let edges = [sewn(6, 0, 1, &[2]), sewn(6, 0, 2, &[2]), bare(4)];
    let g = group(&[&a, &b, &s], &edges);
    assert_eq!(g.members[1].template, 1);
    // Within the seam-parity floor it is no near match: the seams are what differ.
    assert!(g.near.is_empty(), "{:?}", g.near);
}

#[test]
fn a_near_match_says_when_its_seams_do_not_mirror_each_other() {
    let mut off = mirror(&ELL);
    off[3].0 -= 3.0;
    let (a, b, s) = (outline(&ELL), outline(&off), square(40.0));
    let edges = [sewn(6, 0, 1, &[2]), sewn(6, 0, 2, &[2]), bare(4)];
    let g = group(&[&a, &b, &s], &edges);
    let [near] = &g.near[..] else {
        panic!("one near match: {:?}", g.near);
    };
    assert!(!near.seams_mirror);
}
