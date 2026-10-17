//! `obj::read` fills `SourceMaterials`: names exactly as written after `usemtl`, one
//! material per primitive. The glTF half lands with the glTF reader.

use openplush_io::{IoError, obj};
use openplush_mesh::{FaceId, MaterialIndex, TriangleSoup};

type Outcome<T> = Result<T, Box<dyn std::error::Error>>;

/// The fixture loader serves only the top of `fixtures/meshes`, so these two are embedded.
macro_rules! seam_fixture {
    ($name:literal) => {
        obj::read(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/meshes/source-seams/",
            $name
        )))
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
