//! Notches: the constructor's refusals, the even spacing of `fractions`, the neighbour rule of
//! `mark_counts`, and `Outline::outward_at` at convex and reflex corners.

use std::collections::{BTreeMap, BTreeSet};

use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_pattern::notch::{SPACING_MM, fractions, mark_counts};
use openplush_pattern::{Notch, Outline, PatternError};
use openplush_seams::SeamChainId;

type Setup = Box<dyn std::error::Error>;

const CHAIN: SeamChainId = SeamChainId::new(7);

#[test]
fn a_notch_keeps_its_point_and_scales_its_direction_to_unit_length() -> Result<(), Setup> {
    let n = Notch::new(DVec2::new(1.0, 2.0), DVec2::new(0.0, -3.0), CHAIN, 2)?;
    assert_eq!(n.at(), DVec2::new(1.0, 2.0));
    assert_eq!(n.outward(), DVec2::new(0.0, -1.0));
    assert_eq!((n.chain(), n.marks()), (CHAIN, 2));
    Ok(())
}

#[test]
fn a_notch_with_no_marks_no_direction_or_a_point_off_the_plane_is_refused() {
    let bad = PatternError::BadNotch { chain: CHAIN };
    let (p, d) = (DVec2::new(0.0, 0.0), DVec2::new(1.0, 0.0));
    assert_eq!(Notch::new(p, d, CHAIN, 0), Err(bad.clone()));
    assert_eq!(
        Notch::new(p, DVec2::new(0.0, 0.0), CHAIN, 1),
        Err(bad.clone())
    );
    assert_eq!(
        Notch::new(p, DVec2::new(f64::NAN, 1.0), CHAIN, 1),
        Err(bad.clone())
    );
    assert_eq!(
        Notch::new(DVec2::new(f64::INFINITY, 0.0), d, CHAIN, 1),
        Err(bad)
    );
}

#[test]
fn a_side_is_split_evenly_into_gaps_of_at_most_50_mm() -> Result<(), Setup> {
    assert_eq!(
        fractions(CHAIN, Mm::try_new(120.0)?)?,
        vec![1.0 / 3.0, 2.0 / 3.0]
    );
    assert_eq!(fractions(CHAIN, Mm::try_new(100.0)?)?, vec![0.5]);
    assert_eq!(fractions(CHAIN, Mm::try_new(100.5)?)?.len(), 2);
    assert_eq!(SPACING_MM, 50.0);
    Ok(())
}

#[test]
fn a_short_side_or_one_of_zero_length_still_gets_its_midpoint() -> Result<(), Setup> {
    assert_eq!(fractions(CHAIN, Mm::try_new(3.0)?)?, vec![0.5]);
    assert_eq!(fractions(CHAIN, Mm::try_new(0.0)?)?, vec![0.5]);
    Ok(())
}

#[test]
fn even_spacing_reads_the_same_from_either_end() -> Result<(), Setup> {
    let f = fractions(CHAIN, Mm::try_new(437.0)?)?;
    let mut back: Vec<f64> = f.iter().map(|t| 1.0 - t).collect();
    back.reverse();
    for (a, b) in f.iter().zip(&back) {
        assert!((a - b).abs() <= 1e-12);
    }
    Ok(())
}

#[test]
fn a_seam_needing_over_1000_notches_is_refused() -> Result<(), Setup> {
    assert_eq!(fractions(CHAIN, Mm::try_new(50_050.0)?)?.len(), 1000);
    let length = Mm::try_new(50_051.0)?;
    assert_eq!(
        fractions(CHAIN, length),
        Err(PatternError::TooManyNotches {
            chain: CHAIN,
            length
        })
    );
    Ok(())
}

fn meets(pairs: &[(u32, u32)], chains: u32) -> BTreeMap<SeamChainId, BTreeSet<SeamChainId>> {
    let mut m: BTreeMap<_, BTreeSet<_>> = (0..chains)
        .map(|c| (SeamChainId::new(c), BTreeSet::new()))
        .collect();
    for &(a, b) in pairs {
        let (a, b) = (SeamChainId::new(a), SeamChainId::new(b));
        m.entry(a).or_default().insert(b);
        m.entry(b).or_default().insert(a);
    }
    m
}

#[test]
fn seams_that_meet_never_share_a_mark_count() {
    let m = meets(&[(0, 1), (1, 2), (2, 0), (2, 3), (3, 4)], 6);
    let counts = mark_counts(&m);
    assert_eq!(counts.len(), 6);
    for (a, others) in &m {
        for b in others {
            assert_ne!(counts[a], counts[b], "{a:?} and {b:?}");
        }
    }
}

#[test]
fn each_seam_takes_the_least_count_its_lower_neighbours_left() {
    let counts = mark_counts(&meets(&[(0, 1), (1, 2), (2, 0), (2, 3)], 5));
    let got: Vec<u32> = counts.values().copied().collect();
    assert_eq!(got, vec![1, 2, 3, 1, 1]);
}

#[test]
fn outward_at_a_square_corner_points_out_along_its_diagonal() -> Result<(), Setup> {
    let square = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(1.0, 0.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(0.0, 1.0),
    ])?;
    let d = square.outward_at(0).ok_or("a direction")?;
    let h = std::f64::consts::FRAC_1_SQRT_2;
    assert!((d - DVec2::new(-h, -h)).length() <= 1e-12, "{d:?}");
    assert_eq!(square.outward_at(4), None);
    Ok(())
}

#[test]
fn outward_at_a_reflex_corner_points_out_of_the_notch() -> Result<(), Setup> {
    // An L: the corner at (1, 1) is reflex, its outside towards (2, 2).
    let l = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(2.0, 0.0),
        DVec2::new(2.0, 1.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(1.0, 2.0),
        DVec2::new(0.0, 2.0),
    ])?;
    let d = l.outward_at(3).ok_or("a direction")?;
    let h = std::f64::consts::FRAC_1_SQRT_2;
    assert!((d - DVec2::new(h, h)).length() <= 1e-12, "{d:?}");
    Ok(())
}
