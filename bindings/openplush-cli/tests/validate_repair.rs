//! `validate` and `repair` run the engine on a model file and write what D-070 says: the health
//! report and the plan as JSON, the repaired mesh as OBJ, and a refusal as its code and message.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use openplush_pipeline::{
    ImportFile, ImportFiles, RepairPlan, ValidationReport, VertexId, import, repair, validate,
};
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

/// A fresh directory for one test's files, removed when the test ends.
struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn scratch(test: &str) -> Result<Scratch, Setup> {
    let dir = std::env::temp_dir().join(format!("openplush-cli-{}-{test}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(Scratch(dir))
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
        // `components` always has one, so it is not a defect here.
        let found: usize = REPORT_KEYS[1..]
            .iter()
            .filter(|k| **k != "components")
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

/// The engine's own report on `path`, the oracle for what `validate` writes.
fn engine_report(path: &Path) -> Result<ValidationReport, Setup> {
    let files = ImportFiles {
        primary: ImportFile {
            name: path
                .file_name()
                .ok_or("a file")?
                .to_string_lossy()
                .into_owned(),
            bytes: fs::read(path)?,
        },
        others: Vec::new(),
    };
    Ok(validate(&import(&files)?.value).value)
}

fn vertex_ids(l: &[VertexId]) -> Value {
    Value::from(l.iter().map(|x| x.index()).collect::<Vec<_>>())
}

fn face_ids(l: &[openplush_pipeline::FaceId]) -> Value {
    Value::from(l.iter().map(|x| x.index()).collect::<Vec<_>>())
}

fn rows<T>(items: &[T], row: impl Fn(&T) -> Value) -> Value {
    Value::from(items.iter().map(row).collect::<Vec<_>>())
}

/// Every list of `r` as the integers `validate` is documented to write.
fn expected_lists(r: &ValidationReport) -> Vec<(&'static str, Value)> {
    let duplicates = rows(r.duplicates(), |d| {
        serde_json::json!({
            "face": d.face.index(),
            "of": d.of.index(),
            "orientation": format!("{:?}", d.orientation).to_lowercase(),
        })
    });
    let edges = rows(
        r.non_manifold_edges(),
        |e| serde_json::json!({"vertices": vertex_ids(&e.vertices), "faces": face_ids(&e.faces)}),
    );
    vec![
        ("unwelded", rows(r.unwelded(), |g| vertex_ids(g))),
        ("zero_area", face_ids(r.zero_area())),
        ("repeated_index", face_ids(r.repeated_index())),
        ("duplicates", duplicates),
        ("non_manifold_edges", edges),
        (
            "non_manifold_vertices",
            vertex_ids(r.non_manifold_vertices()),
        ),
        (
            "winding_conflicts",
            rows(r.winding_conflicts(), |p| vertex_ids(p)),
        ),
        (
            "non_orientable",
            rows(r.non_orientable(), |p| vertex_ids(p)),
        ),
        ("inside_out", rows(r.inside_out(), |c| face_ids(c))),
        (
            "boundary_loops",
            rows(r.boundary_loops(), |l| vertex_ids(l)),
        ),
        ("components", rows(r.components(), |c| face_ids(c))),
        (
            "self_intersections",
            rows(r.self_intersections(), |p| face_ids(p)),
        ),
    ]
}

/// A cube with every face turned the other way, so its one shell is inside out.
fn inside_out_cube(dir: &Path) -> Result<PathBuf, Setup> {
    let cube = fs::read_to_string(fixture("meshes/cube.obj"))?;
    let turned: String = cube
        .lines()
        .map(|l| match l.strip_prefix("f ") {
            Some(rest) => format!(
                "f {}\n",
                rest.split_whitespace().rev().collect::<Vec<_>>().join(" ")
            ),
            None => format!("{l}\n"),
        })
        .collect();
    let path = dir.join("inside-out.obj");
    fs::write(&path, turned)?;
    Ok(path)
}

#[test]
fn validate_writes_every_list_exactly_as_the_engine_reports_it() -> Result<(), Setup> {
    let dir = scratch("validate-exact")?;
    let small = dir.join("small.obj");
    // A triangle, its reverse, a repeated-index face and a copy wound the same way.
    fs::write(
        &small,
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\nf 1 3 2\nf 1 1 2\nf 1 2 3\n",
    )?;
    let mut models: Vec<PathBuf> = [
        "bowtie-vertex",
        "duplicate-face",
        "flipped-normals",
        "hole",
        "multi-component",
        "non-manifold-edge",
        "self-intersection",
        "unwelded-split-vertices",
        "zero-area-face",
    ]
    .iter()
    .map(|n| PathBuf::from(fixture(&format!("defects/{n}.obj"))))
    .collect();
    models.extend([small, inside_out_cube(&dir)?]);
    let mut seen = std::collections::BTreeSet::new();
    for model in &models {
        let report = dir.join("r.json");
        let out = cli(&["validate", &text(model)?, "--json", &text(&report)?])?;
        assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
        let written = json(&report)?;
        for (key, expected) in expected_lists(&engine_report(model)?) {
            assert_eq!(written[key], expected, "{key} of {}", model.display());
            if expected.as_array().is_some_and(|a| !a.is_empty()) {
                seen.insert(key);
            }
        }
    }
    let orientations: Vec<String> = models
        .iter()
        .filter_map(|m| engine_report(m).ok())
        .flat_map(|r| {
            r.duplicates()
                .iter()
                .map(|d| format!("{:?}", d.orientation))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(orientations.contains(&"Same".into()) && orientations.contains(&"Opposite".into()));
    // Every list but `non_orientable` is non-empty for some model, so a swap or a drop shows.
    let all: std::collections::BTreeSet<&str> = REPORT_KEYS[1..].iter().copied().collect();
    let missing: Vec<_> = all.difference(&seen).collect();
    assert_eq!(missing, [&"non_orientable"], "lists no model exercises");
    Ok(())
}

#[test]
fn validate_writes_why_a_model_cannot_be_measured() -> Result<(), Setup> {
    let dir = scratch("unmeasurable")?;
    let cases = [
        ("v 0 0 0\n", serde_json::json!("no_triangles")),
        (
            "v 1 1 1\nf 1 1 1\n",
            serde_json::json!({"extent": {"diagonal": 0.0}}),
        ),
    ];
    for (obj, why) in cases {
        let (model, report) = (dir.join("m.obj"), dir.join("r.json"));
        fs::write(&model, obj)?;
        let out = cli(&["validate", &text(&model)?, "--json", &text(&report)?])?;
        assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
        let v = json(&report)?;
        assert_eq!(v["unmeasurable"], why, "{obj:?}");
        for key in &REPORT_KEYS[1..] {
            assert_eq!(v[*key], serde_json::json!([]), "{key} of {obj:?}");
        }
    }
    Ok(())
}

const STEP_FLAGS: [(&str, &str); 5] = [
    ("--no-weld", "weld"),
    ("--no-drop-degenerates", "dropDegenerates"),
    ("--no-split-non-manifold-edges", "splitNonManifoldEdges"),
    ("--no-split-bowties", "splitBowties"),
    ("--no-fix-orientation", "fixOrientation"),
];

#[test]
fn each_no_flag_turns_off_its_own_step_and_no_other() -> Result<(), Setup> {
    let dir = scratch("repair-one-off")?;
    let (plan, mesh) = (dir.join("plan.json"), dir.join("fixed.obj"));
    for (flag, key) in STEP_FLAGS {
        let out = cli(&[
            "repair",
            &fixture("meshes/cube.obj"),
            flag,
            "--plan-out",
            &text(&plan)?,
            "-o",
            &text(&mesh)?,
        ])?;
        assert_eq!(out.status.code(), Some(0), "{flag}: {}", stderr(&out));
        let v = json(&plan)?;
        for (_, other) in STEP_FLAGS {
            assert_eq!(
                v[other],
                Value::Bool(other != key),
                "{flag}: {other} in {v}"
            );
        }
    }
    Ok(())
}

fn vertex_lines(path: &Path) -> Result<usize, Setup> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|l| l.starts_with("v "))
        .count())
}

#[test]
fn repair_runs_the_engine_with_the_plan_it_writes() -> Result<(), Setup> {
    let dir = scratch("repair-plan-used")?;
    let (welded, unwelded) = (dir.join("welded.obj"), dir.join("unwelded.obj"));
    let model = fixture("defects/unwelded-split-vertices.obj");
    let out = cli(&["repair", &model, "-o", &text(&welded)?])?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let out = cli(&["repair", &model, "--no-weld", "-o", &text(&unwelded)?])?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(vertex_lines(&unwelded)? > vertex_lines(&welded)?);
    let flipped = fixture("defects/flipped-normals.obj");
    let out = cli(&["repair", &flipped, "-o", &text(&dir.join("f.obj"))?])?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let out = cli(&[
        "repair",
        &flipped,
        "--no-fix-orientation",
        "-o",
        &text(&dir.join("g.obj"))?,
    ])?;
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).starts_with("openplush-cli: Mesh: "),
        "{}",
        stderr(&out)
    );
    Ok(())
}

#[test]
fn a_refused_repair_writes_no_plan() -> Result<(), Setup> {
    let dir = scratch("refusal-plan")?;
    let plan = dir.join("p.json");
    let out = cli(&[
        "repair",
        &fixture("defects/hole.obj"),
        "--fill",
        "0.999",
        "--plan-out",
        &text(&plan)?,
        "-o",
        &text(&dir.join("x.obj"))?,
    ])?;
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(!plan.exists(), "a refusal wrote a plan");
    Ok(())
}

#[test]
fn the_obj_writer_writes_each_position_uv_and_corner_exactly() -> Result<(), Setup> {
    let dir = scratch("repair-exact")?;
    let (model, mesh) = (dir.join("sheet.obj"), dir.join("fixed.obj"));
    fs::write(
        &model,
        "v 0.123456789 0 0\nv 3 0 0.5\nv 3 2 0.25\nv 0 2 0\n\
         vt 0.1 0.2\nvt 0.9 0.2\nvt 0.9 0.8\nvt 0.1 0.8\n\
         f 1/1 2/2 3/3\nf 1/1 3/3 4/4\n",
    )?;
    let out = cli(&["repair", &text(&model)?, "-o", &text(&mesh)?])?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let expected = "v 0.123456789 0 0\nv 3 0 0.5\nv 3 2 0.25\nv 0 2 0\n\
                    vt 0.1 0.2\nvt 0.9 0.2\nvt 0.9 0.8\nvt 0.1 0.2\nvt 0.9 0.8\nvt 0.1 0.8\n\
                    f 1/1 2/2 3/3\nf 1/4 3/5 4/6\n";
    assert_eq!(fs::read_to_string(&mesh)?, expected);
    Ok(())
}

#[test]
fn a_file_that_cannot_be_read_or_written_is_named_on_stderr() -> Result<(), Setup> {
    let dir = scratch("io-named")?;
    let missing = dir.join("missing.obj");
    let out = cli(&[
        "validate",
        &text(&missing)?,
        "--json",
        &text(&dir.join("r.json"))?,
    ])?;
    let prefix = format!("openplush-cli: {}: ", missing.display());
    assert!(stderr(&out).starts_with(&prefix), "{}", stderr(&out));
    let nowhere = dir.join("no-such-dir").join("r.json");
    let out = cli(&[
        "validate",
        &fixture("meshes/cube.obj"),
        "--json",
        &text(&nowhere)?,
    ])?;
    let prefix = format!("openplush-cli: {}: ", nowhere.display());
    assert!(stderr(&out).starts_with(&prefix), "{}", stderr(&out));
    Ok(())
}

#[test]
fn a_gltf_named_without_a_directory_reads_the_working_directory() -> Result<(), Setup> {
    let dir = scratch("bare-gltf")?;
    fs::write(dir.join("model.gltf"), "{}")?;
    let out = Command::new(env!("CARGO_BIN_EXE_openplush-cli"))
        .args(["validate", "model.gltf", "--json", "r.json"])
        .current_dir(&*dir)
        .output()?;
    // No glTF reader yet: the set is gathered, and the engine then refuses the type.
    assert!(
        stderr(&out).starts_with("openplush-cli: UnknownFormat: "),
        "{}",
        stderr(&out)
    );
    Ok(())
}
