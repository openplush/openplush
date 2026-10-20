//! `repair <MODEL> -o <PATH>`: the repaired mesh as OBJ, and the plan that made it as JSON.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use openplush_pipeline::{LoopRef, RepairPlan, SourceCorner, import, repair};
use serde_json::{Value, json};

use crate::failure::{Failure, io};
use crate::input::{CAP, files};
use crate::obj;

/// Repairs `model` with `plan`, writing the mesh to `out` and, if asked, the plan to `plan_out`.
/// Nothing is written unless repair succeeds.
pub(crate) fn run(
    model: &Path,
    plan: &RepairPlan,
    out: &Path,
    plan_out: Option<&Path>,
) -> Result<(), Failure> {
    let soup = import(&files(model, CAP)?)?.value;
    let (mesh, _) = repair(&soup, plan)?.value;
    fs::write(out, obj::write(&mesh)).map_err(io(out))?;
    if let Some(path) = plan_out {
        fs::write(path, plan_json(plan).to_string()).map_err(io(path))?;
    }
    Ok(())
}

/// The plan as the project schema's `repairPlan`.
fn plan_json(plan: &RepairPlan) -> Value {
    json!({
        "weld": plan.weld,
        "dropDegenerates": plan.drop_degenerates,
        "splitNonManifoldEdges": plan.split_non_manifold_edges,
        "splitBowties": plan.split_bowties,
        "fixOrientation": plan.fix_orientation,
        "fillHoles": plan.fill_holes.iter().map(|&l| loop_text(l)).collect::<Vec<_>>(),
    })
}

/// A loop's text, `<primitive>.<vertex>` (D-067).
fn loop_text(l: LoopRef) -> String {
    let c = l.corner();
    format!("{}.{}", c.primitive, c.vertex)
}

/// A `--fill` value: only the text [`loop_text`] writes parses.
pub(crate) fn loop_ref(text: &str) -> Result<LoopRef, String> {
    let bad = || format!("{text:?} is not a loop, which is written <primitive>.<vertex>");
    let (primitive, vertex) = text.split_once('.').ok_or_else(bad)?;
    let parsed = LoopRef::new(SourceCorner {
        primitive: primitive.parse().map_err(|_| bad())?,
        vertex: vertex.parse().map_err(|_| bad())?,
    });
    if loop_text(parsed) == text {
        Ok(parsed)
    } else {
        Err(bad())
    }
}

/// The plan's defaults with each `off` step turned off and `fill` listed.
pub(crate) fn plan(off: &Off, fill: &[LoopRef]) -> RepairPlan {
    RepairPlan {
        weld: !off.weld,
        drop_degenerates: !off.drop_degenerates,
        split_non_manifold_edges: !off.split_non_manifold_edges,
        split_bowties: !off.split_bowties,
        fix_orientation: !off.fix_orientation,
        fill_holes: fill.iter().copied().collect::<BTreeSet<_>>(),
    }
}

/// The steps `--no-<step>` turned off.
#[derive(Debug, Default, clap::Args)]
pub(crate) struct Off {
    /// Do not merge coincident vertices.
    #[arg(long = "no-weld")]
    weld: bool,
    /// Do not drop repeated-index and duplicate faces or stitch caps.
    #[arg(long = "no-drop-degenerates")]
    drop_degenerates: bool,
    /// Do not pair the faces on edges with more than two.
    #[arg(long = "no-split-non-manifold-edges")]
    split_non_manifold_edges: bool,
    /// Do not split vertices whose faces form more than one fan.
    #[arg(long = "no-split-bowties")]
    split_bowties: bool,
    /// Do not turn faces to agree and closed shells outward.
    #[arg(long = "no-fix-orientation")]
    fix_orientation: bool,
}
