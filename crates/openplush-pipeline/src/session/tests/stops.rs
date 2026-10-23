//! Why each chart's flatten stopped is kept beside its layout, so a chart laid from the memo
//! warns as one just flattened would. No public knob caps a pipeline's flatten, so the stop at
//! the cap is filed in the memo by hand.

use openplush_flatten::{FlattenSession, Progress, StoppedBy};
use openplush_pattern::PieceId;

use super::super::memo::{Memo, key};
use super::{Setup, charts, halves};
use crate::{Pipeline, WarningSet, WarningTarget};

/// Each chart's layout from a first run, filed again with the stop `stops` gives its index.
fn refile(pipeline: &mut Pipeline, stops: [StoppedBy; 2]) -> Result<(), Setup> {
    let reference = pipeline.parameterizations()?.value;
    pipeline.memo = Memo::default();
    for ((chart, layout), stopped) in charts(pipeline).into_iter().zip(&reference).zip(stops) {
        let uv = layout.uv().to_vec();
        pipeline.memo.insert(key(&chart), chart, uv, stopped);
    }
    Ok(())
}

fn codes_and_targets(warnings: &WarningSet) -> Vec<(&'static str, WarningTarget)> {
    warnings
        .iter()
        .map(|w| (w.code().as_str(), w.target()))
        .collect()
}

#[test]
fn a_chart_laid_from_the_memo_at_the_cap_warns_on_its_piece_every_time() -> Result<(), Setup> {
    let (mut pipeline, _) = halves()?;
    refile(&mut pipeline, [StoppedBy::Settled, StoppedBy::IterationCap])?;
    let capped = charts(&pipeline)[1].chart();
    let expected = vec![(
        "flatten-iteration-cap",
        WarningTarget::Piece(PieceId::new(capped.index())),
    )];
    for _ in 0..2 {
        let staged = pipeline.parameterizations()?;
        assert_eq!(codes_and_targets(&staged.warnings), expected);
    }
    Ok(())
}

#[test]
fn charts_that_settle_or_find_no_descent_raise_nothing() -> Result<(), Setup> {
    let (mut pipeline, _) = halves()?;
    assert_eq!(
        pipeline.parameterizations()?.warnings,
        WarningSet::default()
    );
    refile(&mut pipeline, [StoppedBy::NoDescent, StoppedBy::Settled])?;
    assert_eq!(
        pipeline.parameterizations()?.warnings,
        WarningSet::default()
    );
    Ok(())
}

#[test]
fn the_memo_keeps_each_charts_stop_as_its_session_reported_it() -> Result<(), Setup> {
    let (mut pipeline, _) = halves()?;
    pipeline.parameterizations()?;
    for chart in charts(&pipeline) {
        let mut session = FlattenSession::new(chart.chart(), chart.xyz(), chart.faces().to_vec())
            .expect("session");
        while session.step().expect("step") != Progress::Done {}
        let reported = session.finish().map(|(_, stopped)| stopped);
        let kept = pipeline.memo.get(key(&chart), &chart).map(|(_, s)| s);
        assert_eq!(kept, reported, "chart {}", chart.chart().index());
    }
    Ok(())
}
