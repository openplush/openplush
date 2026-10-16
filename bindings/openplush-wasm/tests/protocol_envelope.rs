//! The JSON shape of the request and response envelope.

use openplush_pipeline::EngineError;
use openplush_wasm::protocol::{EngineErrorDto, Op, Progress, Request, RequestId, Response};
use serde_json::{Value, json};

fn request(wire: Value) -> Result<Request, serde_json::Error> {
    serde_json::from_value(wire)
}

fn id(n: u32) -> Result<RequestId, serde_json::Error> {
    serde_json::from_value(json!(n))
}

#[test]
fn cancel_request_reads_its_id_and_target() -> Result<(), serde_json::Error> {
    let req = request(json!({ "id": 4, "op": "cancel", "payload": { "target": 3 } }))?;
    assert_eq!(req.id(), id(4)?);
    assert_eq!(req.op(), &Op::Cancel { target: id(3)? });
    Ok(())
}

#[test]
fn the_largest_u32_is_accepted() -> Result<(), serde_json::Error> {
    let max = u32::MAX;
    let req = request(json!({ "id": max, "op": "cancel", "payload": { "target": max } }))?;
    assert_eq!(req.id(), id(max)?);
    assert_eq!(req.op(), &Op::Cancel { target: id(max)? });
    Ok(())
}

#[test]
fn malformed_requests_are_refused() {
    let refused = [
        json!({ "op": "cancel", "payload": { "target": 3 } }),
        json!({ "id": 4, "op": "flatten", "payload": {} }),
        json!({ "id": 4, "op": "cancel" }),
        json!({ "id": 4, "op": "cancel", "payload": {} }),
        json!({ "id": 4_294_967_296_u64, "op": "cancel", "payload": { "target": 3 } }),
        json!({ "id": -1, "op": "cancel", "payload": { "target": 3 } }),
        json!({ "id": 1.5, "op": "cancel", "payload": { "target": 3 } }),
        json!({ "id": 4, "op": "cancel", "payload": { "target": 4_294_967_296_u64 } }),
        json!({ "id": 4, "op": "cancel", "payload": { "target": 3, "targets": [3] } }),
    ];
    for wire in refused {
        assert!(request(wire.clone()).is_err(), "accepted {wire}");
    }
}

#[test]
fn success_carries_its_result_and_no_warnings() -> Result<(), serde_json::Error> {
    let response = Response::success(id(4)?, json!({ "charts": [1, 2] }));
    assert_eq!(
        serde_json::to_value(response)?,
        json!({ "id": 4, "ok": true, "result": { "charts": [1, 2] }, "warnings": [] })
    );
    Ok(())
}

#[test]
fn failure_carries_its_error() -> Result<(), serde_json::Error> {
    let error = EngineErrorDto::from(&EngineError::Cancelled);
    let response: Response<()> = Response::failure(id(4)?, error);
    assert_eq!(
        serde_json::to_value(response)?,
        json!({
            "id": 4,
            "ok": false,
            "error": { "code": "Cancelled", "message": EngineError::Cancelled.to_string() }
        })
    );
    Ok(())
}

#[test]
fn progress_carries_no_ok_key() -> Result<(), serde_json::Error> {
    let response: Response<()> = Response::progress(id(4)?, Progress::Done);
    assert_eq!(
        serde_json::to_value(response)?,
        json!({ "id": 4, "progress": { "kind": "done" } })
    );
    Ok(())
}
