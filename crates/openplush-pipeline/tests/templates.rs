//! The `templates` stage: identical and mirror-image pieces made one template with its cut
//! count, partners named by template, matched seams named together and notched alike, nothing
//! merged where nothing matches.

use std::collections::BTreeMap;

use openplush_pipeline::{
    EngineError, NonBlank, PatternPiece, PieceId, Remedy, StageResult, Warning, WarningCode,
    WarningSet, WarningTarget, templates,
};

#[allow(dead_code, reason = "each test file uses part of the shared fixtures")]
mod common;

use common::sides::{Run, reference};
use common::{CAPSULE_OBJ, CAPSULE_SEAMS, CUBE_OBJ, CUBE_SEAMS, Setup};

fn templated(run: &Run) -> Result<StageResult<Vec<PatternPiece>>, Setup> {
    Ok(templates(
        &run.input(),
        &run.pieces,
        &run.options()?,
        &mut WarningSet::default(),
    ))
}

fn squeezed(piece: PieceId) -> Result<Warning, Setup> {
    let text = |t: &str| NonBlank::new(t.to_owned()).ok_or("non-blank");
    Ok(Warning::new(
        WarningCode::PieceCompressed,
        WarningTarget::Piece(piece),
        text("This piece must stretch.")?,
        Remedy::Advisory {
            what_to_change: text("Add a dart or a seam in this piece.")?,
        },
    ))
}

#[test]
fn a_warning_on_a_merged_piece_names_its_template() -> Result<(), Setup> {
    let run = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    let mut warnings = WarningSet::default();
    for p in &run.pieces {
        warnings.insert(squeezed(p.id())?);
    }
    let staged = templates(&run.input(), &run.pieces, &run.options()?, &mut warnings)
        .map_err(|e| format!("{e:?}"))?;
    let printed: Vec<WarningTarget> = staged
        .value
        .iter()
        .map(|p| WarningTarget::Piece(p.id()))
        .collect();
    let targets: Vec<WarningTarget> = warnings.iter().map(Warning::target).collect();
    // The twin cap's warning, now on cap 0, equals cap 0's own and is kept once.
    assert_eq!(targets, printed);
    Ok(())
}

fn label_lines(p: &PatternPiece) -> Result<Vec<String>, Setup> {
    Ok(p.label().ok_or("a label")?.lines().to_vec())
}

#[test]
fn the_capsule_s_two_end_caps_are_one_template_cut_twice() -> Result<(), Setup> {
    let run = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    let staged = templated(&run)?;
    let staged = staged.map_err(|e| format!("{e:?}"))?;
    assert_eq!(staged.warnings.iter().count(), 0);
    let ids: Vec<u32> = staged.value.iter().map(|p| p.id().index()).collect();
    assert_eq!(ids, [0, 5]);
    let (cap, tube) = (&staged.value[0], &staged.value[1]);
    // A round cap fits its twin as printed, which is tried first.
    assert_eq!((cap.cuts().as_printed(), cap.cuts().flipped()), (2, 0));
    assert_eq!(label_lines(cap)?[1], "Cut 2");
    assert_eq!((tube.cuts().as_printed(), tube.cuts().flipped()), (1, 0));
    assert_eq!(label_lines(cap)?[3], "Sews to piece 5");
    assert_eq!(label_lines(tube)?[3], "Sews to piece 0");
    Ok(())
}

#[test]
fn a_merged_edge_names_both_seams_and_an_unmerged_piece_only_its_own() -> Result<(), Setup> {
    let run = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    let staged = templated(&run)?.map_err(|e| format!("{e:?}"))?;
    let (cap, tube) = (&staged.value[0], &staged.value[1]);
    let cap_seams: Vec<&str> = cap
        .label()
        .ok_or("label")?
        .seams()
        .iter()
        .map(|s| s.text())
        .collect();
    let [joined] = cap_seams[..] else {
        return Err(format!("the cap has one seam: {cap_seams:?}").into());
    };
    let names: Vec<&str> = joined.trim_start_matches("Seam ").split(" / ").collect();
    let [ours, theirs] = names[..] else {
        return Err(format!("two names: {joined}").into());
    };
    let tube_seams: Vec<&str> = tube
        .label()
        .ok_or("label")?
        .seams()
        .iter()
        .map(|s| s.text())
        .collect();
    assert!(
        tube_seams.iter().all(|t| !t.contains(" / ")),
        "{tube_seams:?}"
    );
    assert!(
        tube_seams.contains(&format!("Seam {ours}").as_str()),
        "{tube_seams:?}"
    );
    assert!(
        tube_seams.contains(&format!("Seam {theirs}").as_str()),
        "{tube_seams:?}"
    );
    Ok(())
}

#[test]
fn seams_matched_through_a_merge_carry_the_same_marks() -> Result<(), Setup> {
    let run = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    let staged = templated(&run)?.map_err(|e| format!("{e:?}"))?;
    let tube = &staged.value[1];
    let mut marks: BTreeMap<u32, u32> = BTreeMap::new();
    for n in tube.notches() {
        let m = marks.entry(n.chain().index()).or_insert(n.marks());
        *m = (*m).max(n.marks());
    }
    let cap = &staged.value[0];
    let joined = cap.label().ok_or("label")?.seams()[0].text().to_owned();
    let names: Vec<u32> = joined
        .trim_start_matches("Seam ")
        .split(" / ")
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    assert_eq!(names.len(), 2);
    assert_eq!(marks.get(&names[0]), marks.get(&names[1]), "{marks:?}");
    Ok(())
}

#[test]
fn nothing_merges_where_nothing_matches() -> Result<(), Setup> {
    let run = reference(CUBE_OBJ, CUBE_SEAMS, 5.0)?;
    let staged = templated(&run)?.map_err(|e| format!("{e:?}"))?;
    assert_eq!(staged.value, run.pieces);
    Ok(())
}

#[test]
fn pieces_from_another_run_or_order_are_refused() -> Result<(), Setup> {
    let run = reference(CAPSULE_OBJ, CAPSULE_SEAMS, 5.0)?;
    let reversed: Vec<PatternPiece> = run.pieces.iter().rev().cloned().collect();
    let err = templates(
        &run.input(),
        &reversed,
        &run.options()?,
        &mut WarningSet::default(),
    )
    .err()
    .ok_or("refused")?;
    assert!(
        matches!(err, EngineError::StageInputMismatch { .. }),
        "{err:?}"
    );
    let mut extra = run.pieces.clone();
    extra.push(run.pieces[0].clone());
    let err = templates(
        &run.input(),
        &extra,
        &run.options()?,
        &mut WarningSet::default(),
    )
    .err()
    .ok_or("refused")?;
    assert!(
        matches!(err, EngineError::StageInputMismatch { .. }),
        "{err:?}"
    );
    let short = &run.pieces[..1];
    let err = templates(
        &run.input(),
        short,
        &run.options()?,
        &mut WarningSet::default(),
    )
    .err()
    .ok_or("refused")?;
    assert!(
        matches!(err, EngineError::StageInputMismatch { .. }),
        "{err:?}"
    );
    Ok(())
}
