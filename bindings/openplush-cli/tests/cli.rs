//! The CLI surface parses as documented and fails loudly where a stage is missing.

use std::process::{Command, Output};

fn cli(args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_openplush-cli"))
        .args(args)
        .output()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn help_lists_every_subcommand() {
    let out = cli(&["--help"]).expect("the binary runs");
    assert!(out.status.success());
    let help = String::from_utf8_lossy(&out.stdout);
    for sub in ["validate", "repair", "pattern"] {
        assert!(help.contains(sub), "--help omits {sub}:\n{help}");
    }
}

#[test]
fn a_missing_required_argument_is_a_usage_error() {
    let out = cli(&["pattern", "model.obj"]).expect("the binary runs");
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stderr(&out).contains("--seams"), "{}", stderr(&out));
}

#[test]
fn an_unknown_page_size_is_a_usage_error() {
    let out = cli(&[
        "pattern",
        "m.obj",
        "--seams",
        "s.json",
        "--fabric",
        "minky",
        "--size-mm",
        "300",
        "--page",
        "a3",
        "-o",
        "out",
    ])
    .expect("the binary runs");
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
}

#[test]
fn a_non_positive_size_is_a_usage_error() {
    for size in ["0", "-5", "NaN", "inf", "1e309"] {
        let flag = format!("--size-mm={size}");
        let out = cli(&[
            "pattern", "m.obj", "--seams", "s.json", "--fabric", "minky", &flag, "--page", "a4",
            "-o", "out",
        ])
        .expect("the binary runs");
        assert_eq!(out.status.code(), Some(2), "size {size}: {}", stderr(&out));
        assert!(
            stderr(&out).contains("not a finite length above 0 mm"),
            "size {size} was not refused by the value check: {}",
            stderr(&out)
        );
    }
}

#[test]
fn pattern_fails_loudly_until_its_stage_exists() {
    let out = cli(&[
        "pattern",
        "m.obj",
        "--seams",
        "s.json",
        "--fabric",
        "minky",
        "--size-mm",
        "300",
        "--page",
        "a4",
        "-o",
        "out",
    ])
    .expect("the binary runs");
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("not built yet"), "{}", stderr(&out));
    assert!(out.stdout.is_empty());
}

#[test]
fn repair_s_old_flags_are_gone() {
    for flag in ["--weld", "--fix-orientation", "--fill-holes=auto"] {
        let out = cli(&["repair", "m.obj", flag, "-o", "fixed.obj"]).expect("the binary runs");
        assert_eq!(out.status.code(), Some(2), "{flag}: {}", stderr(&out));
    }
}

#[test]
fn a_fill_that_is_not_a_loop_ref_is_a_usage_error() {
    for text in ["12", "0.12.3", "0.012", "+0.12", "0.-1", "a.b", ""] {
        let flag = format!("--fill={text}");
        let out = cli(&["repair", "m.obj", &flag, "-o", "fixed.obj"]).expect("the binary runs");
        assert_eq!(out.status.code(), Some(2), "{text:?}: {}", stderr(&out));
    }
}
