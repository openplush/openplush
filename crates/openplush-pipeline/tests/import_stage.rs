//! The import stage: the primary file's extension picks the reader, and each reader refusal
//! arrives as the `EngineError` of the same name.

use openplush_io::{obj, ply, stl};
use openplush_pipeline::{
    EngineError, ErrorLocations, Format, ImportFile, ImportFiles, SourceCorner, import,
};

const TRIANGLE_OBJ: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

const TRIANGLE_STL: &str = "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\n\
vertex 0 1 0\nendloop\nendfacet\nendsolid t\n";

const TRIANGLE_PLY: &str = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\n\
property float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\n\
end_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n";

fn file(name: &str, text: &str) -> ImportFile {
    ImportFile {
        name: name.into(),
        bytes: text.as_bytes().to_vec(),
    }
}

fn only(name: &str, text: &str) -> ImportFiles {
    ImportFiles {
        primary: file(name, text),
        others: Vec::new(),
    }
}

#[test]
fn each_extension_reads_with_its_own_reader() -> Result<(), Box<dyn std::error::Error>> {
    let read = import(&only("tri.obj", TRIANGLE_OBJ))?;
    assert_eq!(read.value, obj::read(TRIANGLE_OBJ.as_bytes())?);
    assert_eq!(
        import(&only("tri.stl", TRIANGLE_STL))?.value,
        stl::read(TRIANGLE_STL.as_bytes())?
    );
    assert_eq!(
        import(&only("tri.ply", TRIANGLE_PLY))?.value,
        ply::read(TRIANGLE_PLY.as_bytes())?
    );
    assert_eq!(read.warnings.iter().count(), 0);
    Ok(())
}

#[test]
fn the_extension_is_compared_ignoring_ascii_case() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        import(&only("TRI.OBJ", TRIANGLE_OBJ))?.value,
        import(&only("tri.obj", TRIANGLE_OBJ))?.value
    );
    assert_eq!(
        import(&only("tri.Stl", TRIANGLE_STL))?.value,
        stl::read(TRIANGLE_STL.as_bytes())?
    );
    Ok(())
}

#[test]
fn the_extension_comes_from_the_final_path_segment() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        import(&only("scans/v2.final/tri.obj", TRIANGLE_OBJ))?.value,
        obj::read(TRIANGLE_OBJ.as_bytes())?
    );
    // `my.obj/model` has no extension: the `.obj` belongs to a directory.
    assert_eq!(
        import(&only("my.obj/model", TRIANGLE_OBJ)),
        Err(EngineError::UnknownFormat {
            name: "my.obj/model".into()
        })
    );
    Ok(())
}

#[test]
fn a_name_without_an_extension_is_an_unknown_format() {
    for name in ["model", ".obj", "model.", "dir/.obj", ""] {
        assert_eq!(
            import(&only(name, TRIANGLE_OBJ)),
            Err(EngineError::UnknownFormat { name: name.into() }),
            "{name:?}"
        );
    }
}

#[test]
fn a_type_with_no_reader_yet_is_an_unknown_format() {
    for name in ["tri.gltf", "tri.glb", "tri.fbx", "tri.3ds", "tri.obj.zip"] {
        assert_eq!(
            import(&only(name, TRIANGLE_OBJ)),
            Err(EngineError::UnknownFormat { name: name.into() }),
            "{name:?}"
        );
    }
}

#[test]
fn unknown_format_names_the_file_and_locates_nothing() {
    let e = EngineError::UnknownFormat {
        name: "bunny.3ds".into(),
    };
    assert_eq!(
        e.to_string(),
        "cannot read \"bunny.3ds\": no reader for this file type"
    );
    assert_eq!(e.code(), "UnknownFormat");
    assert_eq!(e.locations(), ErrorLocations::default());
    assert!(e.proposals().is_empty());
}

#[test]
fn a_reader_refusal_is_parse_naming_that_reader() {
    let refused = import(&only("broken.obj", "f 1 2 3\n"));
    assert!(
        matches!(
            &refused,
            Err(EngineError::Parse {
                format: Format::Obj,
                ..
            })
        ),
        "{refused:?}"
    );
    let Err(EngineError::Parse { detail, .. }) = refused else {
        return;
    };
    let Err(openplush_io::IoError::Parse { detail: io, .. }) = obj::read(b"f 1 2 3\n") else {
        panic!("the OBJ reader accepted a face with no vertices");
    };
    assert_eq!(detail, io);
}

#[test]
fn a_stl_refusal_names_the_stl_reader() {
    let refused = import(&only("broken.stl", "solid t\nfacet\n"));
    assert!(
        matches!(
            refused,
            Err(EngineError::Parse {
                format: Format::Stl,
                ..
            })
        ),
        "{refused:?}"
    );
}

#[test]
fn non_finite_input_names_every_non_finite_source_vertex() {
    let text = "v nan 0 0\nv 1 0 0\nv 0 1 0\nv 0 inf 0\nf 1 2 3\n";
    assert_eq!(
        import(&only("nan.obj", text)),
        Err(EngineError::NonFiniteInput {
            vertices: vec![
                SourceCorner {
                    primitive: 0,
                    vertex: 0
                },
                SourceCorner {
                    primitive: 0,
                    vertex: 3
                },
            ]
        })
    );
}

#[test]
fn the_other_files_are_carried_and_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let files = ImportFiles {
        primary: file("tri.obj", TRIANGLE_OBJ),
        others: vec![file("tri.mtl", "newmtl red\n"), file("tex/skin.png", "")],
    };
    assert_eq!(
        import(&files)?.value,
        import(&only("tri.obj", TRIANGLE_OBJ))?.value
    );
    Ok(())
}
