//! `Staged` and `StageResult`: valid output travels with its warnings; a
//! violated invariant carries no output at all.

use openplush_pipeline::{EngineError, StageResult, Staged, WarningSet};

fn stage(succeed: bool) -> StageResult<u8> {
    if succeed {
        Ok(Staged {
            value: 7,
            warnings: WarningSet::default(),
        })
    } else {
        Err(EngineError::Cancelled)
    }
}

#[test]
fn ok_carries_the_value_and_its_warnings() {
    let staged = stage(true).expect("the stage succeeds");
    assert_eq!(staged.value, 7);
    assert_eq!(staged.warnings, WarningSet::default());
}

#[test]
fn err_carries_no_output() {
    assert_eq!(stage(false), Err(EngineError::Cancelled));
}
