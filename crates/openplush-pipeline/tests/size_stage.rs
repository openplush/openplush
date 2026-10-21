//! The size stage: the scale for the maker's finished size, judged on the longest side of the
//! repaired mesh's bounding box, with `size-out-of-range` outside 75 mm to 1000 mm.

use openplush_mesh::{RepairPlan, into_manifold};
use openplush_pipeline::{
    EngineError, ErrorLocations, ManifoldMesh, Mm, PatternError, Remedy, Scale, SizeTarget,
    StageResult, Warning, WarningTarget, size,
};

type Setup = Box<dyn std::error::Error>;

/// A tetrahedron 1 × 0.5 × 2 units: its longest side is 2 along z, its diagonal about 2.29.
fn tetra() -> Result<ManifoldMesh, Setup> {
    let text = "v 0 0 0\nv 1 0 0\nv 0 0.5 0\nv 0 0 2\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 1 4 3\n";
    let soup = openplush_io::obj::read(text.as_bytes())?;
    Ok(into_manifold(&soup, &RepairPlan::default())?.0)
}

fn longest(mm: f64) -> Result<SizeTarget, Setup> {
    Ok(SizeTarget::LongestDimension(Mm::try_new(mm)?))
}

fn reference(model_length: f64, mm: f64) -> Result<SizeTarget, Setup> {
    Ok(SizeTarget::Reference {
        model_length,
        finished: Mm::try_new(mm)?,
    })
}

fn warnings(mesh: &ManifoldMesh, target: SizeTarget) -> Result<Vec<Warning>, Setup> {
    Ok(size(mesh, target)?.warnings.iter().cloned().collect())
}

#[test]
fn the_longest_dimension_finishes_at_the_target() -> Result<(), Setup> {
    let scale = size(&tetra()?, longest(200.0)?)?.value;
    assert_eq!(scale, Scale::mm_per_model_unit(100.0)?);
    Ok(())
}

#[test]
fn the_range_includes_both_ends() -> Result<(), Setup> {
    let mesh = tetra()?;
    for (mm, warned) in [(74.9, true), (75.0, false), (1000.0, false), (1000.1, true)] {
        assert_eq!(
            !warnings(&mesh, longest(mm)?)?.is_empty(),
            warned,
            "{mm} mm"
        );
    }
    Ok(())
}

#[test]
fn out_of_range_is_one_advisory_warning_on_the_project() -> Result<(), Setup> {
    let found = warnings(&tetra()?, longest(74.9)?)?;
    let [w] = found.as_slice() else {
        return Err(format!("one warning expected, got {found:?}").into());
    };
    assert_eq!(w.code().as_str(), "size-out-of-range");
    assert_eq!(w.target(), WarningTarget::Project);
    assert_eq!(
        w.explanation(),
        "The pattern's longest dimension finishes at 74.9 mm, outside the supported 75 mm to \
         1000 mm."
    );
    let Remedy::Advisory { what_to_change } = w.remedy() else {
        return Err("an advisory remedy expected".into());
    };
    assert_eq!(
        what_to_change.as_str(),
        "Resize the model so its longest dimension is between 75 mm and 1000 mm, or keep this \
         size: the pattern is still made, but untested at this size."
    );
    Ok(())
}

#[test]
fn the_longest_dimension_is_the_bounding_boxs_longest_side() -> Result<(), Setup> {
    let mesh = tetra()?;
    // 37.4 mm per unit makes the z side 74.8 mm (warned); the diagonal would be 85.6 mm.
    assert_eq!(warnings(&mesh, reference(1.0, 37.4)?)?.len(), 1);
    // 500.1 mm per unit makes the z side 1000.2 mm (warned); x would be 500.1 mm, y 250.05 mm.
    assert_eq!(warnings(&mesh, reference(1.0, 500.1)?)?.len(), 1);
    assert!(warnings(&mesh, reference(1.0, 37.5)?)?.is_empty());
    Ok(())
}

#[test]
fn a_zero_reference_length_is_pattern_locating_nothing() -> Result<(), Setup> {
    let result: StageResult<Scale> = size(&tetra()?, reference(0.0, 100.0)?);
    let Err(err) = result else {
        return Err("a zero reference length must be refused".into());
    };
    assert!(matches!(
        err,
        EngineError::Pattern {
            piece: None,
            source: PatternError::BadModelLength { model_length },
        } if model_length == 0.0
    ));
    assert_eq!(err.code(), "Pattern");
    assert_eq!(err.locations(), ErrorLocations::default());
    Ok(())
}
