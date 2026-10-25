//! `pattern` runs a model through every stage with a seam-set file and a fabric file, and writes
//! `pattern.svg` and `pattern.pdf` into its output directory; a bad seam or fabric file, or an
//! engine refusal, writes nothing and exits 1.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type Setup = Box<dyn std::error::Error>;

fn fixture(path: &str) -> String {
    format!("{}/../../fixtures/{path}", env!("CARGO_MANIFEST_DIR"))
}

/// A fresh directory for one test's files, removed when the test ends.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn scratch(test: &str) -> Result<Scratch, Setup> {
    let dir = std::env::temp_dir().join(format!(
        "openplush-cli-pattern-{}-{test}",
        std::process::id()
    ));
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

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// One run's inputs: the capsule and its reference seams and fabric unless a test swaps one.
struct Run {
    model: String,
    seams: String,
    fabric: String,
    size_mm: String,
    page: Option<&'static str>,
}

impl Run {
    fn capsule() -> Self {
        Self {
            model: fixture("meshes/capsule.obj"),
            seams: fixture("seams/capsule.json"),
            fabric: fixture("fabrics/woven.json"),
            size_mm: "200".into(),
            page: Some("a4"),
        }
    }

    fn write_to(&self, out: &Path) -> Result<Output, Setup> {
        let mut args = vec![
            "pattern".to_owned(),
            self.model.clone(),
            "--seams".into(),
            self.seams.clone(),
            "--fabric".into(),
            self.fabric.clone(),
            "--size-mm".into(),
            self.size_mm.clone(),
            "-o".into(),
            text(out)?,
        ];
        if let Some(page) = self.page {
            args.extend(["--page".to_owned(), page.to_owned()]);
        }
        Ok(Command::new(env!("CARGO_BIN_EXE_openplush-cli"))
            .args(args)
            .output()?)
    }
}

/// Writes `contents` as `name` in `dir` and returns its path as text.
fn file(dir: &Path, name: &str, contents: &str) -> Result<String, Setup> {
    let path = dir.join(name);
    fs::write(&path, contents)?;
    text(&path)
}

fn listing(dir: &Path) -> Result<Vec<String>, Setup> {
    let mut names = fs::read_dir(dir)?
        .map(|e| Ok(e?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    names.sort();
    Ok(names)
}

#[test]
fn the_capsule_becomes_an_svg_and_a_pdf_in_a_new_directory() -> Result<(), Setup> {
    let dir = scratch("capsule")?;
    let out_dir = dir.0.join("out").join("nested");
    let out = Run::capsule().write_to(&out_dir)?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(listing(&out_dir)?, ["pattern.pdf", "pattern.svg"]);
    let svg = fs::read_to_string(out_dir.join("pattern.svg"))?;
    assert!(
        svg.contains("<svg ") && svg.contains("mm\" height=\""),
        "{svg}"
    );
    assert!(fs::read(out_dir.join("pattern.pdf"))?.starts_with(b"%PDF-"));
    Ok(())
}

type Outline = Vec<(f64, f64)>;

/// The points of every `<path class="{class}" d="M x y L x y … Z">` in `svg`, in document order.
fn outlines(svg: &str, class: &str) -> Result<Vec<Outline>, Setup> {
    let open = format!("<path class=\"{class}\" d=\"");
    let mut found = Vec::new();
    for rest in svg.split(open.as_str()).skip(1) {
        let d = rest.split('"').next().ok_or("an unclosed path")?;
        let numbers = d
            .split_whitespace()
            .filter(|t| !matches!(*t, "M" | "L" | "Z"))
            .map(str::parse::<f64>)
            .collect::<Result<Vec<_>, _>>()?;
        found.push(numbers.chunks_exact(2).map(|p| (p[0], p[1])).collect());
    }
    Ok(found)
}

fn size(outline: &Outline) -> (f64, f64) {
    let (xs, ys): (Vec<f64>, Vec<f64>) = outline.iter().copied().unzip();
    let span = |v: &[f64]| {
        v.iter().copied().fold(f64::MIN, f64::max) - v.iter().copied().fold(f64::MAX, f64::min)
    };
    (span(&xs), span(&ys))
}

fn perimeter(outline: &Outline) -> f64 {
    let next = outline.iter().cycle().skip(1);
    outline
        .iter()
        .zip(next)
        .map(|(a, b)| {
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            (dx * dx + dy * dy).sqrt()
        })
        .sum()
}

fn assert_mm(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() <= 0.05,
        "{what}: {got:.3} mm, want {want:.2} mm"
    );
}

fn assert_size(got: (f64, f64), want: (f64, f64), what: &str) {
    let got = if got.0 < got.1 { got } else { (got.1, got.0) };
    assert_mm(got.0, want.0, what);
    assert_mm(got.1, want.1, what);
}

// At 240 mm the capsule (120 model units long) scales by 2. The tube is its 12-sided ring of
// radius 30 unrolled: 12 sides of 2·30·sin(15°) units, ×2, is 372.70 mm; its 60 units are 120 mm.
#[test]
fn the_capsule_s_pieces_measure_what_its_shape_and_the_allowance_give() -> Result<(), Setup> {
    let dir = scratch("measure")?;
    let out = Run {
        size_mm: "240".into(),
        ..Run::capsule()
    }
    .write_to(&dir.0)?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let svg = fs::read_to_string(dir.0.join("pattern.svg"))?;
    let (sew, cut) = (outlines(&svg, "sew")?, outlines(&svg, "cut")?);
    // The two end caps are one template cut twice, so the print has two pieces.
    assert_eq!((sew.len(), cut.len()), (2, 2), "{svg}");
    assert!(
        svg.contains(">Cut 2<"),
        "the caps' template is cut twice: {svg}"
    );
    let tube = (0..2)
        .max_by(|&a, &b| perimeter(&sew[a]).total_cmp(&perimeter(&sew[b])))
        .ok_or("no pieces")?;
    assert_size(size(&sew[tube]), (120.0, 372.70), "tube sewing line");
    assert_size(size(&cut[tube]), (132.0, 384.70), "tube cut line");
    for cap in (0..2).filter(|&i| i != tube) {
        assert_mm(perimeter(&sew[cap]), 372.70, "cap sewing line");
    }
    Ok(())
}

#[test]
fn the_same_inputs_give_the_same_bytes() -> Result<(), Setup> {
    let dir = scratch("same")?;
    let (a, b) = (dir.0.join("a"), dir.0.join("b"));
    for out_dir in [&a, &b] {
        let out = Run::capsule().write_to(out_dir)?;
        assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    }
    for name in ["pattern.svg", "pattern.pdf"] {
        assert_eq!(fs::read(a.join(name))?, fs::read(b.join(name))?, "{name}");
    }
    Ok(())
}

#[test]
fn the_page_is_a4_unless_letter_is_asked_for() -> Result<(), Setup> {
    let dir = scratch("page")?;
    // A4 is 210 by 297 mm and Letter 8.5 by 11 in, at 72 pt to the inch.
    let a4_box = "/MediaBox [0 0 595.2756 841.8898]";
    let letter_box = "/MediaBox [0 0 612 792]";
    for (page, letter) in [(None, false), (Some("letter"), true)] {
        let out_dir = dir.0.join(page.unwrap_or("default"));
        let run = Run {
            page,
            ..Run::capsule()
        };
        let out = run.write_to(&out_dir)?;
        assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
        let pdf = String::from_utf8_lossy(&fs::read(out_dir.join("pattern.pdf"))?).into_owned();
        assert_eq!(pdf.contains(letter_box), letter, "{page:?}");
        assert_eq!(pdf.contains(a4_box), !letter, "{page:?}");
    }
    Ok(())
}

#[test]
fn warnings_print_in_the_files_and_do_not_stop_the_run() -> Result<(), Setup> {
    let dir = scratch("warned")?;
    let run = Run {
        size_mm: "50".into(),
        ..Run::capsule()
    };
    let out = run.write_to(&dir.0)?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let svg = fs::read_to_string(dir.0.join("pattern.svg"))?;
    // Both end caps are squeezed; the twin cap prints as piece 0, so its warning is piece 0's.
    for row in [
        "1. piece-compressed: piece 0",
        "2. size-out-of-range: the whole pattern",
    ] {
        assert!(svg.contains(&format!(">{row}<")), "{row}: {svg}");
    }
    assert!(!svg.contains(">3. "), "{svg}");
    let pdf = String::from_utf8_lossy(&fs::read(dir.0.join("pattern.pdf"))?).into_owned();
    assert!(pdf.contains("(2. size-out-of-range: the whole pattern) Tj"));
    Ok(())
}

#[test]
fn a_one_based_seam_set_names_the_same_edges() -> Result<(), Setup> {
    let dir = scratch("one-based")?;
    let zero = fs::read_to_string(fixture("seams/capsule.json"))?;
    let mut set: serde_json::Value = serde_json::from_str(&zero)?;
    set["vertexIndexBase"] = 1.into();
    for seam in set["seams"].as_array_mut().ok_or("no seams")? {
        for i in seam["edge"].as_array_mut().ok_or("no edge")? {
            *i = (i.as_u64().ok_or("not an index")? + 1).into();
        }
    }
    let one = file(&dir.0, "one.json", &set.to_string())?;
    let (a, b) = (dir.0.join("zero"), dir.0.join("one"));
    assert_eq!(Run::capsule().write_to(&a)?.status.code(), Some(0));
    let run = Run {
        seams: one,
        ..Run::capsule()
    };
    let out = run.write_to(&b)?;
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(
        fs::read(a.join("pattern.svg"))?,
        fs::read(b.join("pattern.svg"))?
    );
    Ok(())
}

/// Runs the capsule with `seams` as its seam-set file and returns stderr, after checking that it
/// exited 1 and wrote nothing.
fn refused_seams(test: &str, seams: &str) -> Result<String, Setup> {
    let dir = scratch(test)?;
    let path = file(&dir.0, "seams.json", seams)?;
    let out_dir = dir.0.join("out");
    let out = Run {
        seams: path,
        ..Run::capsule()
    }
    .write_to(&out_dir)?;
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(!out_dir.exists(), "{test} wrote output");
    Ok(stderr(&out))
}

fn seam_set(seams: &str) -> String {
    format!(
        r#"{{"format": "openplush-seam-set", "version": 1, "vertexIndexBase": 0, "seams": [{seams}]}}"#
    )
}

#[test]
fn a_seam_on_a_vertex_the_model_lacks_is_refused_naming_the_seam() -> Result<(), Setup> {
    let err = refused_seams(
        "missing",
        &seam_set(r#"{"edge": [25, 26], "kind": "cut"}, {"edge": [0, 9999], "kind": "cut"}"#),
    )?;
    assert!(err.contains("seam 1: no vertex 9999"), "{err}");
    Ok(())
}

#[test]
fn a_seam_between_vertices_with_no_edge_is_refused_naming_the_seam() -> Result<(), Setup> {
    // The capsule's two poles are far apart.
    let err = refused_seams("no-edge", &seam_set(r#"{"edge": [0, 97], "kind": "cut"}"#))?;
    assert!(
        err.contains("seam 0: vertices 0 and 97 share no edge"),
        "{err}"
    );
    Ok(())
}

#[test]
fn a_malformed_seam_set_is_refused() -> Result<(), Setup> {
    for (case, contents, says) in [
        (
            "format",
            r#"{"format": "x", "version": 1, "vertexIndexBase": 0, "seams": []}"#,
            "format",
        ),
        (
            "version",
            r#"{"format": "openplush-seam-set", "version": 2, "vertexIndexBase": 0, "seams": []}"#,
            "version",
        ),
        (
            "base",
            r#"{"format": "openplush-seam-set", "version": 1, "vertexIndexBase": 2, "seams": []}"#,
            "vertexIndexBase",
        ),
        (
            "float",
            &seam_set(r#"{"edge": [25.0, 26], "kind": "cut"}"#),
            "seam 0",
        ),
        (
            "kind",
            &seam_set(r#"{"edge": [25, 26], "kind": "zip"}"#),
            "seam 0",
        ),
        ("json", "{", "seams.json"),
    ] {
        let err = refused_seams(case, contents)?;
        assert!(err.contains(says), "{case}: {err}");
    }
    Ok(())
}

/// Runs the capsule with `fabric` as its fabric file and returns its exit code and stderr.
fn with_fabric(test: &str, fabric: &str) -> Result<(Option<i32>, String), Setup> {
    let dir = scratch(test)?;
    let path = file(&dir.0, "fabric.json", fabric)?;
    let out_dir = dir.0.join("out");
    let out = Run {
        fabric: path,
        ..Run::capsule()
    }
    .write_to(&out_dir)?;
    if out.status.code() != Some(0) {
        assert!(!out_dir.exists(), "{test} wrote output");
    }
    Ok((out.status.code(), stderr(&out)))
}

#[test]
fn a_fabric_with_only_its_required_fields_takes_the_schema_s_defaults() -> Result<(), Setup> {
    let (code, err) = with_fabric(
        "minimal",
        r#"{"id": "f", "name": "Felt", "category": "felt"}"#,
    )?;
    assert_eq!(code, Some(0), "{err}");
    Ok(())
}

#[test]
fn a_fabric_file_the_schema_or_the_engine_refuses_is_refused() -> Result<(), Setup> {
    for (case, contents, says) in [
        (
            "unknown",
            r#"{"id": "f", "name": "F", "category": "felt", "colour": "red"}"#,
            "colour",
        ),
        ("no-name", r#"{"id": "f", "category": "felt"}"#, "name"),
        (
            "category",
            r#"{"id": "f", "name": "F", "category": "silk"}"#,
            "category",
        ),
        (
            "negative",
            r#"{"id": "f", "name": "F", "category": "felt", "defaultAllowanceMm": -1}"#,
            "defaultAllowanceMm",
        ),
        (
            "roll",
            r#"{"id": "f", "name": "F", "category": "felt", "rollWidthMm": 0}"#,
            "rollWidthMm",
        ),
        (
            "type",
            r#"{"id": "f", "name": "F", "category": "felt", "nap": "yes"}"#,
            "nap",
        ),
    ] {
        let (code, err) = with_fabric(case, contents)?;
        assert_eq!(code, Some(1), "{case}: {err}");
        assert!(
            err.contains("fabric.json") && err.contains(says),
            "{case}: {err}"
        );
    }
    Ok(())
}

#[test]
fn an_engine_refusal_prints_its_code_and_writes_nothing() -> Result<(), Setup> {
    let dir = scratch("engine")?;
    let out_dir = dir.0.join("out");
    let run = Run {
        model: fixture("meshes/source-seams/sphere-band.obj"),
        seams: fixture("seams/non-disk/sphere-band.json"),
        ..Run::capsule()
    };
    let out = run.write_to(&out_dir)?;
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).starts_with("openplush-cli: ChartNotDisk: "),
        "{}",
        stderr(&out)
    );
    assert!(!out_dir.exists());
    Ok(())
}

fn one_based(seams: &str) -> String {
    format!(
        r#"{{"format": "openplush-seam-set", "version": 1, "vertexIndexBase": 1, "seams": [{seams}]}}"#
    )
}

#[test]
fn a_one_based_file_is_refused_in_its_own_numbering() -> Result<(), Setup> {
    for (case, seams, says) in [
        (
            "below",
            r#"{"edge": [0, 1], "kind": "cut"}"#,
            "seam 0: an index is below the file's base",
        ),
        (
            "missing-1",
            r#"{"edge": [1, 10000], "kind": "cut"}"#,
            "seam 0: no vertex 10000",
        ),
        (
            "no-edge-1",
            r#"{"edge": [1, 98], "kind": "cut"}"#,
            "seam 0: vertices 1 and 98 share no edge",
        ),
    ] {
        let err = refused_seams(case, &one_based(seams))?;
        assert!(err.contains(says), "{case}: {err}");
    }
    Ok(())
}

#[test]
fn a_kind_the_seam_set_file_cannot_carry_is_refused_naming_the_seam() -> Result<(), Setup> {
    for kind in ["dart", "opening", "mandatory"] {
        let capsule = fs::read_to_string(fixture("seams/capsule.json"))?;
        let mut set: serde_json::Value = serde_json::from_str(&capsule)?;
        let seams = set["seams"].as_array_mut().ok_or("no seams")?;
        seams.insert(0, serde_json::json!({"edge": [25, 26], "kind": kind}));
        let err = refused_seams(kind, &set.to_string())?;
        let says = format!("seam 0: \"kind\" \"{kind}\" is not supported in a seam-set file");
        assert!(err.contains(&says), "{kind}: {err}");
    }
    Ok(())
}

#[test]
fn a_source_vertex_repair_split_in_two_is_refused() -> Result<(), Setup> {
    let dir = scratch("split")?;
    // Two triangles meeting only at the first vertex: repair splits that bowtie.
    let model = file(
        &dir.0,
        "bowtie.obj",
        "v 0 0 0\nv 10 0 0\nv 10 10 0\nv -10 0 0\nv -10 -10 0\nf 1 2 3\nf 1 4 5\n",
    )?;
    let seams = file(
        &dir.0,
        "seams.json",
        &seam_set(r#"{"edge": [0, 1], "kind": "cut"}"#),
    )?;
    let out_dir = dir.0.join("out");
    let out = Run {
        model,
        seams,
        ..Run::capsule()
    }
    .write_to(&out_dir)?;
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("seam 0: vertex 0 is more than one vertex of the repaired model"),
        "{}",
        stderr(&out)
    );
    assert!(!out_dir.exists());
    Ok(())
}

#[test]
fn a_refusal_after_the_pattern_stage_writes_nothing() -> Result<(), Setup> {
    let (code, err) = with_fabric(
        "narrow",
        r#"{"id": "f", "name": "F", "category": "felt", "rollWidthMm": 50}"#,
    )?;
    assert_eq!(code, Some(1), "{err}");
    assert!(err.starts_with("openplush-cli: Layout: "), "{err}");
    Ok(())
}

#[test]
fn a_seam_repair_forces_is_sewn_even_where_it_leaves_no_disk() -> Result<(), Setup> {
    let dir = scratch("fin")?;
    // An octahedron with a fin on its edge from +x to +y: repair splits that edge, so the body
    // keeps a slit there that must be sewn to the fin.
    let model = file(
        &dir.0,
        "fin.obj",
        "v 10 0 0\nv -10 0 0\nv 0 10 0\nv 0 -10 0\nv 0 0 10\nv 0 0 -10\nv 10 10 0\n\
         f 1 3 5\nf 3 2 5\nf 2 4 5\nf 4 1 5\nf 3 1 6\nf 2 3 6\nf 4 2 6\nf 1 4 6\nf 1 3 7\n",
    )?;
    // A cut from +z through -x to -z opens the body; with the slit as well it is a ring.
    let seams = file(
        &dir.0,
        "seams.json",
        &seam_set(r#"{"edge": [4, 1], "kind": "cut"}, {"edge": [1, 5], "kind": "cut"}"#),
    )?;
    let out_dir = dir.0.join("out");
    let out = Run {
        model,
        seams,
        ..Run::capsule()
    }
    .write_to(&out_dir)?;
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).starts_with("openplush-cli: ChartNotDisk: "),
        "{}",
        stderr(&out)
    );
    assert!(!out_dir.exists());
    Ok(())
}
