//! `obj::read` and `gltf::read` fill `SourceMaterials`: names exactly as the file writes
//! them, one material per primitive. glTF also counts the UV sets beyond the first.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use openplush_io::{Format, IoError, gltf, obj};
use openplush_mesh::{FaceId, MaterialIndex, TriangleSoup};

type Outcome<T> = Result<T, Box<dyn std::error::Error>>;

/// The fixture loader serves only the top of `fixtures/meshes`, so these are embedded.
macro_rules! seam_bytes {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/meshes/source-seams/",
            $name
        ))
    };
}

macro_rules! seam_fixture {
    ($name:literal) => {
        obj::read(seam_bytes!($name))
    };
}

macro_rules! seam_glb {
    ($name:literal) => {
        gltf::read($name, seam_bytes!($name), &|_| None).map(|imported| imported.soup)
    };
}

fn read(text: &str) -> Result<TriangleSoup, IoError> {
    obj::read(text.as_bytes())
}

const TRIANGLE: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\n";

fn by_primitive(soup: &TriangleSoup) -> Vec<Option<u32>> {
    soup.materials()
        .by_primitive()
        .iter()
        .map(|m| m.map(MaterialIndex::index))
        .collect()
}

#[test]
fn three_materials_keep_their_names_untrimmed_in_file_order() -> Outcome<()> {
    let soup = seam_fixture!("three-materials.obj")?;
    assert_eq!(
        soup.materials().names(),
        ["minky / cuddle", "Polar fleece ", "Felt Nose"]
    );
    // One `o` group, two `usemtl` changes: still one primitive per run.
    assert_eq!(by_primitive(&soup), [Some(0), Some(1), Some(2)]);
    assert_eq!(soup.extra_uv_sets(), 0);
    Ok(())
}

#[test]
fn every_face_of_three_materials_belongs_to_a_primitive_with_a_material() -> Outcome<()> {
    let soup = seam_fixture!("three-materials.obj")?;
    for f in 0..u32::try_from(soup.triangle_count())? {
        let primitive = soup.corner_source(FaceId::new(f), 0).map(|c| c.primitive);
        assert!(
            primitive.is_some_and(|p| soup.materials().material(p).is_some()),
            "face {f}"
        );
    }
    Ok(())
}

#[test]
fn no_uv_imports_without_uvs_and_with_one_material() -> Outcome<()> {
    let soup = seam_fixture!("no-uv.obj")?;
    assert_eq!(soup.materials().names(), ["Body"]);
    assert!(by_primitive(&soup).iter().all(|m| *m == Some(0)));
    assert!(!by_primitive(&soup).is_empty());
    for f in 0..u32::try_from(soup.triangle_count())? {
        for c in 0..3 {
            assert_eq!(soup.corner_uv(FaceId::new(f), c), None);
        }
    }
    Ok(())
}

#[test]
fn faces_before_any_usemtl_have_no_material() -> Result<(), IoError> {
    let soup = read(&format!("{TRIANGLE}f 1 2 3\nusemtl A\nf 1 2 3\n"))?;
    assert_eq!(soup.materials().names(), ["A"]);
    assert_eq!(by_primitive(&soup), [None, Some(0)]);
    Ok(())
}

#[test]
fn a_file_without_usemtl_has_no_names_and_no_material() -> Result<(), IoError> {
    let soup = read(&format!("{TRIANGLE}f 1 2 3\n"))?;
    assert!(soup.materials().names().is_empty());
    assert_eq!(by_primitive(&soup), [None]);
    Ok(())
}

#[test]
fn a_repeated_name_is_one_material_and_each_change_is_a_primitive() -> Result<(), IoError> {
    let text = format!("{TRIANGLE}o a\nusemtl A\nf 1 2 3\nusemtl B\nf 1 2 3\nusemtl A\nf 1 2 3\n");
    let soup = read(&text)?;
    assert_eq!(soup.materials().names(), ["A", "B"]);
    assert_eq!(by_primitive(&soup), [Some(0), Some(1), Some(0)]);
    Ok(())
}

#[test]
fn a_material_stays_current_across_o_and_g() -> Result<(), IoError> {
    let soup = read(&format!(
        "{TRIANGLE}usemtl A\nf 1 2 3\no next\nf 1 2 3\ng more\nf 1 2 3\n"
    ))?;
    assert_eq!(by_primitive(&soup), [Some(0), Some(0), Some(0)]);
    Ok(())
}

#[test]
fn names_differing_only_in_whitespace_stay_distinct() -> Result<(), IoError> {
    let text = format!("{TRIANGLE}usemtl A\nf 1 2 3\nusemtl A \nf 1 2 3\nusemtl  A\nf 1 2 3\n");
    let soup = read(&text)?;
    assert_eq!(soup.materials().names(), ["A", "A "]);
    assert_eq!(by_primitive(&soup), [Some(0), Some(1), Some(0)]);
    Ok(())
}

#[test]
fn a_comment_ends_the_name_and_crlf_is_not_part_of_it() -> Result<(), IoError> {
    let soup = read(&format!("{TRIANGLE}usemtl A # note\r\nf 1 2 3\r\n"))?;
    assert_eq!(soup.materials().names(), ["A "]);
    Ok(())
}

#[test]
fn a_usemtl_without_a_name_is_refused_with_its_line() {
    let err = read(&format!("{TRIANGLE}usemtl\nf 1 2 3\n")).unwrap_err();
    let IoError::Parse { detail, .. } = err else {
        panic!("expected a parse error, got {err:?}");
    };
    assert!(detail.starts_with("line 4:"), "{detail}");
}

#[test]
fn a_tab_after_the_keyword_is_a_separator_too() -> Result<(), IoError> {
    let soup = read(&format!("{TRIANGLE}usemtl\tA\nf 1 2 3\n"))?;
    assert_eq!(soup.materials().names(), ["A"]);
    Ok(())
}

#[test]
fn a_continued_usemtl_keeps_blanks_before_the_backslash_and_adds_one_space() -> Result<(), IoError>
{
    let soup = read(&format!("{TRIANGLE}usemtl A \\\nB\nf 1 2 3\n"))?;
    assert_eq!(soup.materials().names(), ["A  B"]);
    Ok(())
}

#[test]
fn a_name_no_face_uses_is_still_listed() -> Result<(), IoError> {
    let soup = read(&format!("{TRIANGLE}usemtl A\nf 1 2 3\nusemtl B\n"))?;
    assert_eq!(soup.materials().names(), ["A", "B"]);
    assert_eq!(by_primitive(&soup), [Some(0)]);
    Ok(())
}

/// Every triangle's three corners name one primitive; the primitives, triangle by triangle.
fn triangle_primitives(soup: &TriangleSoup) -> Outcome<Vec<u32>> {
    let mut out = Vec::new();
    for f in 0..u32::try_from(soup.triangle_count())? {
        let corners: Vec<_> = (0..3)
            .map(|c| soup.corner_source(FaceId::new(f), c).map(|s| s.primitive))
            .collect();
        let first = corners[0].ok_or(format!("face {f} has no source"))?;
        assert!(
            corners.iter().all(|p| *p == Some(first)),
            "face {f}: {corners:?}"
        );
        out.push(first);
    }
    Ok(out)
}

#[test]
fn three_materials_glb_keeps_its_names_in_file_order_one_per_primitive() -> Outcome<()> {
    let soup = seam_glb!("three-materials.glb")?;
    assert_eq!(
        soup.materials().names(),
        ["minky / cuddle", "Polar fleece ", "Felt Nose"]
    );
    assert_eq!(by_primitive(&soup), [Some(0), Some(1), Some(2)]);
    assert_eq!(soup.extra_uv_sets(), 0);
    let mut drawn = triangle_primitives(&soup)?;
    drawn.dedup();
    assert_eq!(drawn, [0, 1, 2], "each primitive's triangles stay together");
    Ok(())
}

#[test]
fn a_glb_without_materials_has_no_names_and_no_material() -> Outcome<()> {
    let soup = seam_glb!("capsule-uv.glb")?;
    assert!(soup.materials().names().is_empty());
    assert_eq!(by_primitive(&soup), [None]);
    assert_eq!(soup.extra_uv_sets(), 0);
    Ok(())
}

/// One triangle's positions (accessor 0) and UVs (accessor 1), and a quad's positions
/// (accessor 2), in one buffer, with `rest` after them.
fn document(rest: &str) -> String {
    let triangle = [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let uvs = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0];
    let quad = [
        0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0,
    ];
    let bytes: Vec<u8> = triangle
        .iter()
        .chain(&uvs)
        .chain(&quad)
        .flat_map(|c| c.to_le_bytes())
        .collect();
    format!(
        r#"{{"asset":{{"version":"2.0"}},"buffers":[{{"uri":"data:application/octet-stream;base64,{data}","byteLength":108}}],"bufferViews":[{{"buffer":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":24}},{{"buffer":0,"byteOffset":60,"byteLength":48}}],"accessors":[{{"bufferView":0,"componentType":5126,"type":"VEC3","count":3,"min":[0,0,0],"max":[1,1,0]}},{{"bufferView":1,"componentType":5126,"type":"VEC2","count":3}},{{"bufferView":2,"componentType":5126,"type":"VEC3","count":4,"min":[0,0,0],"max":[1,1,0]}}],{rest}}}"#,
        data = STANDARD.encode(&bytes)
    )
}

/// The triangle as a primitive with `uv_sets` texture sets from `TEXCOORD_0` up and, if
/// given, a material.
fn primitive(material: Option<u32>, uv_sets: u32) -> String {
    let sets: String = (0..uv_sets)
        .map(|n| format!(r#","TEXCOORD_{n}":1"#))
        .collect();
    let material = material
        .map(|m| format!(r#","material":{m}"#))
        .unwrap_or_default();
    format!(r#"{{"attributes":{{"POSITION":0{sets}}}{material}}}"#)
}

/// `materials` and one mesh of `primitives`, drawn by `nodes` root nodes.
fn drawn(materials: &str, primitives: &[String], nodes: u32) -> Result<TriangleSoup, IoError> {
    let node_list: Vec<&str> = (0..nodes).map(|_| r#"{"mesh":0}"#).collect();
    let roots: Vec<String> = (0..nodes).map(|n| n.to_string()).collect();
    let doc = document(&format!(
        r#""materials":[{materials}],"meshes":[{{"primitives":[{}]}}],"nodes":[{}],"scenes":[{{"nodes":[{}]}}],"scene":0"#,
        primitives.join(","),
        node_list.join(","),
        roots.join(",")
    ));
    Ok(gltf::read("model.gltf", doc.as_bytes(), &|_| None)?.soup)
}

const A_AND_B: &str = r#"{"name":"A"},{"name":"B"}"#;

#[test]
fn a_gltf_primitive_without_a_material_has_none_beside_ones_with() -> Result<(), IoError> {
    let primitives = [
        primitive(Some(1), 0),
        primitive(None, 0),
        primitive(Some(0), 0),
    ];
    let soup = drawn(A_AND_B, &primitives, 1)?;
    // File order, where OBJ lists names in first-use order.
    assert_eq!(soup.materials().names(), ["A", "B"]);
    assert_eq!(by_primitive(&soup), [Some(1), None, Some(0)]);
    Ok(())
}

#[test]
fn a_gltf_material_no_primitive_uses_is_still_listed() -> Result<(), IoError> {
    let soup = drawn(A_AND_B, &[primitive(Some(1), 0)], 1)?;
    assert_eq!(soup.materials().names(), ["A", "B"]);
    assert_eq!(by_primitive(&soup), [Some(1)]);
    Ok(())
}

#[test]
fn a_gltf_mesh_drawn_twice_is_two_primitives_each_with_its_material() -> Outcome<()> {
    let soup = drawn(A_AND_B, &[primitive(Some(1), 0), primitive(None, 0)], 2)?;
    assert_eq!(by_primitive(&soup), [Some(1), None, Some(1), None]);
    assert_eq!(triangle_primitives(&soup)?, [0, 1, 2, 3]);
    Ok(())
}

#[test]
fn strips_and_fans_keep_every_corner_under_their_primitive() -> Outcome<()> {
    let quad = |mode: u32, material: u32| {
        format!(r#"{{"attributes":{{"POSITION":2}},"mode":{mode},"material":{material}}}"#)
    };
    let primitives = [quad(5, 1), primitive(None, 0), quad(6, 0)];
    let soup = drawn(A_AND_B, &primitives, 1)?;
    assert_eq!(triangle_primitives(&soup)?, [0, 0, 1, 2, 2]);
    assert_eq!(by_primitive(&soup), [Some(1), None, Some(0)]);
    Ok(())
}

#[test]
fn extra_uv_sets_counts_texcoord_1_and_up_on_the_primitive_with_most() -> Result<(), IoError> {
    assert_eq!(drawn("", &[primitive(None, 1)], 1)?.extra_uv_sets(), 0);
    for order in [[3, 1], [1, 3]] {
        let soup = drawn("", &order.map(|sets| primitive(None, sets)), 1)?;
        assert_eq!(soup.extra_uv_sets(), 2, "sets per primitive {order:?}");
    }
    Ok(())
}

#[test]
fn a_gltf_material_index_past_the_list_is_refused() {
    let err = drawn(A_AND_B, &[primitive(Some(2), 0)], 1).unwrap_err();
    assert!(
        matches!(
            err,
            IoError::Parse {
                format: Format::Gltf,
                ..
            }
        ),
        "{err:?}"
    );
}
