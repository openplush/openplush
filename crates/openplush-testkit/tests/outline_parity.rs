//! The outline and seam-parity oracles must refuse every outline that cannot be
//! cut in one piece and every seam that cannot be sewn flat, and pass the rest.

use openplush_geom::units::Mm;
use openplush_geom::{DVec2, GeomError};
use openplush_testkit::oracles::{OracleFailure, outline_simple, seam_parity};

fn polygon<const N: usize>(points: [(f64, f64); N]) -> Vec<DVec2> {
    points.map(|(x, y)| DVec2::new(x, y)).to_vec()
}

fn reversed(mut points: Vec<DVec2>) -> Vec<DVec2> {
    points.reverse();
    points
}

fn not_simple(a: usize, b: usize) -> Result<(), OracleFailure> {
    Err(OracleFailure::OutlineNotSimple { a, b })
}

const SQUARE: [(f64, f64); 4] = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];

/// A U open at the top-middle, starting at a reflex vertex; its two bottom edges
/// lie on one line without meeting.
const U: [(f64, f64); 8] = [
    (1.0, 2.0),
    (2.0, 2.0),
    (2.0, 0.0),
    (3.0, 0.0),
    (3.0, 3.0),
    (0.0, 3.0),
    (0.0, 0.0),
    (1.0, 0.0),
];

#[test]
fn counter_clockwise_simple_outlines_pass_and_their_reverses_are_clockwise() {
    assert_eq!(outline_simple(&polygon(SQUARE)), Ok(()));
    assert_eq!(outline_simple(&polygon(U)), Ok(()));
    assert_eq!(
        outline_simple(&reversed(polygon(SQUARE))),
        Err(OracleFailure::Clockwise)
    );
    assert_eq!(
        outline_simple(&reversed(polygon(U))),
        Err(OracleFailure::Clockwise)
    );
}

#[test]
fn orientation_is_exact_far_from_the_origin() {
    let far = 1e16;
    let small = polygon([(far, far), (far + 4.0, far), (far, far + 4.0)]);
    assert_eq!(outline_simple(&small), Ok(()));
    assert_eq!(
        outline_simple(&reversed(small)),
        Err(OracleFailure::Clockwise)
    );
}

#[test]
fn crossing_edges_are_not_simple() {
    let bowtie = polygon([(0.0, 0.0), (1.0, 1.0), (1.0, 0.0), (0.0, 1.0)]);
    assert_eq!(outline_simple(&bowtie), not_simple(0, 2));
    // The last edge crosses edge 1, a pair the first-and-last adjacency skip must not hide.
    let hooked = polygon([(0.0, 0.0), (4.0, 0.0), (-1.0, 2.0), (-1.0, 4.0), (1.0, 3.0)]);
    assert_eq!(outline_simple(&hooked), not_simple(1, 4));
}

#[test]
fn an_outline_that_touches_itself_is_not_simple() {
    // A vertex of the notch lands exactly on the bottom edge: cuttable only as two pieces.
    let touching = polygon([(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (2.0, 0.0), (0.0, 4.0)]);
    assert_eq!(outline_simple(&touching), not_simple(0, 2));
}

#[test]
fn an_outline_that_doubles_back_is_not_simple() {
    let flat = polygon([(0.0, 0.0), (2.0, 0.0), (1.0, 0.0)]);
    assert_eq!(outline_simple(&flat), not_simple(0, 1));
    let spike = polygon([(0.0, 0.0), (2.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
    assert_eq!(outline_simple(&spike), not_simple(0, 1));
    // Edge 1 runs back past the start of edge 0.
    let overshoot = polygon([(1.0, 0.0), (2.0, 0.0), (0.0, 0.0), (0.0, 1.0)]);
    assert_eq!(outline_simple(&overshoot), not_simple(0, 1));
}

#[test]
fn a_zero_length_edge_or_too_few_points_is_not_simple() {
    let repeat = polygon([(0.0, 0.0), (1.0, 0.0), (1.0, 0.0), (0.0, 1.0)]);
    assert_eq!(outline_simple(&repeat), not_simple(1, 1));
    assert_eq!(outline_simple(&polygon(SQUARE)[..2]), not_simple(0, 0));
}

#[test]
fn a_non_finite_point_is_not_simple_and_is_named() {
    let nan = polygon([(0.0, 0.0), (1.0, 0.0), (f64::NAN, 0.5), (0.0, 1.0)]);
    assert_eq!(outline_simple(&nan), not_simple(2, 2));
}

#[test]
fn an_outline_the_exact_predicates_cannot_decide_is_not_simple() {
    // Coordinates spanning more than 2^150 make orient2d refuse; that must fail,
    // never read as "no crossing" or "not doubled back".
    let figure_eight = polygon([(0.0, 1.0), (2.0, 1e-60), (2.0, 2.0), (0.0, 0.0)]);
    assert!(matches!(
        outline_simple(&figure_eight),
        Err(OracleFailure::OutlineNotSimple { .. })
    ));
    // Undecidable at the first corner, which has no non-adjacent edge to check.
    let thin = polygon([(0.0, 0.0), (1.0, 0.0), (0.0, 1e-60)]);
    assert_eq!(outline_simple(&thin), not_simple(0, 1));
    // Every corner is decidable at the 1e-16 scale; only edge 1 against the far
    // edge 4 is not.
    let mixed = polygon([
        (0.0, 0.0),
        (1e-16, 0.0),
        (2e-16, 1e-60),
        (3e-16, 0.0),
        (4e-16, 0.0),
        (1.0, 1.0),
        (0.0, 1.0),
    ]);
    assert_eq!(outline_simple(&mixed), not_simple(1, 4));
}

#[test]
fn parity_allows_one_millimetre_on_short_seams() -> Result<(), GeomError> {
    assert_eq!(
        seam_parity(Mm::try_new(100.0)?, Mm::try_new(101.0)?),
        Ok(())
    );
    assert!(matches!(
        seam_parity(Mm::try_new(100.0)?, Mm::try_new(101.01)?),
        Err(OracleFailure::ParityExceeded { .. })
    ));
    Ok(())
}

#[test]
fn parity_allows_half_a_percent_of_the_longer_side_on_long_seams() -> Result<(), GeomError> {
    // 0.5% of 402.005 is 2.010025 mm; of 400 it would be only 2.0.
    let (short, long) = (Mm::try_new(400.0)?, Mm::try_new(402.005)?);
    assert_eq!(seam_parity(short, long), Ok(()));
    assert_eq!(seam_parity(long, short), Ok(()));
    assert!(matches!(
        seam_parity(short, Mm::try_new(402.03)?),
        Err(OracleFailure::ParityExceeded { .. })
    ));
    Ok(())
}
