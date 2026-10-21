//! Which files go into the import set beside the model, and how they are named.
#![allow(clippy::expect_used, reason = "test setup")]

use std::fs;
use std::path::{Path, PathBuf};

use super::{CAP, files};
use crate::failure::Failure;

/// A fresh directory with a `sub` directory, removed when the test ends.
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

fn scratch(test: &str) -> Scratch {
    let dir =
        std::env::temp_dir().join(format!("openplush-cli-unit-{}-{test}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear");
    }
    fs::create_dir_all(dir.join("sub")).expect("scratch dir");
    Scratch(dir)
}

fn names(dir: &Path, model: &str, cap: u64) -> Vec<String> {
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

#[test]
fn the_limit_is_one_gibibyte() {
    assert_eq!(CAP, 1 << 30);
}

#[test]
fn the_cap_counts_the_model_and_every_file_together() {
    let dir = scratch("total");
    fs::write(dir.join("model.gltf"), "{}").expect("write");
    fs::write(dir.join("a.bin"), [0u8; 20]).expect("write");
    fs::write(dir.join("b.bin"), [0u8; 20]).expect("write");
    let refused = files(&dir.join("model.gltf"), 41).expect_err("over the cap");
    assert_eq!(
        refused.to_string(),
        format!(
            "{} and the files beside it are 42 bytes, over the 41-byte limit",
            dir.join("model.gltf").display()
        )
    );
    assert_eq!(names(&dir, "model.gltf", 42), ["a.bin", "b.bin"]);
}

#[test]
fn a_file_past_the_cap_is_refused_before_it_is_read() {
    let dir = scratch("sparse");
    fs::write(dir.join("model.gltf"), "{}").expect("write");
    // Sparse, so it takes no disk; reading it would need a terabyte of memory.
    fs::File::create(dir.join("huge.bin"))
        .and_then(|f| f.set_len(1 << 40))
        .expect("sparse file");
    assert!(matches!(
        files(&dir.join("model.gltf"), CAP),
        Err(Failure::TooLarge { .. })
    ));
}

#[test]
fn names_sort_by_their_bytes_whatever_order_the_directory_lists() {
    let dir = scratch("order");
    fs::write(dir.join("model.gltf"), "{}").expect("write");
    let mut expected = Vec::new();
    for i in [
        7, 3, 19, 0, 12, 5, 16, 1, 9, 14, 2, 18, 6, 11, 4, 17, 8, 13, 10, 15,
    ] {
        let name = format!("{}{i}.bin", if i % 2 == 0 { "a" } else { "B" });
        fs::write(dir.join(&name), "x").expect("write");
        expected.push(name);
    }
    expected.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    assert_eq!(names(&dir, "model.gltf", CAP), expected);
}

#[test]
fn a_nested_file_is_named_by_its_whole_path_and_only_the_model_itself_is_left_out() {
    let dir = scratch("nested");
    fs::write(dir.join("model.gltf"), "{}").expect("write");
    fs::create_dir_all(dir.join("sub").join("deeper")).expect("dirs");
    fs::write(dir.join("sub").join("deeper").join("x.bin"), "1").expect("write");
    fs::write(dir.join("sub").join("model.gltf"), "{}").expect("write");
    assert_eq!(
        names(&dir, "model.gltf", CAP),
        ["sub/deeper/x.bin", "sub/model.gltf"]
    );
}

#[cfg(unix)]
#[test]
fn a_file_name_that_is_not_utf8_is_refused_rather_than_renamed() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let dir = scratch("utf8");
    fs::write(dir.join("model.gltf"), "{}").expect("write");
    let odd = dir.join(OsStr::from_bytes(b"\xff.bin"));
    fs::write(&odd, "1").expect("write");
    let refused = files(&dir.join("model.gltf"), CAP).expect_err("a non-UTF-8 neighbour");
    assert!(
        matches!(&refused, Failure::Name { path } if *path == odd),
        "{refused:?}"
    );
    assert_eq!(
        refused.to_string(),
        format!("{}: the file name is not UTF-8", odd.display())
    );
    let model = dir.join(OsStr::from_bytes(b"\xfe.obj"));
    fs::write(&model, "v 0 0 0\n").expect("write");
    assert!(matches!(
        files(&model, CAP),
        Err(Failure::Name { path }) if path == model
    ));
}
