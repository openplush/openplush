//! `pattern <MODEL> --seams <PATH> --fabric <PATH> --size-mm <MM> -o <DIR>`: the model through
//! every stage, written as `pattern.svg` and `pattern.pdf` with every warning the run raised.

use std::fs;
use std::path::Path;

use openplush_pipeline::{
    EdgeId, LayoutOptions, ManifoldMesh, Mm, Paper, PatternInput, PatternOptions, PatternPiece,
    RepairPlan, RepairReport, SeamKind, SeamSet, SeamsError, Sheet, SizeTarget, Staged, WarningSet,
    chart, export_pdf, export_svg, flatten, import, layout, pattern, repair, size, templates,
};

use crate::failure::{Failure, io};
use crate::input::{CAP, files};
use crate::{fabric, seam_set};

/// Space between pieces on the print.
const GAP_MM: f64 = 10.0;

/// What `pattern` reads and where it writes.
pub(crate) struct Inputs<'a> {
    pub(crate) model: &'a Path,
    pub(crate) seams: &'a Path,
    pub(crate) fabric: &'a Path,
    pub(crate) size: Mm,
    pub(crate) page: Paper,
    pub(crate) out: &'a Path,
}

/// Runs every stage and writes both files; nothing is written unless every stage succeeds.
pub(crate) fn run(inputs: &Inputs<'_>) -> Result<(), Failure> {
    let fabric = read(inputs.fabric, fabric::parse)?;
    let seam_file = read(inputs.seams, seam_set::parse)?;
    let mut warnings = WarningSet::default();
    let soup = keep(&mut warnings, import(&files(inputs.model, CAP)?)?);
    let (mesh, report) = keep(&mut warnings, repair(&soup, &RepairPlan::default())?);
    let mut seams = seam_set::apply(&seam_file, &mesh).map_err(|why| match why {
        seam_set::Unplaced::Seam(detail) => Failure::Input {
            path: inputs.seams.to_path_buf(),
            detail,
        },
        seam_set::Unplaced::Set(e) => Failure::Engine(e.into()),
    })?;
    forced(&mut seams, &mesh, &report).map_err(|e| Failure::Engine(e.into()))?;
    let (charts, sewing) = keep(&mut warnings, chart(&mesh, &seams)?);
    let layouts = keep(&mut warnings, flatten(&mesh, &charts)?);
    let target = SizeTarget::LongestDimension(inputs.size);
    let scale = keep(&mut warnings, size(&mesh, target)?);
    let input = PatternInput {
        mesh: &mesh,
        charts: &charts,
        sewing: &sewing,
        layouts: &layouts,
    };
    let roll = fabric.spec().roll_width;
    let opts = PatternOptions { scale, fabric };
    let pieces = keep(&mut warnings, pattern(&input, &opts)?);
    let pieces = templates(&input, &pieces, &opts, &mut warnings)?;
    let pieces = keep(&mut warnings, pieces);
    let sheet = print(&pieces, roll, &mut warnings)?;
    write(inputs, &sheet, &pieces, &warnings)
}

/// Marks mandatory every copy of each edge repair split where it forced a seam, so the copies
/// are sewn back together instead of left open; another kind already on one is refused.
fn forced(
    seams: &mut SeamSet,
    mesh: &ManifoldMesh,
    report: &RepairReport,
) -> Result<(), SeamsError> {
    let is_forced = |e: &EdgeId| {
        let h = e.half_edge();
        let mut ends = [mesh.origin(h), mesh.dest(h)];
        ends.sort();
        report.mandatory_seams().contains(&ends)
    };
    for group in mesh
        .edge_copies()
        .iter()
        .filter(|g| g.iter().any(is_forced))
    {
        for &e in group {
            seams.insert(e, SeamKind::Mandatory)?;
        }
    }
    Ok(())
}

/// The print's one sheet: every piece once, on a roll as wide as the fabric's.
fn print(pieces: &[PatternPiece], roll: Mm, warnings: &mut WarningSet) -> Result<Sheet, Failure> {
    // INVARIANT: the gap is a finite non-negative literal.
    let gap = Mm::try_new(GAP_MM).unwrap_or_else(|_| unreachable!());
    let opts = LayoutOptions::new(roll, gap, false).map_err(|e| Failure::Engine(e.into()))?;
    let sheets = keep(warnings, layout(pieces, &opts)?);
    // INVARIANT: `layout` places every template on one sheet.
    let [sheet] = <[Sheet; 1]>::try_from(sheets).unwrap_or_else(|_| unreachable!());
    Ok(sheet)
}

fn write(
    inputs: &Inputs<'_>,
    sheet: &Sheet,
    pieces: &[PatternPiece],
    warnings: &WarningSet,
) -> Result<(), Failure> {
    let svg = export_svg(sheet, pieces, warnings)?.value;
    let pdf = export_pdf(sheet, pieces, inputs.page, warnings)?.value;
    fs::create_dir_all(inputs.out).map_err(io(inputs.out))?;
    for (name, bytes) in [("pattern.svg", svg), ("pattern.pdf", pdf)] {
        let path = inputs.out.join(name);
        fs::write(&path, bytes).map_err(io(&path))?;
    }
    Ok(())
}

/// `path`'s bytes through `parse`, a refusal naming the file.
fn read<T>(path: &Path, parse: fn(&[u8]) -> Result<T, String>) -> Result<T, Failure> {
    let bytes = fs::read(path).map_err(io(path))?;
    parse(&bytes).map_err(|detail| Failure::Input {
        path: path.to_path_buf(),
        detail,
    })
}

/// A stage's output, its warnings added to `warnings`.
fn keep<T>(warnings: &mut WarningSet, staged: Staged<T>) -> T {
    for w in staged.warnings.iter() {
        warnings.insert(w.clone());
    }
    staged.value
}
