//! `scale::size`: the scale that makes a model its finished size, and whether that size is in 75 mm to 1000 mm.

use openplush_geom::GeomError;
use openplush_geom::units::{Mm, Scale};
use openplush_pattern::PatternError;
use openplush_pattern::scale::{SizeRange, SizeTarget, size};

type Setup = Box<dyn std::error::Error>;

fn longest(mm: f64) -> Result<SizeTarget, GeomError> {
    Ok(SizeTarget::LongestDimension(Mm::try_new(mm)?))
}

fn reference(model_length: f64, mm: f64) -> Result<SizeTarget, GeomError> {
    Ok(SizeTarget::Reference {
        model_length,
        finished: Mm::try_new(mm)?,
    })
}

#[test]
fn the_longest_dimension_sets_the_scale() -> Result<(), Setup> {
    let (scale, range) = size(2.0, longest(300.0)?)?;
    assert_eq!(scale, Scale::mm_per_model_unit(150.0)?);
    assert_eq!(range, SizeRange::Within);
    Ok(())
}

#[test]
fn a_reference_distance_sets_the_scale_and_the_longest_side_is_judged() -> Result<(), Setup> {
    // 0.5 units measure 40 mm, so 80 mm per unit and a 160 mm longest side.
    let (scale, range) = size(2.0, reference(0.5, 40.0)?)?;
    assert_eq!(scale, Scale::mm_per_model_unit(80.0)?);
    assert_eq!(range, SizeRange::Within);
    // The same reference on a 0.9-unit model finishes at 72 mm.
    assert_eq!(size(0.9, reference(0.5, 40.0)?)?.1, SizeRange::Below);
    // And on a 13-unit model at 1040 mm.
    assert_eq!(size(13.0, reference(0.5, 40.0)?)?.1, SizeRange::Above);
    Ok(())
}

#[test]
fn both_ends_of_the_range_are_within() -> Result<(), Setup> {
    // 3 units does not divide 75 or 1000 exactly, so a judgement on the rounded product would wobble.
    for (mm, range) in [
        (74.999, SizeRange::Below),
        (75.0, SizeRange::Within),
        (1000.0, SizeRange::Within),
        (1000.001, SizeRange::Above),
    ] {
        assert_eq!(size(3.0, longest(mm)?)?.1, range, "{mm} mm");
    }
    Ok(())
}

/// Whether `got` refuses exactly `bad`, compared bit for bit so a NaN matches itself.
fn refuses(got: Result<(Scale, SizeRange), PatternError>, bad: f64) -> bool {
    matches!(got, Err(PatternError::BadModelLength { model_length }) if model_length.to_bits() == bad.to_bits())
}

#[test]
fn a_model_length_that_is_not_positive_and_finite_is_refused() -> Result<(), Setup> {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(refuses(size(bad, longest(300.0)?), bad), "longest {bad}");
        assert!(
            refuses(size(2.0, reference(bad, 40.0)?), bad),
            "reference {bad}"
        );
    }
    Ok(())
}

#[test]
fn a_zero_finished_size_is_refused() -> Result<(), Setup> {
    assert!(matches!(
        size(2.0, longest(0.0)?),
        Err(PatternError::BadScale {
            source: GeomError::InvalidScale { .. }
        })
    ));
    Ok(())
}

#[test]
fn a_finished_size_that_overflows_is_refused() -> Result<(), Setup> {
    // 1e303 mm per unit is a valid scale, but the 1e300-unit model would finish at infinity.
    assert!(matches!(
        size(1e300, reference(1e-300, 1000.0)?),
        Err(PatternError::BadScale { .. })
    ));
    Ok(())
}

#[test]
fn the_new_errors_say_what_is_wrong() {
    assert_eq!(
        PatternError::BadModelLength { model_length: -1.0 }.to_string(),
        "a model length of -1 units is not usable (must be finite and positive)"
    );
    assert_eq!(
        PatternError::BadScale {
            source: GeomError::InvalidScale { mm_per_unit: 0.0 }
        }
        .to_string(),
        "the finished size cannot be reached: scale of 0 mm per model unit is not usable (must be finite and positive)"
    );
}
