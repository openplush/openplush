//! The layout and export stages: the print laid out one template per piece, and the SVG and the
//! tiled PDF of it, each printing the warnings it is given. Refusals carry the layout and export
//! crates' errors whole, located by piece.

#[allow(dead_code, reason = "each test file uses part of the shared fixtures")]
mod common;

use common::{Setup, fabric};
use openplush_mesh::{RepairPlan, into_manifold};
use openplush_pipeline::{
    EngineError, ExportError, LayoutError, LayoutOptions, Mm, NonBlank, Paper, PatternInput,
    PatternOptions, PatternPiece, PieceId, Remedy, Scale, SeamSet, Sheet, Warning, WarningCode,
    WarningSet, WarningTarget, chart, export_pdf, export_svg, flatten, layout, pattern,
};

/// A unit square at `mm_per_unit`, cut at 6 mm: at 100 mm per unit, 100 mm sewn and 112 mm cut.
fn square(mm_per_unit: f64) -> Result<Vec<PatternPiece>, Setup> {
    let text = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n";
    let soup = openplush_io::obj::read(text.as_bytes())?;
    let mesh = into_manifold(&soup, &RepairPlan::default())?.0;
    let (charts, sewing) = chart(&mesh, &SeamSet::new())?.value;
    let layouts = flatten(&mesh, &charts)?.value;
    let input = PatternInput {
        mesh: &mesh,
        charts: &charts,
        sewing: &sewing,
        layouts: &layouts,
    };
    let opts = PatternOptions {
        scale: Scale::mm_per_model_unit(mm_per_unit)?,
        fabric: fabric(6.0)?,
    };
    Ok(pattern(&input, &opts)?.value)
}

fn roll(width_mm: f64) -> Result<LayoutOptions, Setup> {
    Ok(LayoutOptions::new(
        Mm::try_new(width_mm)?,
        Mm::try_new(10.0)?,
        false,
    )?)
}

fn one_sheet(pieces: &[PatternPiece], width_mm: f64) -> Result<Sheet, Setup> {
    let sheets = layout(pieces, &roll(width_mm)?)?.value;
    let [sheet] = <[Sheet; 1]>::try_from(sheets).map_err(|s| format!("{} sheets", s.len()))?;
    Ok(sheet)
}

fn import_ignored() -> Result<WarningSet, Setup> {
    let text = |t: &str| NonBlank::new(t.into()).ok_or("blank");
    let mut set = WarningSet::default();
    set.insert(Warning::new(
        WarningCode::ImportIgnored,
        WarningTarget::Project,
        text("The file has parts the pattern does not follow: animation 0, not played.")?,
        Remedy::Advisory {
            what_to_change: text("Apply the pose in your 3D tool and export again.")?,
        },
    ));
    Ok(set)
}

/// Every string a PDF content stream shows, in order: the text inside `(...)` before ` Tj`.
fn shown(pdf: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(pdf);
    text.split(") Tj")
        .filter_map(|before| before.rsplit_once('(').map(|(_, s)| s.to_owned()))
        .collect()
}

fn is_tile_label(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase()) && chars.all(|c| c.is_ascii_digit())
}

#[test]
fn the_svg_is_the_sheet_at_true_scale() -> Result<(), Setup> {
    let pieces = square(100.0)?;
    let sheet = one_sheet(&pieces, 500.0)?;
    let svg = String::from_utf8(export_svg(&sheet, &pieces, &WarningSet::default())?.value)?;
    let size = format!(
        "width=\"{:.3}mm\" height=\"{:.3}mm\"",
        sheet.width_mm(),
        sheet.length_mm()
    );
    assert!(svg.contains(&size), "{svg}");
    let cut = svg
        .lines()
        .find(|l| l.starts_with("<path class=\"cut\""))
        .ok_or("no cut path")?;
    let xs: Vec<f64> = cut
        .split(['M', 'L', 'Z', '"'])
        .filter_map(|p| p.split_whitespace().next()?.parse().ok())
        .collect();
    let span =
        xs.iter().copied().fold(f64::MIN, f64::max) - xs.iter().copied().fold(f64::MAX, f64::min);
    assert_eq!(format!("{span:.3}"), "112.000", "{cut}");
    Ok(())
}

#[test]
fn the_svg_prints_the_warnings_it_is_given() -> Result<(), Setup> {
    let pieces = square(100.0)?;
    let sheet = one_sheet(&pieces, 500.0)?;
    let svg = String::from_utf8(export_svg(&sheet, &pieces, &import_ignored()?)?.value)?;
    assert!(
        svg.contains(">1. import-ignored: the whole pattern<"),
        "{svg}"
    );
    Ok(())
}

#[test]
fn the_pdf_cover_prints_the_warnings_and_the_tiles_keep_their_labels() -> Result<(), Setup> {
    let pieces = square(100.0)?;
    let sheet = one_sheet(&pieces, 500.0)?;
    let warned = export_pdf(&sheet, &pieces, Paper::A4, &import_ignored()?)?.value;
    let plain = export_pdf(&sheet, &pieces, Paper::A4, &WarningSet::default())?.value;
    let lines = shown(&warned);
    assert!(
        lines
            .iter()
            .any(|l| l == "1. import-ignored: the whole pattern"),
        "{lines:?}"
    );
    let labels = |pdf: &[u8]| -> Vec<String> {
        shown(pdf)
            .into_iter()
            .filter(|s| is_tile_label(s))
            .collect()
    };
    assert!(!labels(&plain).is_empty());
    assert_eq!(labels(&warned), labels(&plain));
    Ok(())
}

#[test]
fn a_piece_wider_than_the_roll_is_layout_naming_it() -> Result<(), Setup> {
    let err = layout(&square(100.0)?, &roll(50.0)?)
        .err()
        .ok_or("a 112 mm piece on a 50 mm roll must be refused")?;
    assert!(matches!(
        err,
        EngineError::Layout(LayoutError::PieceTooWide { piece, .. }) if piece == PieceId::new(0)
    ));
    assert_eq!(err.code(), "Layout");
    assert_eq!(err.locations().pieces, vec![PieceId::new(0)]);
    Ok(())
}

#[test]
fn a_sheet_placing_a_piece_not_given_is_export_naming_it() -> Result<(), Setup> {
    let pieces = square(100.0)?;
    let sheet = one_sheet(&pieces, 500.0)?;
    for err in [
        export_svg(&sheet, &[], &WarningSet::default()).err(),
        export_pdf(&sheet, &[], Paper::Letter, &WarningSet::default()).err(),
    ] {
        let err = err.ok_or("a missing piece must be refused")?;
        assert!(matches!(
            err,
            EngineError::Export(ExportError::UnknownPiece { piece }) if piece == PieceId::new(0)
        ));
        assert_eq!(err.code(), "Export");
        assert_eq!(err.locations().pieces, vec![PieceId::new(0)]);
    }
    Ok(())
}

#[test]
fn a_sheet_too_big_to_tile_is_layout_locating_nothing() -> Result<(), Setup> {
    // 10 m square: far more than 26 A4 pages across.
    let pieces = square(10_000.0)?;
    let sheet = one_sheet(&pieces, 11_000.0)?;
    let err = export_pdf(&sheet, &pieces, Paper::A4, &WarningSet::default())
        .err()
        .ok_or("a 10 m sheet must be refused")?;
    assert!(matches!(
        err,
        EngineError::Layout(LayoutError::TooManyPages { .. })
    ));
    assert!(err.locations().pieces.is_empty());
    Ok(())
}

#[test]
fn nothing_to_lay_out_is_layout_locating_nothing() -> Result<(), Setup> {
    let err = layout(&[], &roll(500.0)?)
        .err()
        .ok_or("no pieces must be refused")?;
    assert!(matches!(err, EngineError::Layout(LayoutError::NoPieces)));
    assert!(err.locations().pieces.is_empty());
    Ok(())
}

#[test]
fn each_row_names_its_target_in_words_and_its_remedy_s_text() -> Result<(), Setup> {
    use std::collections::BTreeMap;

    use openplush_pipeline::{
        ChartId, ComponentId, EdgeId, FaceId, Proposal, ProposalKind, SeamChainId, SeamKind,
    };
    let text = |t: &str| NonBlank::new(t.into()).ok_or("blank");
    let advise = || -> Result<Remedy, Setup> {
        Ok(Remedy::Advisory {
            what_to_change: text("Change it.")?,
        })
    };
    let dart = Proposal::new(
        ProposalKind::Dart {
            chart: ChartId::new(0),
        },
        BTreeMap::from([(EdgeId::new(7), SeamKind::Dart)]),
        "Add a dart at the crown.".into(),
    )?;
    let mut set = WarningSet::default();
    let targets = [
        (WarningTarget::Face(FaceId::new(3)), "face 3"),
        (WarningTarget::Edge(EdgeId::new(4)), "edge 4"),
        (WarningTarget::Chain(SeamChainId::new(5)), "seam chain 5"),
        (WarningTarget::Piece(PieceId::new(8)), "piece 8"),
        (WarningTarget::Component(ComponentId::new(6)), "component 6"),
        (WarningTarget::Project, "the whole pattern"),
    ];
    for (n, (target, _)) in targets.iter().enumerate() {
        set.insert(Warning::new(
            WarningCode::ImportIgnored,
            *target,
            text(&format!("Why {n}."))?,
            advise()?,
        ));
    }
    set.insert(Warning::new(
        WarningCode::SizeOutOfRange,
        WarningTarget::Project,
        text("Why.")?,
        Remedy::Actionable { proposal: dart },
    ));
    let pieces = square(100.0)?;
    let sheet = one_sheet(&pieces, 500.0)?;
    let svg = String::from_utf8(export_svg(&sheet, &pieces, &set)?.value)?;
    for (n, (_, words)) in targets.iter().enumerate() {
        let line = format!(">{}. import-ignored: {words}<", n + 1);
        assert!(svg.contains(&line), "{line} in {svg}");
        assert!(
            svg.contains(&format!(">Why {n}.<")),
            "explanation {n} in {svg}"
        );
    }
    assert!(
        svg.contains(">7. size-out-of-range: the whole pattern<"),
        "{svg}"
    );
    assert!(svg.contains("Fix: Change it.<"), "{svg}");
    assert!(svg.contains("Fix: Add a dart at the crown.<"), "{svg}");
    Ok(())
}

#[test]
fn the_exports_print_the_warnings_and_raise_none_of_their_own() -> Result<(), Setup> {
    let pieces = square(100.0)?;
    let sheet = one_sheet(&pieces, 500.0)?;
    let warnings = import_ignored()?;
    assert!(
        export_svg(&sheet, &pieces, &warnings)?
            .warnings
            .iter()
            .next()
            .is_none()
    );
    let pdf = export_pdf(&sheet, &pieces, Paper::A4, &warnings)?;
    assert!(pdf.warnings.iter().next().is_none());
    Ok(())
}

#[test]
fn the_pdf_is_tiled_on_the_paper_asked_for() -> Result<(), Setup> {
    let pieces = square(100.0)?;
    let sheet = one_sheet(&pieces, 500.0)?;
    let none = WarningSet::default();
    let letter = String::from_utf8_lossy(&export_pdf(&sheet, &pieces, Paper::Letter, &none)?.value)
        .into_owned();
    let a4 =
        String::from_utf8_lossy(&export_pdf(&sheet, &pieces, Paper::A4, &none)?.value).into_owned();
    // Letter is 612 by 792 pt; A4 is about 595 by 842 pt.
    assert!(letter.contains("/MediaBox [0 0 612 792]"), "{letter}");
    assert!(!a4.contains("/MediaBox [0 0 612 792]"));
    Ok(())
}

#[test]
fn the_print_places_one_template_of_a_piece_cut_several_times() -> Result<(), Setup> {
    use openplush_pipeline::CutCount;
    let [piece] = <[PatternPiece; 1]>::try_from(square(100.0)?).map_err(|_| "one piece")?;
    let thrice = PatternPiece::new(
        piece.id(),
        piece.sewing_line().clone(),
        piece.cut_line().clone(),
        CutCount::new(3, 0)?,
        piece.rotations_deg().to_vec(),
    )?;
    let sheet = one_sheet(&[thrice], 500.0)?;
    assert_eq!(sheet.placements().len(), 1);
    Ok(())
}
