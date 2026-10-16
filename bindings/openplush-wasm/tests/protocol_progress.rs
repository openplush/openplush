//! The JSON shape of progress and handles, and the refusal of non-finite progress.

use openplush_pipeline::ChartId;
use openplush_wasm::protocol::{MeshHandle, PipelineHandle, Progress, ProtocolError, Working};
use serde_json::{Value, json};

fn working(max_distortion: f64) -> Result<Working, ProtocolError> {
    Working::new(ChartId::new(3), 12, max_distortion)
}

#[test]
fn working_serializes_with_its_kind_and_camel_case_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let progress = Progress::Working(working(1.25)?);
    assert_eq!(
        serde_json::to_value(progress)?,
        json!({ "kind": "working", "chart": 3, "iteration": 12, "maxDistortion": 1.25 })
    );
    Ok(())
}

#[test]
fn done_serializes_as_its_kind_alone() -> Result<(), serde_json::Error> {
    assert_eq!(
        serde_json::to_value(Progress::Done)?,
        json!({ "kind": "done" })
    );
    Ok(())
}

#[test]
fn non_finite_distortion_is_refused() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let refused = working(value);
        assert!(
            matches!(refused, Err(ProtocolError::NonFiniteMaxDistortion { chart, value: v })
                if chart == ChartId::new(3) && v.to_bits() == value.to_bits()),
            "{value} gave {refused:?}"
        );
    }
}

#[test]
fn non_finite_error_names_the_chart_and_field() {
    let message = ProtocolError::NonFiniteMaxDistortion {
        chart: ChartId::new(3),
        value: f64::NAN,
    }
    .to_string();
    assert_eq!(
        message,
        "chart 3: progress maxDistortion is NaN, not a finite number"
    );
}

#[test]
fn extreme_finite_distortion_is_kept_exactly() -> Result<(), Box<dyn std::error::Error>> {
    for value in [0.0, -0.0, f64::MIN_POSITIVE, f64::MAX] {
        let wire = serde_json::to_value(Progress::Working(working(value)?))?;
        assert_eq!(
            wire["maxDistortion"].as_f64().map(f64::to_bits),
            Some(value.to_bits())
        );
    }
    Ok(())
}

#[test]
fn handles_serialize_as_their_number() -> Result<(), serde_json::Error> {
    let expected: Value = json!({ "n": 7 });
    assert_eq!(serde_json::to_value(MeshHandle::new(7))?, expected);
    assert_eq!(serde_json::to_value(PipelineHandle::new(7))?, expected);
    assert_eq!(
        serde_json::to_value(MeshHandle::new(u32::MAX))?,
        json!({ "n": 4_294_967_295_u32 })
    );
    Ok(())
}
