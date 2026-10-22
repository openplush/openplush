//! `Format`: the import formats, each named.

use openplush_io::Format;

#[test]
fn every_format_has_a_stable_name() {
    let names: Vec<&str> = [
        Format::Obj,
        Format::Gltf,
        Format::Glb,
        Format::Stl,
        Format::Ply,
        Format::Fbx,
    ]
    .iter()
    .map(|f| f.name())
    .collect();
    assert_eq!(names, ["OBJ", "glTF", "GLB", "STL", "PLY", "FBX"]);
}
