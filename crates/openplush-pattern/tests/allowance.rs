//! `allowance::cut_line`: the cut line from per-edge strips, against hand-computed outlines.

use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_pattern::allowance::cut_line;
use openplush_pattern::{Outline, PatternError, PieceId};

const PIECE: PieceId = PieceId::new(5);

fn outline(points: &[(f64, f64)]) -> Result<Outline, PatternError> {
    Outline::new(points.iter().map(|&(x, y)| DVec2::new(x, y)).collect())
}

fn mm(values: &[f64]) -> Vec<Mm> {
    values
        .iter()
        .map(|&v| Mm::try_new(v).unwrap_or_default())
        .collect()
}

/// Shoelace area of a counter-clockwise outline.
fn area(o: &Outline) -> f64 {
    let p = o.points();
    let n = p.len();
    (0..n).map(|i| p[i].perp_dot(p[(i + 1) % n])).sum::<f64>() / 2.0
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-6,
        "{actual} is not {expected}"
    );
}

/// The points on a 1e-6 mm grid, sorted, so two outlines compare whatever their start.
fn grid(points: impl Iterator<Item = (f64, f64)>) -> Vec<(i64, i64)> {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "test coordinates are small"
    )]
    let mut c: Vec<(i64, i64)> = points
        .map(|(x, y)| ((x * 1e6).round() as i64, (y * 1e6).round() as i64))
        .collect();
    c.sort();
    c
}

fn corners(o: &Outline) -> Vec<(i64, i64)> {
    grid(o.points().iter().map(|p| (p.x, p.y)))
}

fn square() -> Result<Outline, PatternError> {
    outline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])
}

#[test]
fn a_uniform_allowance_adds_a_strip_along_every_edge() -> Result<(), PatternError> {
    let cut = cut_line(PIECE, &square()?, &mm(&[1.0; 4]))?;
    // Four strips with a mitered square at each corner: the square 1 larger all round.
    assert_close(area(&cut), 144.0);
    let expected = [(-1.0, -1.0), (11.0, -1.0), (11.0, 11.0), (-1.0, 11.0)];
    assert_eq!(corners(&cut), grid(expected.into_iter()));
    Ok(())
}

#[test]
fn each_edge_gets_its_own_allowance() -> Result<(), PatternError> {
    let cut = cut_line(PIECE, &square()?, &mm(&[1.0, 2.0, 3.0, 4.0]))?;
    // Mitered corners make it the rectangle [-4, 12] × [-1, 13].
    assert_close(area(&cut), 224.0);
    assert_eq!(cut.points().len(), 4);
    let xs = cut.points().iter().map(|p| p.x);
    let ys = cut.points().iter().map(|p| p.y);
    assert_close(xs.clone().fold(f64::INFINITY, f64::min), -4.0);
    assert_close(xs.fold(f64::NEG_INFINITY, f64::max), 12.0);
    assert_close(ys.clone().fold(f64::INFINITY, f64::min), -1.0);
    assert_close(ys.fold(f64::NEG_INFINITY, f64::max), 13.0);
    Ok(())
}

#[test]
fn zero_allowance_everywhere_cuts_on_the_sewing_line() -> Result<(), PatternError> {
    let sewing = square()?;
    assert_eq!(cut_line(PIECE, &sewing, &mm(&[0.0; 4]))?, sewing);
    Ok(())
}

#[test]
fn a_fold_edge_gets_no_strip() -> Result<(), PatternError> {
    let cut = cut_line(PIECE, &square()?, &mm(&[1.0, 0.0, 1.0, 1.0]))?;
    // [-1, 10] × [-1, 11]: the corners beside the fold run out to its line and stop.
    assert_close(area(&cut), 132.0);
    assert!(cut.points().iter().all(|p| p.x <= 10.0));
    Ok(())
}

/// An L: a 20 × 10 foot and a 10 × 10 upright, reflex corner at (10, 10). Edge 3 runs up the
/// inside of the upright.
fn ell() -> Result<Outline, PatternError> {
    outline(&[
        (0.0, 0.0),
        (20.0, 0.0),
        (20.0, 10.0),
        (10.0, 10.0),
        (10.0, 20.0),
        (0.0, 20.0),
    ])
}

#[test]
fn no_strip_reaches_past_a_fold() -> Result<(), PatternError> {
    // Edge 3 is a fold. Its guard is 4 × 2 = 8 deep, so it trims the foot's top strip to the
    // 2 × 2 at its far end: 300 + 40 + 20 + 4 + 20 + 40, and a 2 × 2 at each of the four
    // corners away from the fold.
    let cut = cut_line(PIECE, &ell()?, &mm(&[2.0, 2.0, 2.0, 0.0, 2.0, 2.0]))?;
    assert_close(area(&cut), 440.0);
    Ok(())
}

#[test]
fn a_reflex_corner_overlaps_its_strips() -> Result<(), PatternError> {
    // The two strips at (10, 10) overlap in a 1 × 1 square, counted once; the five convex
    // corners add a 1 × 1 each.
    let cut = cut_line(PIECE, &ell()?, &mm(&[1.0; 6]))?;
    assert_close(area(&cut), 300.0 + 80.0 - 1.0 + 5.0);
    Ok(())
}

#[test]
fn a_count_that_is_not_one_per_edge_is_refused() -> Result<(), PatternError> {
    for given in [3, 5] {
        assert_eq!(
            cut_line(PIECE, &square()?, &mm(&vec![1.0; given])),
            Err(PatternError::AllowanceCount {
                piece: PIECE,
                edges: 4,
                allowances: given,
            })
        );
    }
    Ok(())
}

#[test]
fn allowances_that_close_off_a_pocket_are_refused() -> Result<(), PatternError> {
    // A 4 × 4 pocket inside a 10 × 10 square, open through a 0.5 wide slot: the slot's strips
    // close it, and the pocket's strips leave a 2 × 2 gap inside.
    let pocket = outline(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (5.25, 10.0),
        (5.25, 7.0),
        (7.0, 7.0),
        (7.0, 3.0),
        (3.0, 3.0),
        (3.0, 7.0),
        (4.75, 7.0),
        (4.75, 10.0),
        (0.0, 10.0),
    ])?;
    assert_eq!(
        cut_line(PIECE, &pocket, &mm(&[1.0; 12])),
        Err(PatternError::AllowanceEnclosesGap { piece: PIECE })
    );
    Ok(())
}

/// A 10 × 10 square with a 1 wide slot down to y = 4. Edge 3 is the slot's right side.
fn slotted() -> Result<Outline, PatternError> {
    outline(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (6.0, 10.0),
        (6.0, 4.0),
        (5.0, 4.0),
        (5.0, 10.0),
        (0.0, 10.0),
    ])
}

#[test]
fn a_fold_whose_guard_reaches_into_the_piece_is_refused() -> Result<(), PatternError> {
    // Edge 3's guard is 4 deep and crosses the 1 wide slot into the left half.
    let mut allowances = [1.0; 8];
    allowances[3] = 0.0;
    assert_eq!(
        cut_line(PIECE, &slotted()?, &mm(&allowances)),
        Err(PatternError::FoldConflict {
            piece: PIECE,
            edge: 3,
        })
    );
    Ok(())
}

#[test]
fn a_fold_whose_guard_stays_outside_the_piece_is_kept() -> Result<(), PatternError> {
    // The same slot with 0.2 allowances: the guard is 0.8 deep, inside the 1 wide slot.
    let mut allowances = [0.2; 8];
    allowances[3] = 0.0;
    let cut = cut_line(PIECE, &slotted()?, &mm(&allowances))?;
    assert!(area(&cut) > 94.0);
    Ok(())
}

#[test]
fn an_edge_far_shorter_than_the_piece_is_merged_into_the_next() -> Result<(), PatternError> {
    // The top edge, split by a point 1e-9 along it. That edge is under the merge length, so
    // it goes, and its 5 mm allowance with it: the cut line is the plain square's.
    let split = outline(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (5.0, 10.0),
        (5.0 - 1e-9, 10.0),
        (0.0, 10.0),
    ])?;
    let cut = cut_line(PIECE, &split, &mm(&[1.0, 1.0, 1.0, 5.0, 1.0, 1.0]))?;
    let plain = cut_line(PIECE, &square()?, &mm(&[1.0; 4]))?;
    assert_eq!(corners(&cut), corners(&plain));
    Ok(())
}

/// The plain square's cut line at 1 mm all round.
fn plain() -> Result<Vec<(i64, i64)>, PatternError> {
    Ok(corners(&cut_line(PIECE, &square()?, &mm(&[1.0; 4]))?))
}

#[test]
fn a_short_first_edge_is_merged_and_merging_goes_on() -> Result<(), PatternError> {
    // The same split top edge, starting at the split, and the right edge split as well: both
    // 1e-9 edges go, the second after the first.
    let split = outline(&[
        (5.0, 10.0),
        (5.0 - 1e-9, 10.0),
        (0.0, 10.0),
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 5.0),
        (10.0, 5.0 + 1e-9),
        (10.0, 10.0),
    ])?;
    let cut = cut_line(
        PIECE,
        &split,
        &mm(&[5.0, 1.0, 1.0, 1.0, 1.0, 5.0, 1.0, 1.0]),
    )?;
    assert_eq!(corners(&cut), plain()?);
    Ok(())
}

#[test]
fn merging_carries_on_past_the_last_edge() -> Result<(), PatternError> {
    // Two short edges around (0, 0): the last edge's merge runs into the first, which is short
    // as well, so both go.
    let split = outline(&[
        (0.0, -1.3e-5),
        (1.2e-5, 0.5e-5),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
        (0.0, 0.0),
    ])?;
    let cut = cut_line(PIECE, &split, &mm(&[5.0, 1.0, 1.0, 1.0, 1.0, 1.0]))?;
    assert_eq!(corners(&cut), plain()?);
    Ok(())
}

/// The square with its top edge split by an edge `short` long at x = 5, offset by `at`.
fn split_top(short: f64, at: f64) -> Result<Outline, PatternError> {
    let p = |x: f64, y: f64| (x + at, y + at);
    outline(&[
        p(0.0, 0.0),
        p(10.0, 0.0),
        p(10.0, 10.0),
        p(5.0, 10.0),
        p(5.0 - short, 10.0),
        p(0.0, 10.0),
    ])
}

#[test]
fn an_edge_over_the_merge_length_keeps_its_allowance() -> Result<(), PatternError> {
    // The merge length is 1e-6 of the 14.1 mm diagonal: a 2e-5 edge stays and gets its 5 mm,
    // wherever the piece lies.
    for at in [0.0, 1000.0] {
        let cut = cut_line(
            PIECE,
            &split_top(2e-5, at)?,
            &mm(&[1.0, 1.0, 1.0, 5.0, 1.0, 1.0]),
        )?;
        let top = cut
            .points()
            .iter()
            .map(|p| p.y)
            .fold(f64::NEG_INFINITY, f64::max);
        assert_close(top, 15.0 + at);
    }
    Ok(())
}

#[test]
fn merging_stops_at_a_triangle() -> Result<(), PatternError> {
    // Edge 1 is 1e-9 long, but a triangle has no edge to spare.
    let sliver = outline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 1e-9)])?;
    let cut = cut_line(PIECE, &sliver, &mm(&[1.0; 3]))?;
    // Two 10 × 1 strips and a 1 × 1 miter at each right angle; the sliver corner is beveled.
    assert_close(area(&cut), 22.0);
    Ok(())
}

#[test]
fn a_merge_that_folds_the_outline_is_refused() -> Result<(), PatternError> {
    // The slit's 1e-9 tip is merged away, which leaves the outline doubling back at (5, 10).
    let slit = outline(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (5.0, 10.0),
        (5.0, 10.0 + 1e-9),
        (12.0, 10.0),
        (12.0, 12.0),
        (0.0, 12.0),
    ])?;
    assert_eq!(
        cut_line(PIECE, &slit, &mm(&[1.0; 8])),
        Err(PatternError::AllowanceNotSimple { piece: PIECE })
    );
    Ok(())
}

#[test]
fn a_guard_just_deeper_than_the_slot_is_a_conflict() -> Result<(), PatternError> {
    // 4 × 0.25005 = 1.0002 reaches 0.0002 past the 1 wide slot: 1.2e-3 mm² of overlap.
    let mut allowances = [0.25005; 8];
    allowances[3] = 0.0;
    assert_eq!(
        cut_line(PIECE, &slotted()?, &mm(&allowances)),
        Err(PatternError::FoldConflict {
            piece: PIECE,
            edge: 3,
        })
    );
    Ok(())
}

#[test]
fn a_conflict_names_the_edge_of_the_sewing_line() -> Result<(), PatternError> {
    // The slot's right side starts with a 1e-9 edge, merged into the fold after it: edge 4.
    let points = [
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (6.0, 10.0),
        (6.0, 10.0 - 1e-9),
        (6.0, 4.0),
        (5.0, 4.0),
        (5.0, 10.0),
        (0.0, 10.0),
    ];
    let mut allowances = [1.0; 9];
    allowances[4] = 0.0;
    assert_eq!(
        cut_line(PIECE, &outline(&points)?, &mm(&allowances)),
        Err(PatternError::FoldConflict {
            piece: PIECE,
            edge: 4,
        })
    );
    Ok(())
}

#[test]
fn a_guard_that_cuts_a_strip_from_the_piece_is_refused() -> Result<(), PatternError> {
    // A tab hangs off the right on a neck 1e-5 wide. The fold guard over edge 2 overlaps the
    // piece by under the weld tolerance, so it is no conflict, but it cuts the neck: two parts.
    let h = 1e-5;
    let tab = outline(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 2.0),
        (1.0, 2.0),
        (1.0, 3.0),
        (10.5, 3.0),
        (10.5, 2.5),
        (12.0, 2.5),
        (12.0, 6.0),
        (10.5, 6.0),
        (10.5, 3.0 + h),
        (1.0, 3.0 + h),
        (1.0, 5.0),
        (0.0, 5.0),
    ])?;
    let mut allowances = [0.4; 14];
    allowances[2] = 0.0;
    assert_eq!(
        cut_line(PIECE, &tab, &mm(&allowances)),
        Err(PatternError::AllowanceSplitsPiece { piece: PIECE })
    );
    Ok(())
}

#[test]
fn a_fold_beside_a_square_notch_is_no_conflict_at_any_angle() -> Result<(), PatternError> {
    // The L turned 0.123 rad: the fold's guard runs along the notch's other side, and rounding
    // alone makes it overlap the piece. Without the weld-sized allowance for that, this fails.
    let (s, c) = libm::sincos(0.123);
    let turned: Vec<(f64, f64)> = ell()?
        .points()
        .iter()
        .map(|p| (37.1 + p.x * c - p.y * s, -12.7 + p.x * s + p.y * c))
        .collect();
    let cut = cut_line(
        PIECE,
        &outline(&turned)?,
        &mm(&[2.0, 2.0, 2.0, 0.0, 2.0, 2.0]),
    );
    assert!(cut.is_ok(), "{cut:?}");
    Ok(())
}

/// An isosceles triangle 100 tall with apex angle `apex_deg` at (0, 100).
fn spike(apex_deg: f64) -> Result<Outline, PatternError> {
    let half = 100.0 * libm::tan(apex_deg.to_radians() / 2.0);
    outline(&[(-half, 0.0), (half, 0.0), (0.0, 100.0)])
}

/// The farthest cut-line point from the apex among those near it.
fn reach_at_apex(cut: &Outline) -> f64 {
    let apex = DVec2::new(0.0, 100.0);
    cut.points()
        .iter()
        .map(|&p| (p - apex).length())
        .filter(|&d| d < 10.0)
        .fold(0.0, f64::max)
}

#[test]
fn a_corner_under_29_degrees_is_beveled() -> Result<(), PatternError> {
    // A miter at 20° would reach 1 / sin 10° = 5.76, past 4 × the allowance; the bevel's
    // points are each 1 from the apex.
    let cut = cut_line(PIECE, &spike(20.0)?, &mm(&[1.0; 3]))?;
    assert_close(reach_at_apex(&cut), 1.0);
    Ok(())
}

#[test]
fn a_corner_over_29_degrees_is_mitered() -> Result<(), PatternError> {
    // 1 / sin 20°.
    let cut = cut_line(PIECE, &spike(40.0)?, &mm(&[1.0; 3]))?;
    assert_close(reach_at_apex(&cut), 1.0 / libm::sin(20.0_f64.to_radians()));
    Ok(())
}

/// Crossing-number test; `p` is never on the outline here.
fn inside(o: &Outline, p: DVec2) -> bool {
    let pts = o.points();
    let n = pts.len();
    (0..n)
        .filter(|&i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x)
        })
        .count()
        % 2
        == 1
}

#[test]
fn a_miter_that_would_cross_itself_is_beveled() -> Result<(), PatternError> {
    // At (0, 0) the outline turns 60° from a 10 mm allowance onto a 1 mm one. The offset lines
    // meet behind the wide strip's end, so a miter there would be a bowtie, and a bowtie would
    // cancel part of the wide strip.
    let corner = outline(&[(-50.0, 0.0), (0.0, 0.0), (25.0, 25.0 * 3.0_f64.sqrt())])?;
    let cut = cut_line(PIECE, &corner, &mm(&[10.0, 1.0, 1.0]))?;
    assert!(inside(&cut, DVec2::new(-2.0, -9.0)));
    assert!(inside(&cut, DVec2::new(0.3, -1.2)));
    Ok(())
}

#[test]
fn a_miter_that_would_cross_itself_is_beveled_when_the_wide_edge_comes_second()
-> Result<(), PatternError> {
    // The same corner mirrored: the 10 mm allowance is on the edge leaving (0, 0).
    let corner = outline(&[(-25.0, 25.0 * 3.0_f64.sqrt()), (0.0, 0.0), (50.0, 0.0)])?;
    let cut = cut_line(PIECE, &corner, &mm(&[1.0, 10.0, 1.0]))?;
    assert!(inside(&cut, DVec2::new(2.0, -9.0)));
    assert!(inside(&cut, DVec2::new(-0.3, -1.2)));
    Ok(())
}

#[test]
fn a_corner_beside_a_fold_is_joined_from_the_other_side() -> Result<(), PatternError> {
    // An equilateral triangle cut on a fold along its base: the 60° base corners still need the
    // wedge from their sloping edges, so the cut is the equilateral triangle h = 5√3 + 2 tall.
    let tri = outline(&[(0.0, 0.0), (10.0, 0.0), (5.0, 5.0 * 3.0_f64.sqrt())])?;
    let cut = cut_line(PIECE, &tri, &mm(&[0.0, 1.0, 1.0]))?;
    let h = 5.0 * 3.0_f64.sqrt() + 2.0;
    assert_close(area(&cut), h * h / 3.0_f64.sqrt());
    Ok(())
}

#[test]
fn the_miter_limit_is_set_by_the_wider_allowance() -> Result<(), PatternError> {
    // At (10, 0) the miter reaches √17 ≈ 4.12: past 4 × 1 but within 4 × 4, so it stays, and
    // the cut is the rectangle [-1, 14] × [-1, 11].
    let cut = cut_line(PIECE, &square()?, &mm(&[1.0, 4.0, 1.0, 1.0]))?;
    assert_close(area(&cut), 180.0);
    Ok(())
}

#[test]
fn a_miter_exactly_at_the_limit_is_kept() -> Result<(), PatternError> {
    // The corner at (0, 0) is chosen so its miter is exactly 4 × the 1 mm allowance in f64.
    let wedge = outline(&[(-50.0, 0.0), (0.0, 0.0), (-38.729_833_462_074_154, 10.0)])?;
    let cut = cut_line(PIECE, &wedge, &mm(&[0.0, 1.0, 1.0]))?;
    let right = cut
        .points()
        .iter()
        .map(|p| p.x)
        .fold(f64::NEG_INFINITY, f64::max);
    assert_close(right, 4.0);
    Ok(())
}
