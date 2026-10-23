//! The same input gives byte-identical exports: each fixture with a reference seam set, from its
//! file bytes through every stage to the SVG and to the PDF on both papers, run twice from scratch.

#[allow(dead_code, reason = "each test file uses part of the shared fixtures")]
mod common;

use common::{CAPSULE_OBJ, CAPSULE_SEAMS, CUBE_OBJ, CUBE_SEAMS, Setup, fabric, reference_seams};
use openplush_pipeline::{
    ImportFile, ImportFiles, LayoutOptions, Mm, Paper, PatternInput, PatternOptions, RepairPlan,
    SizeTarget, WarningSet, chart, export_pdf, export_svg, flatten, import, layout, pattern,
    repair, size,
};

/// One run's exports: the SVG, then the PDF on A4 and on Letter.
struct Exports {
    svg: Vec<u8>,
    pdf_a4: Vec<u8>,
    pdf_letter: Vec<u8>,
}

/// `obj` with the seam set `seams`, from its bytes to its exports, with every stage's warnings
/// printed as a shell would print them.
fn run(name: &str, obj: &str, seams: &str) -> Result<Exports, Setup> {
    let files = ImportFiles {
        primary: ImportFile {
            name: name.into(),
            bytes: obj.as_bytes().to_vec(),
        },
        others: vec![],
    };
    let mut warnings = WarningSet::default();
    let mut keep = |w: &WarningSet| w.iter().for_each(|w| warnings.insert(w.clone()));
    let soup = import(&files)?;
    keep(&soup.warnings);
    let repaired = repair(&soup.value, &RepairPlan::default())?;
    keep(&repaired.warnings);
    let (mesh, _) = repaired.value;
    let charted = chart(&mesh, &reference_seams(obj, seams, &mesh)?)?;
    keep(&charted.warnings);
    let (charts, sewing) = charted.value;
    let flattened = flatten(&mesh, &charts)?;
    keep(&flattened.warnings);
    let scale = size(&mesh, SizeTarget::LongestDimension(Mm::try_new(200.0)?))?;
    keep(&scale.warnings);
    let input = PatternInput {
        mesh: &mesh,
        charts: &charts,
        sewing: &sewing,
        layouts: &flattened.value,
    };
    let opts = PatternOptions {
        scale: scale.value,
        fabric: fabric(6.0)?,
    };
    let pieces = pattern(&input, &opts)?;
    keep(&pieces.warnings);
    let roll = LayoutOptions::new(Mm::try_new(1500.0)?, Mm::try_new(10.0)?, false)?;
    let sheets = layout(&pieces.value, &roll)?;
    keep(&sheets.warnings);
    let [sheet] = <[_; 1]>::try_from(sheets.value).map_err(|s| format!("{} sheets", s.len()))?;
    let pieces = &pieces.value;
    Ok(Exports {
        svg: export_svg(&sheet, pieces, &warnings)?.value,
        pdf_a4: export_pdf(&sheet, pieces, Paper::A4, &warnings)?.value,
        pdf_letter: export_pdf(&sheet, pieces, Paper::Letter, &warnings)?.value,
    })
}

fn assert_runs_twice_to_the_same_bytes(name: &str, obj: &str, seams: &str) -> Result<(), Setup> {
    let first = run(name, obj, seams)?;
    assert!(first.svg.starts_with(b"<?xml"));
    assert!(first.pdf_a4.starts_with(b"%PDF-"));
    assert_ne!(first.pdf_a4, first.pdf_letter);
    // Compared field by field so a failure names the export that drifted.
    let second = run(name, obj, seams)?;
    assert!(first.svg == second.svg, "{name}: the SVG differs");
    assert!(first.pdf_a4 == second.pdf_a4, "{name}: the A4 PDF differs");
    assert!(
        first.pdf_letter == second.pdf_letter,
        "{name}: the Letter PDF differs"
    );
    Ok(())
}

#[test]
fn the_capsule_exports_the_same_bytes_twice() -> Result<(), Setup> {
    assert_runs_twice_to_the_same_bytes("capsule.obj", CAPSULE_OBJ, CAPSULE_SEAMS)
}

#[test]
fn the_cube_exports_the_same_bytes_twice() -> Result<(), Setup> {
    assert_runs_twice_to_the_same_bytes("cube.obj", CUBE_OBJ, CUBE_SEAMS)
}
