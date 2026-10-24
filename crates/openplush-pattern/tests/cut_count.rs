//! `CutCount::new`: copies cut as printed plus copies cut flipped, 1 to 100 in total.

use openplush_pattern::{CutCount, PatternError};

#[test]
fn a_pair_is_one_as_printed_and_one_flipped() -> Result<(), PatternError> {
    let c = CutCount::new(1, 1)?;
    assert_eq!((c.as_printed(), c.flipped(), c.total()), (1, 1, 2));
    Ok(())
}

#[test]
fn either_count_may_be_zero_while_the_total_is_not() -> Result<(), PatternError> {
    assert_eq!(CutCount::new(4, 0)?.total(), 4);
    assert_eq!(CutCount::new(0, 1)?.total(), 1);
    Ok(())
}

#[test]
fn nothing_to_cut_is_refused() {
    assert_eq!(CutCount::new(0, 0), Err(PatternError::ZeroCutQuantity));
}

#[test]
fn a_hundred_in_total_is_the_most_allowed() {
    assert!(CutCount::new(100, 0).is_ok());
    assert!(CutCount::new(50, 50).is_ok());
    assert_eq!(
        CutCount::new(60, 41),
        Err(PatternError::TooManyCuts { total: 101 })
    );
}

#[test]
fn a_total_too_large_for_u32_is_reported_whole() {
    assert_eq!(
        CutCount::new(u32::MAX, u32::MAX),
        Err(PatternError::TooManyCuts {
            total: 2 * u64::from(u32::MAX)
        })
    );
}

#[test]
fn the_errors_say_what_is_wrong() {
    assert_eq!(
        PatternError::TooManyCuts { total: 101 }.to_string(),
        "a piece can be cut at most 100 times, got 101"
    );
    assert_eq!(
        PatternError::ZeroCutQuantity.to_string(),
        "a piece must be cut at least once"
    );
}

#[test]
fn each_count_reads_back_as_given() -> Result<(), PatternError> {
    for (as_printed, flipped) in [(2, 0), (0, 2), (3, 5), (5, 3)] {
        let c = CutCount::new(as_printed, flipped)?;
        assert_eq!((c.as_printed(), c.flipped()), (as_printed, flipped));
    }
    Ok(())
}
