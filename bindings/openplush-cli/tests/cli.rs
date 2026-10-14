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
    for size in ["0", "-5", "NaN", "inf"] {
        let out = cli(&[
            "pattern",
            "m.obj",
            "--seams",
            "s.json",
            "--fabric",
            "minky",
            "--size-mm",
            size,
            "--page",
            "a4",
            "-o",
            "out",
        ])
        .expect("the binary runs");
        assert_eq!(out.status.code(), Some(2), "size {size}: {}", stderr(&out));
    }
}

#[test]
fn every_valid_invocation_fails_loudly_until_its_stage_exists() {
    let invocations: [&[&str]; 3] = [
        &["validate", "m.obj", "--json", "report.json"],
        &[
            "repair",
            "m.obj",
            "--weld",
            "--fix-orientation",
            "--fill-holes",
            "auto",
            "--plan-out",
            "plan.json",
            "-o",
            "fixed.obj",
        ],
        &[
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
        ],
    ];
    for args in invocations {
        let out = cli(args).expect("the binary runs");
        assert_eq!(out.status.code(), Some(1), "{args:?}: {}", stderr(&out));
        assert!(
            stderr(&out).contains("not built yet"),
            "{args:?}: {}",
            stderr(&out)
        );
        assert!(out.stdout.is_empty(), "{args:?} wrote to stdout");
    }
}
