//! Grouping pieces into templates: by shape as printed or flipped, only when their seams sew to
//! the same templates edge for edge, and near mirror images reported, never merged.
#![allow(clippy::expect_used, clippy::panic, reason = "test setup")]

use std::collections::BTreeSet;

use openplush_geom::DVec2;
use openplush_pattern::Outline;
use openplush_pattern::dedupe::Facing;
use openplush_seams::SeamChainId;

use super::{EdgeSeam, alike, group};

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

fn rect(w: f64, h: f64) -> Outline {
    outline(&[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)])
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

/// The L with its corner 3 moved `d` mm left: fitted, its largest corner gap is 5/6 of `d`.
fn nudged(points: &[(f64, f64)], d: f64) -> Outline {
    let mut p = points.to_vec();
    p[3].0 -= d;
    outline(&p)
}

/// The mirror L with its corner 2, its match for the L's corner 3, moved `d` mm right: fitted
/// flipped, its largest gap is 5/6 of `d`.
fn nudged_mirror(d: f64) -> Outline {
    let mut p = mirror(&ELL);
    p[2].0 += d;
    outline(&p)
}

#[test]
fn a_mirror_within_the_1_mm_floor_merges_and_one_past_it_does_not() {
    let a = outline(&ELL);
    let close = nudged_mirror(0.96);
    let g = group(&[&a, &close], &[bare(6), bare(6)]);
    let fit = g.members[1].fit.expect("merged at a 0.8 mm gap");
    assert!((fit.gap_mm() - 0.8).abs() < 1e-9, "{}", fit.gap_mm());
    let past = nudged_mirror(1.8);
    let g = group(&[&a, &past], &[bare(6), bare(6)]);
    assert_eq!(g.members[1].template, 1, "apart at a 1.5 mm gap");
    let [near] = &g.near[..] else {
        panic!("a 1.5 mm gap is a near match: {:?}", g.near);
    };
    assert!((near.gap_mm - 1.5).abs() < 1e-9);
}

#[test]
fn a_near_match_reaches_to_5_mm_and_no_further() {
    let a = outline(&ELL);
    let g = group(&[&a, &nudged_mirror(5.7)], &[bare(6), bare(6)]);
    assert_eq!(g.near.len(), 1, "4.75 mm is near");
    let g = group(&[&a, &nudged_mirror(6.6)], &[bare(6), bare(6)]);
    assert!(g.near.is_empty(), "5.5 mm is not: {:?}", g.near);
}

#[test]
fn identical_pieces_join_as_printed_even_when_flipped_also_fits() {
    let (a, b) = (square(40.0), square(40.0));
    let g = group(&[&a, &b], &[bare(4), bare(4)]);
    assert_eq!(
        g.members[1].fit.expect("merged").facing(),
        Facing::AsPrinted
    );
}

#[test]
fn a_piece_joins_the_first_template_it_fits() {
    // Piece 1 is 1.6 mm off piece 0, so its own template; piece 2 is 0.8 mm off both.
    let (a, b, c) = (outline(&ELL), nudged(&ELL, 1.92), nudged(&ELL, 0.96));
    let g = group(&[&a, &b, &c], &[bare(6), bare(6), bare(6)]);
    assert_eq!(g.members[1].template, 1);
    assert_eq!(g.members[2].template, 0);
}

#[test]
fn near_matches_are_between_templates_only() {
    let (a, b, c) = (outline(&ELL), outline(&ELL), nudged_mirror(3.6));
    let g = group(&[&a, &b, &c], &[bare(6), bare(6), bare(6)]);
    assert_eq!(g.members[1].template, 0);
    let pairs: Vec<(usize, usize)> = g.near.iter().map(|n| (n.a, n.b)).collect();
    assert_eq!(pairs, [(0, 2)]);
}

#[test]
fn a_sewn_edge_never_matches_an_unsewn_one() {
    let (a, b) = (outline(&ELL), outline(&ELL));
    let mut lonely = bare(6);
    lonely[2].chain = Some(SeamChainId::new(9));
    let g = group(&[&a, &b], &[lonely, bare(6)]);
    assert_eq!(g.members[1].template, 1);
}

#[test]
fn a_symmetric_piece_merges_under_whichever_turn_lines_its_seams_up() {
    // Two squares sewn to the L on different edges: turned a quarter, the seams line up.
    let (s, t, l) = (square(40.0), square(40.0), outline(&ELL));
    let edges = [sewn(4, 0, 1, &[2]), sewn(4, 1, 2, &[2]), bare(6)];
    let g = group(&[&s, &t, &l], &edges);
    assert_eq!(g.members[1].template, 0);
}

/// `n` edges, each `(edge, chain, partner)` sewn, the rest unsewn.
fn sewn_many(n: usize, seams: &[(usize, u32, usize)]) -> Vec<EdgeSeam> {
    let mut edges = bare(n);
    for &(e, chain, partner) in seams {
        edges[e] = EdgeSeam {
            chain: Some(SeamChainId::new(chain)),
            partners: BTreeSet::from([partner]),
        };
    }
    edges
}

#[test]
fn regrouping_runs_until_a_split_has_reached_every_piece_it_touches() {
    // U and V share a shape but not their seams; S and T match only while U and V do, and A and
    // its twin only while S and T do. Each split shows only on the next pass.
    let (u, v) = (outline(&ELL), outline(&ELL));
    let (s, t) = (square(40.0), square(40.0));
    let (a, a2) = (rect(20.0, 50.0), rect(20.0, 50.0));
    let edges = [
        sewn_many(6, &[(0, 1, 2)]),
        sewn_many(6, &[(1, 2, 3)]),
        sewn_many(4, &[(0, 1, 0), (2, 3, 4)]),
        sewn_many(4, &[(0, 2, 1), (2, 4, 5)]),
        sewn_many(4, &[(0, 3, 2)]),
        sewn_many(4, &[(0, 4, 3)]),
    ];
    let g = group(&[&u, &v, &s, &t, &a, &a2], &edges);
    let templates: Vec<usize> = g.members.iter().map(|m| m.template).collect();
    assert_eq!(templates, [0, 1, 2, 3, 4, 5]);
}

#[test]
fn matched_seams_share_the_lowest_chain_and_list_the_template_s_name_first() {
    let (a, b, c) = (outline(&ELL), outline(&ELL), outline(&mirror(&ELL)));
    let edges = [
        sewn_many(6, &[(0, 30, 0)]),
        sewn_many(6, &[(0, 20, 0)]),
        sewn_many(6, &[(4, 10, 0)]),
    ];
    let g = group(&[&a, &b, &c], &edges);
    assert_eq!((g.members[1].template, g.members[2].template), (0, 0));
    let al = alike(&g, &edges);
    let names: Vec<u32> = al
        .names(0, SeamChainId::new(30))
        .iter()
        .map(|c| c.index())
        .collect();
    assert_eq!(names, [30, 20, 10]);
    for chain in [30, 20, 10] {
        assert_eq!(al.of(SeamChainId::new(chain)), SeamChainId::new(10));
    }
    assert_eq!((al.template_of(2), al.template_of(7)), (0, 7));
}
