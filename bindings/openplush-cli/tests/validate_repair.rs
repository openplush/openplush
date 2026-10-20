//! `validate` and `repair` run the engine on a model file and write what D-070 says: the health
//! report and the plan as JSON, the repaired mesh as OBJ, and a refusal as its code and message.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use openplush_pipeline::{ImportFile, ImportFiles, RepairPlan, import, repair};
use serde_json::Value;

type Setup = Box<dyn std::error::Error>;

fn cli(args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_openplush-cli"))
        .args(args)
        .output()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn fixture(path: &str) -> String {
    format!("{}/../../fixtures/{path}", env!("CARGO_MANIFEST_DIR"))
}

/// A fresh directory for one test's files.
fn scratch(test: &str) -> Result<PathBuf, Setup> {
    let dir = std::env::temp_dir().join(format!("openplush-cli-{}-{test}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn text(path: &Path) -> Result<String, Setup> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "a non-UTF-8 path".into())
}

fn json(path: &Path) -> Result<Value, Setup> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

const REPORT_KEYS: [&str; 13] = [
    "unmeasurable",
    "unwelded",
    "zero_area",
    "repeated_index",
    "duplicates",
    "non_manifold_edges",
    "non_manifold_vertices",
    "winding_conflicts",
    "non_orientable",
    "inside_out",
    "boundary_loops",
    "components",
    "self_intersections",
];

#[test]
fn validate_writes_every_report_list_under_its_accessor_s_name() -> Result<(), Setup> {
    let dir = scratch("validate-hole")?;
    let report = dir.join("report.json");
    let out = cli(&[
        "validate",
        &fixture("defects/hole.obj"),
        "--json",
        &text(&report)?,
    ])?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let v = json(&report)?;
    let keys: Vec<&str> = v
        .as_object()
        .ok_or("not an object")?
        .keys()
        .map(String::as_str)
        .collect();
    let mut expected = REPORT_KEYS.to_vec();
    expected.sort();
    assert_eq!(keys, expected);
    assert_eq!(v["unmeasurable"], Value::Null);
    let loops = v["boundary_loops"].as_array().ok_or("no loops")?;
    assert_eq!(loops.len(), 1, "{v}");
    assert!(
        loops[0]
            .as_array()
            .ok_or("a loop")?
            .iter()
            .all(Value::is_u64),
        "{v}"
    );
    assert_eq!(v["components"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[test]
fn validate_succeeds_on_a_model_full_of_defects() -> Result<(), Setup> {
    let dir = scratch("validate-defects")?;
    for name in ["duplicate-face", "non-manifold-edge", "zero-area-face"] {
        let report = dir.join(format!("{name}.json"));
        let model = fixture(&format!("defects/{name}.obj"));
        let out = cli(&["validate", &model, "--json", &text(&report)?])?;
        assert_eq!(out.status.code(), Some(0), "{name}: {}", stderr(&out));
        let v = json(&report)?;
        let found: usize = REPORT_KEYS[1..]
            .iter()
            .map(|k| v[*k].as_array().map_or(0, Vec::len))
            .sum();
        assert!(found > 0, "{name}: no defect reported: {v}");
    }
    Ok(())
}

#[test]
fn validate_writes_duplicates_and_non_manifold_edges_as_objects() -> Result<(), Setup> {
    let dir = scratch("validate-objects")?;
    let (dup, edge) = (dir.join("dup.json"), dir.join("edge.json"));
    let ran = cli(&[
        "validate",
        &fixture("defects/duplicate-face.obj"),
        "--json",
        &text(&dup)?,
    ])?;
    assert!(ran.status.success(), "{}", stderr(&ran));
    let d = &json(&dup)?["duplicates"][0];
    assert!(d["face"].is_u64() && d["of"].is_u64(), "{d}");
    assert!(["same", "opposite"].contains(&d["orientation"].as_str().ok_or("orientation")?));
    let ran = cli(&[
        "validate",
        &fixture("defects/non-manifold-edge.obj"),
        "--json",
        &text(&edge)?,
    ])?;
    assert!(ran.status.success(), "{}", stderr(&ran));
    let e = &json(&edge)?["non_manifold_edges"][0];
    assert_eq!(e["vertices"].as_array().map(Vec::len), Some(2), "{e}");
    assert!(e["faces"].as_array().map_or(0, Vec::len) > 2, "{e}");
    Ok(())
}

/// The fixture's one hole, named as `--fill` takes it.
fn hole_loop() -> Result<String, Setup> {
    let files = ImportFiles {
        primary: ImportFile {
            name: "hole.obj".into(),
            bytes: fs::read(fixture("defects/hole.obj"))?,
        },
        others: Vec::new(),
    };
    let soup = import(&files)?.value;
    let (_, report) = repair(&soup, &RepairPlan::default())?.value;
    let rim = report.holes().first().ok_or("no hole")?.loop_ref().corner();
    Ok(format!("{}.{}", rim.primitive, rim.vertex))
}

/// Faces and boundary loops of an OBJ the CLI wrote, read back through the engine.
fn read_back(path: &Path) -> Result<(usize, usize), Setup> {
    let files = ImportFiles {
        primary: ImportFile {
            name: "back.obj".into(),
            bytes: fs::read(path)?,
        },
        others: Vec::new(),
    };
    let soup = import(&files)?.value;
    let (mesh, _) = repair(&soup, &RepairPlan::default())?.value;
    Ok((mesh.face_count(), mesh.boundary_loops().len()))
}

#[test]
fn repair_writes_the_repaired_mesh_as_obj_and_fills_only_the_listed_loop() -> Result<(), Setup> {
    let dir = scratch("repair-fill")?;
    let (open, filled) = (dir.join("open.obj"), dir.join("filled.obj"));
    let model = fixture("defects/hole.obj");
    let out = cli(&["repair", &model, "-o", &text(&open)?])?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let (open_faces, open_loops) = read_back(&open)?;
    assert_eq!(open_loops, 1, "an unlisted hole stays open");
    let rim = hole_loop()?;
    let out = cli(&["repair", &model, "--fill", &rim, "-o", &text(&filled)?])?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let (filled_faces, filled_loops) = read_back(&filled)?;
    assert_eq!(filled_loops, 0);
    assert!(filled_faces > open_faces);
    Ok(())
}

#[test]
fn repair_writes_its_plan_as_the_project_schema_s_repair_plan() -> Result<(), Setup> {
    let dir = scratch("repair-plan")?;
    let (plan, mesh) = (dir.join("plan.json"), dir.join("fixed.obj"));
    let rim = hole_loop()?;
    let out = cli(&[
        "repair",
        &fixture("defects/hole.obj"),
        "--no-weld",
        "--no-split-bowties",
        "--fill",
        &rim,
        "--plan-out",
        &text(&plan)?,
        "-o",
        &text(&mesh)?,
    ])?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let expected = serde_json::json!({
        "weld": false,
        "dropDegenerates": true,
        "splitNonManifoldEdges": true,
        "splitBowties": false,
        "fixOrientation": true,
        "fillHoles": [rim],
    });
    assert_eq!(json(&plan)?, expected);
    Ok(())
}

#[test]
fn every_no_flag_turns_its_step_off() -> Result<(), Setup> {
    let dir = scratch("repair-no")?;
    let (plan, mesh) = (dir.join("plan.json"), dir.join("fixed.obj"));
    let out = cli(&[
        "repair",
        &fixture("meshes/cube.obj"),
        "--no-weld",
        "--no-drop-degenerates",
        "--no-split-non-manifold-edges",
        "--no-split-bowties",
        "--no-fix-orientation",
        "--plan-out",
        &text(&plan)?,
        "-o",
        &text(&mesh)?,
    ])?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let v = json(&plan)?;
    for key in [
        "weld",
        "dropDegenerates",
        "splitNonManifoldEdges",
        "splitBowties",
        "fixOrientation",
    ] {
        assert_eq!(v[key], Value::Bool(false), "{key}: {v}");
    }
    assert_eq!(v["fillHoles"], serde_json::json!([]));
    Ok(())
}

#[test]
fn the_obj_writer_keeps_each_corner_s_uv() -> Result<(), Setup> {
    let dir = scratch("repair-uv")?;
    let (model, mesh) = (dir.join("sheet.obj"), dir.join("fixed.obj"));
    fs::write(
        &model,
        "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\n\
         f 1/1 2/2 3/3\nf 1/1 3/3 4/4\n",
    )?;
    let out = cli(&["repair", &text(&model)?, "-o", &text(&mesh)?])?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let written = fs::read_to_string(&mesh)?;
    let count = |p: &str| written.lines().filter(|l| l.starts_with(p)).count();
    assert_eq!(
        (count("v "), count("vt "), count("f ")),
        (4, 6, 2),
        "{written}"
    );
    assert!(
        written
            .lines()
            .filter(|l| l.starts_with("f "))
            .all(|l| l.matches('/').count() == 3),
        "{written}"
    );
    assert!(!written.contains("vn"), "{written}");
    Ok(())
}

#[test]
fn an_engine_refusal_prints_its_code_and_message_and_exits_1() -> Result<(), Setup> {
    let dir = scratch("refusal")?;
    let model = dir.join("model.xyz");
    fs::write(&model, "not a model")?;
    let out = cli(&[
        "validate",
        &text(&model)?,
        "--json",
        &text(&dir.join("r.json"))?,
    ])?;
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let expected =
        "openplush-cli: UnknownFormat: cannot read \"model.xyz\": no reader for this file type\n";
    assert_eq!(stderr(&out), expected);
    assert!(!dir.join("r.json").exists(), "a refusal wrote a report");
    let out = cli(&[
        "repair",
        &fixture("defects/hole.obj"),
        "--fill",
        "0.999",
        "-o",
        &text(&dir.join("x.obj"))?,
    ])?;
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).starts_with("openplush-cli: Mesh: "),
        "{}",
        stderr(&out)
    );
    assert!(!dir.join("x.obj").exists(), "a refusal wrote a mesh");
    Ok(())
}

#[test]
fn a_file_that_cannot_be_read_or_written_exits_1() -> Result<(), Setup> {
    let dir = scratch("io")?;
    let missing = dir.join("missing.obj");
    let out = cli(&[
        "validate",
        &text(&missing)?,
        "--json",
        &text(&dir.join("r.json"))?,
    ])?;
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).starts_with("openplush-cli: "),
        "{}",
        stderr(&out)
    );
    let nowhere = dir.join("no-such-dir").join("r.json");
    let out = cli(&[
        "validate",
        &fixture("meshes/cube.obj"),
        "--json",
        &text(&nowhere)?,
    ])?;
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    Ok(())
}
