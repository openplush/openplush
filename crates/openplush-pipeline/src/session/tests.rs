//! What the session's public steps cannot show: the memo trusts a key only with the chart it
//! was stored for, holds one entry per current chart, and a kept refusal is never recomputed.
#![allow(clippy::expect_used, clippy::panic, reason = "test setup")]

#[path = "../../tests/common/grid.rs"]
#[allow(
    dead_code,
    reason = "shared with the integration tests, which use the rest"
)]
mod grid;
mod stops;

use std::collections::BTreeMap;

use openplush_flatten::StoppedBy;
use openplush_seams::{ChartMesh, Proposal, ProposalKind, SeamKind, SeamSet};

use self::grid::{
    Setup, bumpy, column_cut, edge_between, mesh_of, seam, tetra_cut, tetrahedron_then_grid,
    vertex_at,
};
use super::memo::{Memo, key};
use super::{Pipeline, Stepping, fresh};
use crate::{
    ChartId, EdgeId, EngineError, FlattenError, NonBlank, Parameterization, Remedy, Warning,
    WarningCode, WarningTarget,
};

fn halves() -> Result<(Pipeline, SeamSet), Setup> {
    let mesh = mesh_of(&bumpy(4, 2, 3.0))?;
    let mut seams = SeamSet::new();
    column_cut(&mesh, &mut seams, 2, 2)?;
    let mut pipeline = Pipeline::new(mesh);
    pipeline.set_seams(&seams);
    Ok((pipeline, seams))
}

fn charts(pipeline: &Pipeline) -> Vec<ChartMesh> {
    let staged = pipeline.charts().as_ref().expect("charts");
    staged.value.0.clone()
}

fn bits(layouts: &[Parameterization]) -> Vec<(ChartId, Vec<[u64; 2]>)> {
    layouts
        .iter()
        .map(|l| {
            let uv = l.uv().iter().map(|p| [p.x.to_bits(), p.y.to_bits()]);
            (l.chart(), uv.collect())
        })
        .collect()
}

/// Chart 0's layout filed under chart 1's key, as a collision of the two keys would file it.
fn collide(pipeline: &mut Pipeline, layouts: &[Parameterization]) {
    let [left, right] = &charts(pipeline)[..] else {
        panic!("two charts");
    };
    let uv = layouts[0].uv().to_vec();
    pipeline.memo = Memo::default();
    let settled = StoppedBy::Settled;
    pipeline
        .memo
        .insert(key(left), left.clone(), uv.clone(), settled);
    pipeline.memo.insert(key(right), left.clone(), uv, settled);
}

#[test]
fn a_layout_filed_under_a_charts_key_for_other_content_is_not_reused() -> Result<(), Setup> {
    let (mut pipeline, _) = halves()?;
    let reference = pipeline.parameterizations()?.value;
    collide(&mut pipeline, &reference);
    assert_eq!(bits(&pipeline.parameterizations()?.value), bits(&reference));
    Ok(())
}

#[test]
fn a_seam_change_carries_over_only_entries_whose_content_matches() -> Result<(), Setup> {
    let (mut pipeline, seams) = halves()?;
    let reference = pipeline.parameterizations()?.value;
    collide(&mut pipeline, &reference);
    assert_eq!(pipeline.memo.len(), 2);
    pipeline.set_seams(&seams);
    assert_eq!(pipeline.memo.len(), 1, "only chart 0's own entry is kept");
    Ok(())
}

#[test]
fn a_seam_change_keeps_a_charts_own_entry_and_drops_others_filed_beside_it() -> Result<(), Setup> {
    let (mut pipeline, seams) = halves()?;
    let reference = pipeline.parameterizations()?.value;
    let [left, right] = &charts(&pipeline)[..] else {
        panic!("two charts");
    };
    let (left, right) = (left.clone(), right.clone());
    pipeline.memo = Memo::default();
    pipeline.memo.insert(
        key(&right),
        left.clone(),
        reference[0].uv().to_vec(),
        StoppedBy::Settled,
    );
    pipeline.memo.insert(
        key(&right),
        right,
        reference[1].uv().to_vec(),
        StoppedBy::Settled,
    );
    pipeline.memo.insert(
        key(&left),
        left,
        reference[0].uv().to_vec(),
        StoppedBy::Settled,
    );
    pipeline.set_seams(&seams);
    assert_eq!(
        pipeline.memo.len(),
        2,
        "each chart's own entry, and no other"
    );
    Ok(())
}

#[test]
fn a_chart_is_fresh_when_only_its_key_matches_an_old_charts() -> Result<(), Setup> {
    let (pipeline, _) = halves()?;
    let [left, right] = &charts(&pipeline)[..] else {
        panic!("two charts");
    };
    let collided = key(right);
    let named = fresh(
        std::slice::from_ref(left),
        &[collided],
        std::slice::from_ref(right),
        &[collided],
    );
    assert_eq!(named, vec![right.chart()]);
    Ok(())
}

#[test]
fn entries_for_charts_the_seams_no_longer_make_are_dropped() -> Result<(), Setup> {
    let (mut pipeline, seams) = halves()?;
    pipeline.parameterizations()?;
    assert_eq!(pipeline.memo.len(), 2);
    let mesh = pipeline.mesh.clone();
    let mut dart = seams.clone();
    seam(&mesh, &mut dart, SeamKind::Dart, &[(3, 0), (3, 1)])?;
    pipeline.set_seams(&dart);
    assert_eq!(pipeline.memo.len(), 1, "the right half is gone");
    pipeline.set_seams(&SeamSet::new());
    assert_eq!(pipeline.memo.len(), 0);
    Ok(())
}

#[test]
fn a_kept_refusal_is_returned_as_kept_and_cleared_by_the_next_seam_set() -> Result<(), Setup> {
    let mesh = tetrahedron_then_grid()?;
    let seams = tetra_cut(&mesh, &[[100., 0., 0.], [110., 0., 0.]])?;
    let mut pipeline = Pipeline::new(mesh);
    pipeline.set_seams(&seams);
    let refusal = EngineError::Flatten(FlattenError::NotADisk {
        chart: ChartId::new(0),
    });
    assert_eq!(pipeline.flatten_step(), Err(refusal.clone()));
    assert!(matches!(&pipeline.stepping, Stepping::Refused(e) if *e == refusal));
    pipeline.stepping = Stepping::Refused(EngineError::Cancelled);
    assert_eq!(pipeline.flatten_step(), Err(EngineError::Cancelled));
    assert_eq!(
        pipeline.parameterizations().err(),
        Some(EngineError::Cancelled)
    );
    assert_eq!(pipeline.memo.len(), 0, "no other chart was laid out");
    pipeline.set_seams(&seams);
    assert_eq!(pipeline.flatten_step(), Err(refusal));
    Ok(())
}

#[test]
fn a_seam_change_drops_the_session_in_progress() -> Result<(), Setup> {
    let (mut pipeline, seams) = halves()?;
    pipeline.flatten_step()?;
    assert!(matches!(pipeline.stepping, Stepping::Flattening { .. }));
    pipeline.set_seams(&seams);
    assert!(matches!(pipeline.stepping, Stepping::Idle));
    Ok(())
}

/// 64-bit FNV-1a over `bytes`, written out independently of the session's own.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// A chart's key bytes, written out independently of the session's own.
fn key_bytes(chart: &ChartMesh) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (face, corners) in chart.faces() {
        bytes.extend(face.index().to_le_bytes());
        for v in corners {
            bytes.extend(v.index().to_le_bytes());
        }
    }
    for p in chart.xyz() {
        for c in [p.x, p.y, p.z] {
            bytes.extend(c.to_bits().to_le_bytes());
        }
    }
    bytes
}

#[test]
fn the_key_is_fnv1a_over_face_ids_and_corners_then_positions() -> Result<(), Setup> {
    let mesh = mesh_of("v 0 0 0\nv 3 0 0\nv 0 4 0.5\nf 1 2 3\n")?;
    let pipeline = Pipeline::new(mesh);
    let [one] = &charts(&pipeline)[..] else {
        panic!("one chart");
    };
    assert_eq!(key_bytes(one).len(), 16 + 3 * 24);
    assert_eq!(key(one), fnv1a(&key_bytes(one)));
    Ok(())
}

#[test]
fn the_key_writes_face_ids_little_endian() -> Result<(), Setup> {
    let (pipeline, _) = halves()?;
    let charts = charts(&pipeline);
    // The right half's faces have IDs above 0, whose bytes differ between byte orders.
    let right = charts
        .iter()
        .find(|c| c.chart().index() > 0)
        .expect("right half");
    assert_eq!(key(right), fnv1a(&key_bytes(right)));
    Ok(())
}

#[test]
fn working_reports_the_flatten_sessions_own_iteration_and_energy() -> Result<(), Setup> {
    let (mut pipeline, _) = halves()?;
    let first = charts(&pipeline).swap_remove(0);
    let mut bare =
        openplush_flatten::FlattenSession::new(first.chart(), first.xyz(), first.faces().to_vec())
            .expect("session");
    loop {
        let bare_step = bare.step().expect("bare step");
        let step = pipeline.flatten_step().expect("step");
        match (bare_step, step) {
            (
                openplush_flatten::Progress::Working { iteration, energy },
                super::Progress::Working {
                    chart,
                    iteration: i,
                    energy: e,
                },
            ) => {
                assert_eq!(chart, first.chart());
                assert_eq!((i, e.to_bits()), (iteration, energy.to_bits()));
            }
            (openplush_flatten::Progress::Done, super::Progress::ChartDone { chart }) => {
                assert_eq!(chart, first.chart());
                return Ok(());
            }
            other => panic!("the pipeline and the bare session disagree: {other:?}"),
        }
    }
}

/// The cut's bottom edge, a `Cut` seam, and the edge beside it on column 1, which is no seam.
fn on_cut_and_beside(pipeline: &Pipeline) -> Result<(EdgeId, EdgeId), Setup> {
    let at = |c, r| vertex_at(&pipeline.mesh, c, r);
    Ok((
        edge_between(&pipeline.mesh, at(2, 0)?, at(2, 1)?)?,
        edge_between(&pipeline.mesh, at(1, 0)?, at(1, 1)?)?,
    ))
}

/// A dart for chart 0 over the cut's bottom edge and the edge beside it.
fn dart_over_the_cut(pipeline: &Pipeline) -> Result<Proposal, Setup> {
    let (on_cut, beside) = on_cut_and_beside(pipeline)?;
    let add = BTreeMap::from([(on_cut, SeamKind::Dart), (beside, SeamKind::Dart)]);
    let kind = ProposalKind::Dart {
        chart: ChartId::new(0),
    };
    Ok(Proposal::new(kind, add, "a dart".to_owned())?)
}

#[test]
fn a_kept_refusals_darts_are_offered_once_each() -> Result<(), Setup> {
    let (mut pipeline, _) = halves()?;
    let dart = dart_over_the_cut(&pipeline)?;
    pipeline.stepping = Stepping::Refused(EngineError::BoundaryNotRealizable {
        chart: ChartId::new(0),
        residual: 0.5,
        proposals: vec![dart.clone(), dart.clone()],
    });
    assert_eq!(pipeline.proposals(), vec![dart]);
    Ok(())
}

#[test]
fn a_preview_adds_only_edges_with_no_seam_and_keeps_each_seams_kind() -> Result<(), Setup> {
    let (mut pipeline, seams) = halves()?;
    let dart = dart_over_the_cut(&pipeline)?;
    pipeline.stepping = Stepping::Refused(EngineError::BoundaryNotRealizable {
        chart: ChartId::new(0),
        residual: 0.5,
        proposals: vec![dart.clone()],
    });
    let (on_cut, beside) = on_cut_and_beside(&pipeline)?;
    let mut expected = seams;
    assert_eq!(expected.get(on_cut), Some(SeamKind::Cut));
    expected.insert(beside, SeamKind::Dart)?;
    assert_eq!(pipeline.preview(dart.id())?, expected);
    Ok(())
}

#[test]
fn an_actionable_warning_remedy_is_offered_and_an_advisory_is_not() -> Result<(), Setup> {
    let (mut pipeline, _) = halves()?;
    let dart = dart_over_the_cut(&pipeline)?;
    let warning = |remedy| {
        let text = NonBlank::new("why".to_owned()).expect("not blank");
        Warning::new(WarningCode::TestAlpha, WarningTarget::Project, text, remedy)
    };
    let staged = pipeline.charts.as_mut().expect("charts");
    staged.warnings.insert(warning(Remedy::Actionable {
        proposal: dart.clone(),
    }));
    staged.warnings.insert(warning(Remedy::Advisory {
        what_to_change: NonBlank::new("use a stretchier fabric".to_owned()).expect("not blank"),
    }));
    assert_eq!(pipeline.proposals(), vec![dart]);
    Ok(())
}
