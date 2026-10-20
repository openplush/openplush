//! glTF and GLB through the import stage: the sidecars the shell supplied, a reference it
//! did not, and what the reader skipped as one warning.

use openplush_io::gltf;
use openplush_pipeline::{
    EngineError, ErrorLocations, ImportFile, ImportFiles, Remedy, WarningCode, WarningTarget,
    import,
};

type Setup = Box<dyn std::error::Error>;

const FACETED: &[u8] = include_bytes!("../../../fixtures/meshes/faceted.glb");

fn file(name: &str, bytes: &[u8]) -> ImportFile {
    ImportFile {
        name: name.into(),
        bytes: bytes.to_vec(),
    }
}

/// One triangle whose buffer is the file `uri` beside it; `primitives` is the mesh's
/// primitive list, with accessor 0 the positions.
fn gltf_beside(uri: &str, primitives: &str) -> (String, Vec<u8>) {
    let points = [0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let bin: Vec<u8> = points.iter().flat_map(|c| c.to_le_bytes()).collect();
    let json = format!(
        r#"{{"asset":{{"version":"2.0"}},"buffers":[{{"uri":"{uri}","byteLength":{len}}}],"bufferViews":[{{"buffer":0,"byteLength":{len}}}],"accessors":[{{"bufferView":0,"componentType":5126,"type":"VEC3","count":3,"min":[0,0,0],"max":[1,1,0]}}],"meshes":[{{"primitives":[{primitives}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0}}"#,
        len = bin.len()
    );
    (json, bin)
}

const ONE_TRIANGLE: &str = r#"{"attributes":{"POSITION":0}}"#;

#[test]
fn a_glb_reads_through_the_import_stage() -> Result<(), Setup> {
    let imported = import(&ImportFiles {
        primary: file("faceted.glb", FACETED),
        others: Vec::new(),
    })?;
    let direct = gltf::read("faceted.glb", FACETED, &|_| None)?;
    assert_eq!(imported.value, direct.soup);
    assert_eq!(imported.warnings.iter().count(), 0);
    Ok(())
}

#[test]
fn a_gltf_reads_its_buffer_from_the_supplied_files() -> Result<(), Setup> {
    let (json, bin) = gltf_beside("tri.bin", ONE_TRIANGLE);
    let imported = import(&ImportFiles {
        primary: file("scans/tri.GLTF", json.as_bytes()),
        others: vec![file("scans/other.bin", &[]), file("scans/tri.bin", &bin)],
    })?;
    assert_eq!(imported.value.triangle_count(), 1);
    Ok(())
}

#[test]
fn a_buffer_the_shell_did_not_supply_is_a_missing_resource() {
    let (json, _) = gltf_beside("tri.bin", ONE_TRIANGLE);
    let refused = import(&ImportFiles {
        primary: file("tri.gltf", json.as_bytes()),
        others: vec![file("elsewhere/tri.bin", &[])],
    });
    assert_eq!(
        refused,
        Err(EngineError::MissingResource {
            uri: "tri.bin".into()
        })
    );
}

#[test]
fn missing_resource_names_the_reference_and_locates_nothing() {
    let e = EngineError::MissingResource {
        uri: "tri.bin".into(),
    };
    assert_eq!(
        e.to_string(),
        "the file refers to \"tri.bin\", which was not supplied beside it"
    );
    assert_eq!(e.code(), "MissingResource");
    assert_eq!(e.locations(), ErrorLocations::default());
    assert!(e.proposals().is_empty());
}

#[test]
fn what_the_reader_skipped_is_one_import_ignored_warning() -> Result<(), Setup> {
    let primitives = format!(r#"{{"attributes":{{"POSITION":0}},"mode":0}},{ONE_TRIANGLE}"#);
    let (json, bin) = gltf_beside("tri.bin", &primitives);
    let imported = import(&ImportFiles {
        primary: file("tri.gltf", json.as_bytes()),
        others: vec![file("tri.bin", &bin)],
    })?;
    assert_eq!(imported.value.triangle_count(), 1);
    let warnings: Vec<_> = imported.warnings.iter().collect();
    assert_eq!(warnings.len(), 1);
    let w = warnings[0];
    assert_eq!(w.code(), WarningCode::ImportIgnored);
    assert_eq!(w.code().as_str(), "import-ignored");
    assert_eq!(w.target(), WarningTarget::Project);
    assert_eq!(
        w.explanation(),
        "The file has parts the pattern does not follow: primitive 0 of mesh 0 (node 0), \
         which is points or lines. The pattern is made from the triangles as modelled."
    );
    assert!(matches!(w.remedy(), Remedy::Advisory { .. }));
    Ok(())
}
