//! On every fixture with a reference seam set, every side of every seam is as long, flat, as
//! every other side it sews to, within the spec's max(1.0 mm, 0.5%), judged by the testkit's
//! own restatement of that rule rather than the engine's.

use openplush_pipeline::Mm;
use openplush_testkit::oracles::seam_parity;

#[allow(dead_code, reason = "each test file uses part of the shared fixtures")]
mod common;

use common::Setup;
use common::sides::{path, reference};

const FIXTURES: [(&str, &str, &str); 4] = [
    (
        "capsule",
        include_str!("../../../fixtures/meshes/capsule.obj"),
        include_str!("../../../fixtures/seams/capsule.json"),
    ),
    (
        "cube",
        include_str!("../../../fixtures/meshes/cube.obj"),
        include_str!("../../../fixtures/seams/cube.json"),
    ),
    (
        "sphere",
        include_str!("../../../fixtures/meshes/sphere.obj"),
        include_str!("../../../fixtures/seams/sphere.json"),
    ),
    (
        "capsule-uv",
        include_str!("../../../fixtures/meshes/source-seams/capsule-uv.obj"),
        include_str!("../../../fixtures/seams/capsule-uv.json"),
    ),
];

#[test]
fn both_sides_of_every_seam_match_on_every_fixture() -> Result<(), Setup> {
    // At 5 mm a model unit, the 0.5% term outgrows the 1 mm floor on the longer seams.
    for mm_per_unit in [1.0, 5.0] {
        for (name, obj, seams) in FIXTURES {
            let run = reference(obj, seams, mm_per_unit)?;
            assert!(!run.sewing.bundles().is_empty(), "{name}: no seams");
            for bundle in run.sewing.bundles() {
                let lengths = bundle
                    .sides()
                    .iter()
                    .map(|s| Ok(Mm::try_new(path(&run, s)?.length())?))
                    .collect::<Result<Vec<_>, Setup>>()?;
                for (i, &a) in lengths.iter().enumerate() {
                    for &b in &lengths[i + 1..] {
                        seam_parity(a, b).map_err(|e| {
                            format!(
                                "{name} at {mm_per_unit} mm/unit, {:?}: {e:?}",
                                bundle.chain()
                            )
                        })?;
                    }
                }
            }
        }
    }
    Ok(())
}
