//! `Fabric::new`: a fabric profile that obeys its invariants, and the values derived from it.

use openplush_geom::units::Mm;
use openplush_pattern::{
    Fabric, FabricCategory, FabricId, FabricSpec, PatternError, StretchDirection,
};

type Setup = Box<dyn std::error::Error>;

const ID: FabricId = FabricId::new(7);

fn mm(v: f64) -> Result<Mm, Setup> {
    Ok(Mm::try_new(v)?)
}

/// A plain non-napped woven every test starts from and changes one thing in.
fn woven() -> Result<FabricSpec, Setup> {
    Ok(FabricSpec {
        id: ID,
        name: "Quilting cotton".to_owned(),
        category: FabricCategory::Woven,
        stretch_grain_pct: 0.0,
        stretch_cross_pct: 2.0,
        stretch_bias_pct: 10.0,
        nap: false,
        pile_len: mm(0.0)?,
        thickness: mm(0.3)?,
        frays: true,
        default_allowance: mm(6.0)?,
        roll_width: mm(1100.0)?,
    })
}

fn long_fur() -> Result<FabricSpec, Setup> {
    Ok(FabricSpec {
        name: "Long faux fur".to_owned(),
        category: FabricCategory::Pile,
        nap: true,
        pile_len: mm(30.0)?,
        frays: false,
        ..woven()?
    })
}

#[test]
fn a_valid_fabric_reads_back_what_it_was_built_from() -> Result<(), Setup> {
    let f = Fabric::new(woven()?)?;
    assert_eq!(f.spec(), &woven()?);
    assert_eq!(f.id(), ID);
    Ok(())
}

#[test]
fn rotations_follow_nap_and_category() -> Result<(), Setup> {
    let felt = FabricSpec {
        category: FabricCategory::Felt,
        frays: false,
        ..woven()?
    };
    let knit = FabricSpec {
        category: FabricCategory::Knit,
        ..woven()?
    };
    let napped_knit = FabricSpec {
        nap: true,
        ..knit.clone()
    };
    assert_eq!(Fabric::new(woven()?)?.rotations_deg(), [0.0, 180.0]);
    assert_eq!(Fabric::new(knit)?.rotations_deg(), [0.0, 180.0]);
    assert_eq!(
        Fabric::new(felt)?.rotations_deg(),
        [0.0, 90.0, 180.0, 270.0]
    );
    assert_eq!(Fabric::new(napped_knit)?.rotations_deg(), [0.0]);
    assert_eq!(Fabric::new(long_fur()?)?.rotations_deg(), [0.0]);
    Ok(())
}

#[test]
fn nap_outranks_felt_having_no_grain() -> Result<(), Setup> {
    let napped_felt = FabricSpec {
        category: FabricCategory::Felt,
        nap: true,
        ..woven()?
    };
    assert_eq!(Fabric::new(napped_felt)?.rotations_deg(), [0.0]);
    Ok(())
}

#[test]
fn only_pile_fabrics_ask_about_mirrored_printing() -> Result<(), Setup> {
    assert!(Fabric::new(long_fur()?)?.asks_mirroring());
    let napped_knit = FabricSpec {
        category: FabricCategory::Knit,
        nap: true,
        ..woven()?
    };
    assert!(!Fabric::new(napped_knit)?.asks_mirroring());
    assert!(!Fabric::new(woven()?)?.asks_mirroring());
    Ok(())
}

#[test]
fn a_stretch_that_is_negative_or_not_finite_names_its_direction() -> Result<(), Setup> {
    for bad in [-0.5, f64::NAN, f64::INFINITY] {
        let cases = [
            (
                StretchDirection::Grain,
                FabricSpec {
                    stretch_grain_pct: bad,
                    ..woven()?
                },
            ),
            (
                StretchDirection::Cross,
                FabricSpec {
                    stretch_cross_pct: bad,
                    ..woven()?
                },
            ),
            (
                StretchDirection::Bias,
                FabricSpec {
                    stretch_bias_pct: bad,
                    ..woven()?
                },
            ),
        ];
        for (direction, spec) in cases {
            let got = Fabric::new(spec);
            assert!(
                matches!(got, Err(PatternError::BadStretch { fabric, direction: d, percent })
                    if fabric == ID && d == direction && percent.to_bits() == bad.to_bits()),
                "{direction:?} {bad}: {got:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_zero_roll_width_is_refused() -> Result<(), Setup> {
    let spec = FabricSpec {
        roll_width: mm(0.0)?,
        ..woven()?
    };
    assert_eq!(
        Fabric::new(spec),
        Err(PatternError::ZeroRollWidth { fabric: ID })
    );
    Ok(())
}

#[test]
fn zero_allowance_is_only_for_fabric_that_does_not_fray() -> Result<(), Setup> {
    let fraying = FabricSpec {
        default_allowance: mm(0.0)?,
        ..woven()?
    };
    assert_eq!(
        Fabric::new(fraying),
        Err(PatternError::ZeroAllowanceFrays { fabric: ID })
    );
    let felt = FabricSpec {
        category: FabricCategory::Felt,
        frays: false,
        default_allowance: mm(0.0)?,
        ..woven()?
    };
    assert!(Fabric::new(felt).is_ok());
    Ok(())
}

#[test]
fn a_pile_with_length_must_have_nap() -> Result<(), Setup> {
    let spec = FabricSpec {
        nap: false,
        ..long_fur()?
    };
    assert_eq!(
        Fabric::new(spec),
        Err(PatternError::PileWithoutNap { fabric: ID })
    );
    let flat_pile = FabricSpec {
        nap: false,
        pile_len: mm(0.0)?,
        ..long_fur()?
    };
    assert!(Fabric::new(flat_pile).is_ok());
    let brushed = FabricSpec {
        pile_len: mm(2.0)?,
        ..woven()?
    };
    assert!(
        Fabric::new(brushed).is_ok(),
        "the rule is for pile fabrics only"
    );
    Ok(())
}

#[test]
fn the_new_errors_say_what_is_wrong() {
    let cases = [
        (
            PatternError::BadStretch {
                fabric: ID,
                direction: StretchDirection::Cross,
                percent: -0.5,
            },
            "fabric 7: a cross stretch of -0.5% is not usable (must be finite and not negative)",
        ),
        (
            PatternError::ZeroRollWidth { fabric: ID },
            "fabric 7: the roll width must be greater than zero",
        ),
        (
            PatternError::ZeroAllowanceFrays { fabric: ID },
            "fabric 7: a fabric that frays needs a seam allowance greater than zero",
        ),
        (
            PatternError::PileWithoutNap { fabric: ID },
            "fabric 7: a pile fabric with a pile length has a nap",
        ),
    ];
    for (err, text) in cases {
        assert_eq!(err.to_string(), text);
    }
}
