//! Which files go into the import set beside the model, and how they are named.
#![allow(clippy::expect_used, reason = "test setup")]

use std::fs;
use std::path::PathBuf;

use super::{CAP, files};
use crate::failure::Failure;

fn scratch(test: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("openplush-cli-unit-{}-{test}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear");
    }
    fs::create_dir_all(dir.join("sub")).expect("scratch dir");
    dir
}

fn names(dir: &std::path::Path, model: &str, cap: u64) -> Vec<String> {
    let set = files(&dir.join(model), cap).expect("the set");
    set.others.into_iter().map(|f| f.name).collect()
}

#[test]
fn a_model_that_needs_no_other_file_is_read_alone() {
    let dir = scratch("alone");
    for model in ["m.obj", "m.STL", "m.ply", "m.glb"] {
        fs::write(dir.join(model), "x").expect("write");
        fs::write(dir.join("other.bin"), "y").expect("write");
        let set = files(&dir.join(model), CAP).expect("the set");
        assert_eq!(set.primary.name, model);
        assert_eq!(set.primary.bytes, b"x");
        assert!(set.others.is_empty(), "{model}");
    }
}

#[test]
fn a_gltf_takes_every_file_below_its_directory_named_relative_in_byte_order() {
    let dir = scratch("gltf");
    fs::write(dir.join("model.gltf"), "{}").expect("write");
    fs::write(dir.join("b.bin"), "1").expect("write");
    fs::write(dir.join("B.bin"), "2").expect("write");
    fs::write(dir.join("sub").join("a.png"), "3").expect("write");
    assert_eq!(
        names(&dir, "model.gltf", CAP),
        ["B.bin", "b.bin", "sub/a.png"]
    );
    let set = files(&dir.join("model.gltf"), CAP).expect("the set");
    assert_eq!(set.others[1].bytes, b"1");
}

#[test]
fn a_gltf_s_extension_is_matched_ignoring_case() {
    let dir = scratch("case");
    fs::write(dir.join("model.GLTF"), "{}").expect("write");
    fs::write(dir.join("a.bin"), "1").expect("write");
    assert_eq!(names(&dir, "model.GLTF", CAP), ["a.bin"]);
}

#[cfg(unix)]
#[test]
fn symlinks_are_not_followed() {
    let dir = scratch("links");
    fs::write(dir.join("model.gltf"), "{}").expect("write");
    fs::write(dir.join("sub").join("real.bin"), "1").expect("write");
    std::os::unix::fs::symlink(dir.join("sub").join("real.bin"), dir.join("link.bin"))
        .expect("file link");
    std::os::unix::fs::symlink(dir.join("sub"), dir.join("linked-dir")).expect("dir link");
    assert_eq!(names(&dir, "model.gltf", CAP), ["sub/real.bin"]);
}

#[test]
fn a_set_over_the_cap_is_refused() {
    let dir = scratch("cap");
    fs::write(dir.join("model.gltf"), "{}").expect("write");
    fs::write(dir.join("big.bin"), [0u8; 64]).expect("write");
    assert!(matches!(
        files(&dir.join("model.gltf"), 32),
        Err(Failure::TooLarge { .. })
    ));
    assert_eq!(names(&dir, "model.gltf", 66), ["big.bin"]);
}

#[test]
fn a_model_that_cannot_be_read_is_an_io_failure() {
    let dir = scratch("missing");
    assert!(matches!(
        files(&dir.join("missing.obj"), CAP),
        Err(Failure::Io { .. })
    ));
}
