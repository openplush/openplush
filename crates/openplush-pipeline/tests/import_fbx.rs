//! FBX through the import stage: the reader its extension names, and what the reader
//! did not apply as one warning.

use openplush_io::fbx;
use openplush_pipeline::{ImportFile, ImportFiles, Remedy, WarningCode, WarningTarget, import};
use openplush_testkit::fbx::{Scene, mesh, model, object};

type Setup = Box<dyn std::error::Error>;

/// Model 1 under the root drawing one triangle (geometry 2).
fn one_triangle() -> Scene {
    let mut scene = Scene::default();
    scene
        .object(model(1, "Tri", Vec::new()))
        .object(mesh(
            2,
            "Tri",
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            vec![0, 1, -3],
        ))
        .connect(1, 0)
        .connect(2, 1);
    scene
}

fn only(name: &str, bytes: Vec<u8>) -> ImportFiles {
    ImportFiles {
        primary: ImportFile {
            name: name.into(),
            bytes,
        },
        others: Vec::new(),
    }
}

#[test]
fn an_fbx_reads_with_the_fbx_reader_whatever_the_extensions_case() -> Result<(), Setup> {
    let bytes = one_triangle().bytes()?;
    let direct = fbx::read(&bytes)?.soup;
    for name in ["tri.fbx", "models/TRI.FBX", "tri.Fbx"] {
        let imported = import(&only(name, bytes.clone()))?;
        assert_eq!(imported.value, direct, "{name}");
        assert_eq!(imported.warnings.iter().count(), 0, "{name}");
    }
    Ok(())
}

#[test]
fn a_deformer_and_an_animation_stack_are_one_import_ignored_warning() -> Result<(), Setup> {
    let mut scene = one_triangle();
    scene
        .object(object("Deformer", 5, "Armature", "Deformer", "Skin"))
        .object(object("AnimationStack", 7, "Take 001", "AnimStack", ""))
        .connect(5, 2);
    let imported = import(&only("tri.fbx", scene.bytes()?))?;
    assert_eq!(imported.value.triangle_count(), 1);
    let warnings: Vec<_> = imported.warnings.iter().collect();
    assert_eq!(warnings.len(), 1);
    let w = warnings[0];
    assert_eq!(w.code(), WarningCode::ImportIgnored);
    assert_eq!(w.target(), WarningTarget::Project);
    assert_eq!(
        w.explanation(),
        "The file has parts the pattern does not follow: the deformer \"Armature\" on model \
         \"Tri\", read in its rest pose; animation stack \"Take 001\", not played. The pattern \
         is made from the triangles as modelled."
    );
    assert!(matches!(w.remedy(), Remedy::Advisory { .. }));
    Ok(())
}
