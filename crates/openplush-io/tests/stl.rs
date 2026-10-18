//! `stl::read`: binary and ASCII STL to a `TriangleSoup`, with provenance and located errors.

use openplush_geom::DVec3;
use openplush_io::{Format, IoError, stl};
use openplush_mesh::{FaceId, SourceCorner, TriangleSoup, VertexId};

type Tri = [[f32; 3]; 3];

/// Two triangles: a square split on its diagonal.
const SQUARE: [Tri; 2] = [
    [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
    [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
];

fn binary(tris: &[Tri]) -> Vec<u8> {
    let mut out = vec![0u8; 80];
    out.extend((tris.len() as u32).to_le_bytes());
    for tri in tris {
        out.extend([0u8; 12]);
        tri.iter()
            .flatten()
            .for_each(|c| out.extend(c.to_le_bytes()));
        out.extend([0u8; 2]);
    }
    out
}

fn ascii(solids: &[&[Tri]]) -> String {
    let mut out = String::new();
    for (i, tris) in solids.iter().enumerate() {
        out += &format!("solid part{i}\n");
        for tri in *tris {
            out += "  facet normal 0 0 1\n    outer loop\n";
            for [x, y, z] in tri {
                out += &format!("      vertex {x} {y} {z}\n");
            }
            out += "    endloop\n  endfacet\n";
        }
        out += &format!("endsolid part{i}\n");
    }
    out
}

fn triangles(soup: &TriangleSoup) -> Vec<[u32; 3]> {
    (0..soup.triangle_count())
        .filter_map(|f| soup.triangle(FaceId::new(f as u32)))
        .map(|tri| tri.map(|v| v.index()))
        .collect()
}

fn src(primitive: u32, vertex: u32) -> SourceCorner {
    SourceCorner { primitive, vertex }
}

fn parse_detail(bytes: &[u8]) -> Option<String> {
    match stl::read(bytes) {
        Err(IoError::Parse {
            format: Format::Stl,
            detail,
        }) => Some(detail),
        _ => None,
    }
}

#[test]
fn binary_gives_each_triangle_three_vertices_in_file_order() {
    let soup = stl::read(&binary(&SQUARE)).unwrap();
    assert_eq!(soup.vertex_count(), 6);
    assert_eq!(triangles(&soup), [[0, 1, 2], [3, 4, 5]]);
    assert_eq!(
        soup.position(VertexId::new(5)),
        Some(DVec3::new(0.0, 1.0, 0.0))
    );
}

#[test]
fn a_corner_is_sourced_from_primitive_zero_and_its_vertex_position() {
    let soup = stl::read(&binary(&SQUARE)).unwrap();
    assert_eq!(soup.corner_source(FaceId::new(1), 2), Some(src(0, 5)));
    assert_eq!(soup.corner_uv(FaceId::new(0), 0), None);
    assert_eq!(soup.materials().by_primitive(), [None]);
}

#[test]
fn ascii_reads_the_same_soup_as_binary() {
    let from_ascii = stl::read(ascii(&[&SQUARE]).as_bytes()).unwrap();
    assert_eq!(from_ascii, stl::read(&binary(&SQUARE)).unwrap());
}

#[test]
fn a_binary_file_whose_header_starts_with_solid_is_still_binary() {
    let mut bytes = binary(&SQUARE);
    bytes[..11].copy_from_slice(b"solid sofa ");
    assert_eq!(
        triangles(&stl::read(&bytes).unwrap()),
        [[0, 1, 2], [3, 4, 5]]
    );
}

#[test]
fn ascii_ignores_case_blank_lines_and_crlf() {
    let text = ascii(&[&SQUARE]).to_uppercase().replace('\n', "\r\n\r\n");
    assert_eq!(
        stl::read(text.as_bytes()).unwrap(),
        stl::read(&binary(&SQUARE)).unwrap()
    );
}

#[test]
fn each_ascii_solid_is_its_own_primitive() {
    let text = ascii(&[&SQUARE[..1], &SQUARE[1..]]);
    let soup = stl::read(text.as_bytes()).unwrap();
    assert_eq!(soup.corner_source(FaceId::new(0), 0), Some(src(0, 0)));
    assert_eq!(soup.corner_source(FaceId::new(1), 0), Some(src(1, 3)));
    assert_eq!(soup.materials().by_primitive(), [None, None]);
}

#[test]
fn a_file_with_no_triangles_is_an_empty_soup() {
    assert_eq!(stl::read(&binary(&[])).unwrap().triangle_count(), 0);
    let empty = stl::read(b"solid nothing\nendsolid nothing\n").unwrap();
    assert_eq!(empty.triangle_count(), 0);
}

#[test]
fn a_truncated_binary_file_is_refused_with_both_sizes() {
    let mut bytes = binary(&SQUARE);
    bytes.truncate(bytes.len() - 10);
    let detail = parse_detail(&bytes).unwrap();
    assert!(detail.contains("2 triangles"), "{detail}");
    assert!(detail.contains(&(bytes.len() + 10).to_string()), "{detail}");
}

#[test]
fn trailing_bytes_after_the_declared_triangles_are_refused() {
    let mut bytes = binary(&SQUARE);
    bytes.push(0);
    assert!(parse_detail(&bytes).is_some());
}

#[test]
fn a_file_too_short_for_a_header_is_refused() {
    assert!(parse_detail(&[0u8; 20]).is_some());
}

#[test]
fn an_ascii_error_names_its_line() {
    let text = "solid a\nfacet normal 0 0 1\nouter loop\nvertex 0 0 zero\n";
    assert!(
        parse_detail(text.as_bytes())
            .unwrap()
            .starts_with("line 4:")
    );
}

#[test]
fn ascii_refuses_a_facet_with_other_than_three_vertices() {
    let two = "solid a\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nendloop\nendfacet\nendsolid a\n";
    assert!(parse_detail(two.as_bytes()).unwrap().starts_with("line 6:"));
    let four = "solid a\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nvertex 1 1 0\nendloop\nendfacet\nendsolid a\n";
    assert!(
        parse_detail(four.as_bytes())
            .unwrap()
            .starts_with("line 7:")
    );
}

#[test]
fn ascii_refuses_a_vertex_outside_a_loop_and_a_missing_endsolid() {
    let stray = "solid a\nvertex 0 0 0\nendsolid a\n";
    assert!(
        parse_detail(stray.as_bytes())
            .unwrap()
            .starts_with("line 2:")
    );
    let open = ascii(&[&SQUARE]).replace("endsolid part0\n", "");
    assert!(parse_detail(open.as_bytes()).is_some());
}

#[test]
fn non_finite_vertices_are_all_named_in_file_order() {
    let mut tris = SQUARE;
    tris[0][1][0] = f32::NAN;
    tris[1][2][1] = f32::INFINITY;
    let named = |bytes: &[u8]| match stl::read(bytes) {
        Err(IoError::NonFiniteInput { vertices }) => vertices,
        other => panic!("{other:?}"),
    };
    assert_eq!(named(&binary(&tris)), [src(0, 1), src(0, 5)]);
    let text = ascii(&[&tris]).replace("NaN", "nan");
    assert_eq!(named(text.as_bytes()), [src(0, 1), src(0, 5)]);
}
