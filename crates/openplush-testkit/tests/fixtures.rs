//! Fixtures load by kind and name; anything else is a typed error, never a panic.

use openplush_testkit::fixtures::{FixtureError, Kind, load, names};

#[test]
fn a_mesh_loads_by_name() {
    let bytes = load(Kind::Mesh, "capsule").expect("capsule.obj is committed");
    let text = std::str::from_utf8(&bytes).expect("OBJ is text");
    assert!(text.lines().any(|l| l.starts_with("v ")));
    assert!(text.lines().any(|l| l.starts_with("f ")));
}

#[test]
fn seams_load_by_their_mesh_name() {
    let bytes = load(Kind::Seams, "cube").expect("cube.json is committed");
    assert!(bytes.starts_with(b"{"));
}

#[test]
fn every_defect_class_is_listed_in_name_order() {
    let defects = names(Kind::Defect).expect("fixtures/defects exists");
    assert_eq!(
        defects,
        [
            "bowtie-vertex",
            "duplicate-face",
            "flipped-normals",
            "hole",
            "multi-component",
            "non-manifold-edge",
            "non-orientable",
            "opposite-duplicate-face",
            "repeated-index-face",
            "self-intersection",
            "unwelded-split-vertices",
            "zero-area-face",
        ]
    );
    for name in &defects {
        load(Kind::Defect, name).expect("every listed defect loads");
    }
}

#[test]
fn mesh_names_are_sorted_and_carry_no_extension() {
    assert_eq!(
        names(Kind::Mesh).expect("fixtures/meshes exists"),
        ["capsule", "cube", "sphere"]
    );
}

#[test]
fn an_unknown_name_is_not_found() {
    match load(Kind::Mesh, "teapot") {
        Err(FixtureError::NotFound { path, .. }) => {
            assert!(path.ends_with("meshes/teapot.obj"), "{}", path.display());
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn a_name_that_could_leave_its_directory_is_refused() {
    for bad in [
        "",
        "../meshes/cube",
        "sub/cube",
        "cube.obj",
        ".hidden",
        "a\\b",
    ] {
        assert!(
            matches!(load(Kind::Mesh, bad), Err(FixtureError::BadName { .. })),
            "{bad:?} was not refused"
        );
    }
}

#[test]
fn a_bad_name_message_shows_each_refused_character_once() {
    let err = load(Kind::Mesh, "a/b").expect_err("a slash is refused");
    assert!(err.to_string().contains(r"'/', '\', '.'"), "{err}");
}
