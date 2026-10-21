//! The pipeline session: one repaired mesh, the charts its current seams make, and each chart's
//! layout stepped one flatten step at a time and remembered by the chart's content.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared helpers"
)]
#[path = "common/grid.rs"]
mod grid;

use grid::{Setup, bumpy, column_cut, mesh_of, seam, tetra_cut, tetrahedron_then_grid};
use openplush_pipeline::{
    ChartId, EdgeId, EngineError, FlattenError, ManifoldMesh, Parameterization, Pipeline, Progress,
    SeamKind, SeamSet, SeamsError, chart, flatten,
};

/// The bumpy 6 by 2 sheet cut up column 3: chart 0 is the left half, chart 6 the right, each
/// numbered by its lowest face.
fn halves() -> Result<(ManifoldMesh, SeamSet), Setup> {
    let mesh = mesh_of(&bumpy(6, 2, 3.0))?;
    let mut seams = SeamSet::new();
    column_cut(&mesh, &mut seams, 3, 2)?;
    Ok((mesh, seams))
}

fn ids(n: &[u32]) -> Vec<ChartId> {
    n.iter().copied().map(ChartId::new).collect()
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

/// Every step to `Done`, the `Done` included.
fn steps(pipeline: &mut Pipeline) -> Result<Vec<Progress>, EngineError> {
    let mut seen = Vec::new();
    loop {
        let progress = pipeline.flatten_step()?;
        seen.push(progress);
        if progress == Progress::Done {
            return Ok(seen);
        }
        assert!(seen.len() < 2_000, "the session must finish");
    }
}

/// The chart each step worked on; `None` for `Done`.
fn worked_on(steps: &[Progress]) -> Vec<Option<ChartId>> {
    steps
        .iter()
        .map(|p| match p {
            Progress::Working { chart, .. } | Progress::ChartDone { chart } => Some(*chart),
            _ => None,
        })
        .collect()
}

#[test]
fn a_new_session_holds_the_charts_of_no_seams() -> Result<(), Setup> {
    let mesh = mesh_of(&bumpy(4, 2, 3.0))?;
    let pipeline = Pipeline::new(mesh.clone());
    assert_eq!(pipeline.charts(), &chart(&mesh, &SeamSet::new()));
    Ok(())
}

#[test]
fn charts_are_the_chart_stages_for_the_current_seams() -> Result<(), Setup> {
    let (mesh, seams) = halves()?;
    let mut pipeline = Pipeline::new(mesh.clone());
    pipeline.set_seams(&seams);
    assert_eq!(pipeline.charts(), &chart(&mesh, &seams));
    Ok(())
}

#[test]
fn a_bad_seam_set_leaves_the_chart_stages_refusal_and_nothing_to_flatten() -> Result<(), Setup> {
    let (mesh, _) = halves()?;
    let mut seams = SeamSet::new();
    seams.insert(EdgeId::new(10_000), SeamKind::Cut)?;
    let mut pipeline = Pipeline::new(mesh.clone());
    assert_eq!(pipeline.set_seams(&seams), ids(&[]));
    let refusal = chart(&mesh, &seams).map(|_| ());
    assert!(matches!(
        refusal,
        Err(EngineError::Seams(SeamsError::UnknownEdge { .. }))
    ));
    assert_eq!(
        pipeline.charts().as_ref().map(|_| ()),
        refusal.as_ref().map(|_| ())
    );
    let refusal = refusal.err();
    assert_eq!(pipeline.flatten_step().err(), refusal);
    assert_eq!(pipeline.parameterizations().err(), refusal);
    Ok(())
}

#[test]
fn set_seams_names_every_chart_the_old_seams_did_not_make_and_no_other() -> Result<(), Setup> {
    let (mesh, seams) = halves()?;
    let mut pipeline = Pipeline::new(mesh.clone());
    assert_eq!(pipeline.set_seams(&seams), ids(&[0, 6]));
    assert_eq!(pipeline.set_seams(&seams), ids(&[]));
    let mut dart = seams.clone();
    seam(&mesh, &mut dart, SeamKind::Dart, &[(4, 0), (4, 1)])?;
    assert_eq!(
        pipeline.set_seams(&dart),
        ids(&[6]),
        "only the right half changed"
    );
    assert_eq!(pipeline.set_seams(&seams), ids(&[6]));
    Ok(())
}

#[test]
fn a_split_elsewhere_leaves_a_chart_unchanged_and_keeps_its_layout() -> Result<(), Setup> {
    let (mesh, seams) = halves()?;
    let mut pipeline = Pipeline::new(mesh.clone());
    pipeline.set_seams(&seams);
    pipeline.parameterizations()?;
    let mut split = seams.clone();
    column_cut(&mesh, &mut split, 1, 2)?;
    assert_eq!(
        pipeline.set_seams(&split),
        ids(&[0, 2]),
        "the left half's two strips"
    );
    let reused = steps(&mut pipeline)?;
    assert!(
        !worked_on(&reused).contains(&Some(ChartId::new(6))),
        "the right half is not flattened again: {reused:?}"
    );
    let (charts, _) = chart(&mesh, &split)?.value;
    assert_eq!(
        bits(&pipeline.parameterizations()?.value),
        bits(&flatten(&mesh, &charts)?.value)
    );
    Ok(())
}

#[test]
fn steps_flatten_the_lowest_chart_without_a_layout_first_and_are_done_only_after_all()
-> Result<(), Setup> {
    let (mesh, seams) = halves()?;
    let mut pipeline = Pipeline::new(mesh.clone());
    pipeline.set_seams(&seams);
    let all = steps(&mut pipeline)?;
    let on = worked_on(&all);
    let last_of_0 = on.iter().rposition(|c| *c == Some(ChartId::new(0)));
    let first_of_6 = on.iter().position(|c| *c == Some(ChartId::new(6)));
    assert_eq!(on.first(), Some(&Some(ChartId::new(0))));
    assert!(
        last_of_0 < first_of_6,
        "chart 0 is finished before chart 6: {all:?}"
    );
    assert!(
        on.iter().filter(|c| *c == &Some(ChartId::new(6))).count() > 1,
        "chart 6 takes several steps: {all:?}"
    );
    assert_eq!(
        all.iter().filter(|p| **p == Progress::Done).count(),
        1,
        "only the last step is Done"
    );
    assert!(all.contains(&Progress::ChartDone {
        chart: ChartId::new(0)
    }));
    assert_eq!(pipeline.flatten_step()?, Progress::Done, "Done stays Done");
    let (charts, _) = chart(&mesh, &seams)?.value;
    assert_eq!(
        bits(&pipeline.parameterizations()?.value),
        bits(&flatten(&mesh, &charts)?.value)
    );
    Ok(())
}

#[test]
fn after_an_edit_only_the_charts_without_a_layout_are_stepped() -> Result<(), Setup> {
    let (mesh, seams) = halves()?;
    let mut pipeline = Pipeline::new(mesh.clone());
    pipeline.set_seams(&seams);
    pipeline.parameterizations()?;
    let mut dart = seams.clone();
    seam(&mesh, &mut dart, SeamKind::Dart, &[(4, 0), (4, 1)])?;
    pipeline.set_seams(&dart);
    let on = worked_on(&steps(&mut pipeline)?);
    assert!(on.iter().flatten().all(|c| *c == ChartId::new(6)), "{on:?}");
    assert!(!on.is_empty());
    assert_eq!(pipeline.parameterizations()?.value.len(), 2);
    Ok(())
}

#[test]
fn parameterizations_runs_the_rest_to_the_end_as_the_flatten_stage_would() -> Result<(), Setup> {
    let (mesh, seams) = halves()?;
    let mut pipeline = Pipeline::new(mesh.clone());
    pipeline.set_seams(&seams);
    assert!(matches!(pipeline.flatten_step()?, Progress::Working { .. }));
    let staged = pipeline.parameterizations()?;
    assert_eq!(staged.warnings.iter().count(), 0);
    let (charts, _) = chart(&mesh, &seams)?.value;
    assert_eq!(bits(&staged.value), bits(&flatten(&mesh, &charts)?.value));
    assert_eq!(pipeline.flatten_step()?, Progress::Done);
    Ok(())
}

#[test]
fn a_seam_change_mid_flatten_drops_the_session_in_progress() -> Result<(), Setup> {
    let mesh = mesh_of(&bumpy(4, 4, 4.0))?;
    let mut pipeline = Pipeline::new(mesh);
    let first = ChartId::new(0);
    for expected in 1..=3 {
        assert!(matches!(
            pipeline.flatten_step()?,
            Progress::Working { chart, iteration, .. } if chart == first && iteration == expected
        ));
    }
    pipeline.set_seams(&SeamSet::new());
    assert!(matches!(
        pipeline.flatten_step()?,
        Progress::Working { chart, iteration: 1, .. } if chart == first
    ));
    Ok(())
}

#[test]
fn a_refused_chart_keeps_its_refusal_and_nothing_else_is_stepped_until_the_seams_change()
-> Result<(), Setup> {
    let mesh = tetrahedron_then_grid()?;
    let one_edge = tetra_cut(&mesh, &[[100., 0., 0.], [110., 0., 0.]])?;
    let mut pipeline = Pipeline::new(mesh.clone());
    assert_eq!(pipeline.set_seams(&one_edge), ids(&[0, 4]));
    let refusal = EngineError::Flatten(FlattenError::NotADisk {
        chart: ChartId::new(0),
    });
    for _ in 0..3 {
        assert_eq!(pipeline.flatten_step(), Err(refusal.clone()));
        assert_eq!(pipeline.parameterizations().err(), Some(refusal.clone()));
    }
    let two_edges = tetra_cut(&mesh, &[[100., 0., 0.], [110., 0., 0.], [100., 10., 0.]])?;
    assert_eq!(pipeline.set_seams(&two_edges), ids(&[0]));
    let on = worked_on(&steps(&mut pipeline)?);
    assert!(
        on.contains(&Some(ChartId::new(4))),
        "the grid was not laid out while the tetrahedron was refused: {on:?}"
    );
    Ok(())
}
