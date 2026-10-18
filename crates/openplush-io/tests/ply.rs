//! `ply::read`: ASCII and binary PLY to a `TriangleSoup`, with provenance, UVs and
//! located errors.

use openplush_geom::{DVec2, DVec3};
use openplush_io::{Format, IoError, ply};
use openplush_mesh::{FaceId, SourceCorner, TriangleSoup, VertexId};

#[derive(Clone, Copy, PartialEq)]
enum Enc {
    Ascii,
    Little,
    Big,
}

impl Enc {
    fn name(self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::Little => "binary_little_endian",
            Self::Big => "binary_big_endian",
        }
    }
}

/// One value in the named PLY type, as text or bytes.
fn value(enc: Enc, ty: &str, v: f64, out: &mut Vec<u8>) {
    if enc == Enc::Ascii {
        out.extend(format!("{v} ").bytes());
        return;
    }
    let bytes: Vec<u8> = match ty {
        "char" | "int8" => (v as i8).to_le_bytes().to_vec(),
        "uchar" | "uint8" => (v as u8).to_le_bytes().to_vec(),
        "short" | "int16" => (v as i16).to_le_bytes().to_vec(),
        "ushort" | "uint16" => (v as u16).to_le_bytes().to_vec(),
        "int" | "int32" => (v as i32).to_le_bytes().to_vec(),
        "uint" | "uint32" => (v as u32).to_le_bytes().to_vec(),
        "float" | "float32" => (v as f32).to_le_bytes().to_vec(),
        "double" | "float64" => v.to_le_bytes().to_vec(),
        other => panic!("{other}"),
    };
    if enc == Enc::Big {
        out.extend(bytes.iter().rev());
    } else {
        out.extend(bytes);
    }
}

struct Spec<'a> {
    enc: Enc,
    /// Vertex properties as (type, name).
    vertex: &'a [(&'a str, &'a str)],
    /// (count type, index type, list name).
    face: (&'a str, &'a str, &'a str),
}

const XYZ: [(&str, &str); 3] = [("float", "x"), ("float", "y"), ("float", "z")];

fn file(spec: &Spec, extra_header: &str, vertices: &[Vec<f64>], faces: &[Vec<u32>]) -> Vec<u8> {
    let mut out = format!("ply\nformat {} 1.0\n{extra_header}", spec.enc.name()).into_bytes();
    out.extend(format!("element vertex {}\n", vertices.len()).bytes());
    for (ty, name) in spec.vertex {
        out.extend(format!("property {ty} {name}\n").bytes());
    }
    out.extend(format!("element face {}\n", faces.len()).bytes());
    let (count, index, name) = spec.face;
    out.extend(format!("property list {count} {index} {name}\nend_header\n").bytes());
    for row in vertices {
        for (cell, (ty, _)) in row.iter().zip(spec.vertex) {
            value(spec.enc, ty, *cell, &mut out);
        }
        if spec.enc == Enc::Ascii {
            out.push(b'\n');
        }
    }
    for face in faces {
        value(spec.enc, count, face.len() as f64, &mut out);
        for &i in face {
            value(spec.enc, index, f64::from(i), &mut out);
        }
        if spec.enc == Enc::Ascii {
            out.push(b'\n');
        }
    }
    out
}

fn spec(enc: Enc) -> Spec<'static> {
    Spec {
        enc,
        vertex: &XYZ,
        face: ("uchar", "int", "vertex_indices"),
    }
}

fn square() -> Vec<Vec<f64>> {
    vec![
        vec![0.0, 0.0, 0.0],
        vec![1.0, 0.0, 0.0],
        vec![1.0, 1.0, 0.0],
        vec![0.0, 1.0, 0.0],
    ]
}

fn triangles(soup: &TriangleSoup) -> Vec<[u32; 3]> {
    (0..soup.triangle_count())
        .filter_map(|f| soup.triangle(FaceId::new(f as u32)))
        .map(|tri| tri.map(|v| v.index()))
        .collect()
}

fn src(vertex: u32) -> SourceCorner {
    SourceCorner {
        primitive: 0,
        vertex,
    }
}

fn parse_detail(bytes: &[u8]) -> Option<String> {
    match ply::read(bytes) {
        Err(IoError::Parse {
            format: Format::Ply,
            detail,
        }) => Some(detail),
        _ => None,
    }
}

#[test]
fn a_quad_is_triangulated_and_each_corner_keeps_its_vertex() {
    let bytes = file(&spec(Enc::Ascii), "", &square(), &[vec![0, 1, 2, 3]]);
    let soup = ply::read(&bytes).unwrap();
    assert_eq!(soup.vertex_count(), 4);
    assert_eq!(soup.triangle_count(), 2);
    let sources: Vec<SourceCorner> = (0..2)
        .flat_map(|f| (0..3).map(move |c| (f, c)))
        .map(|(f, c)| soup.corner_source(FaceId::new(f), c).unwrap())
        .collect();
    let by_triangle: Vec<[u32; 3]> = triangles(&soup);
    for (t, tri) in by_triangle.iter().enumerate() {
        for (c, v) in tri.iter().enumerate() {
            assert_eq!(sources[t * 3 + c], src(*v));
        }
    }
    assert_eq!(soup.materials().by_primitive(), [None]);
}

#[test]
fn every_encoding_reads_the_same_soup() {
    let faces = [vec![0, 1, 2], vec![0, 2, 3]];
    let ascii = ply::read(&file(&spec(Enc::Ascii), "", &square(), &faces)).unwrap();
    for enc in [Enc::Little, Enc::Big] {
        let soup = ply::read(&file(&spec(enc), "", &square(), &faces)).unwrap();
        assert_eq!(soup, ascii);
    }
    assert_eq!(triangles(&ascii), [[0, 1, 2], [0, 2, 3]]);
}

#[test]
fn coordinate_and_list_types_may_be_any_numeric_type() {
    let faces = [vec![0, 1, 2]];
    let want = ply::read(&file(&spec(Enc::Ascii), "", &square(), &faces)).unwrap();
    for enc in [Enc::Little, Enc::Big] {
        for (vt, count, index) in [
            ("double", "uint8", "uint32"),
            ("float32", "char", "short"),
            ("float64", "ushort", "ushort"),
        ] {
            let s = Spec {
                enc,
                vertex: &[(vt, "x"), (vt, "y"), (vt, "z")],
                face: (count, index, "vertex_index"),
            };
            assert_eq!(ply::read(&file(&s, "", &square(), &faces)).unwrap(), want);
        }
    }
}

#[test]
fn texture_coordinates_become_per_corner_uvs_under_any_of_their_names() {
    for (u, v) in [
        ("s", "t"),
        ("u", "v"),
        ("texture_u", "texture_v"),
        ("texture_s", "texture_t"),
    ] {
        let props = [
            ("float", "x"),
            ("float", "y"),
            ("float", "z"),
            ("float", u),
            ("float", v),
        ];
        let s = Spec {
            enc: Enc::Little,
            vertex: &props,
            face: ("uchar", "int", "vertex_indices"),
        };
        let rows: Vec<Vec<f64>> = square()
            .into_iter()
            .enumerate()
            .map(|(i, mut r)| {
                r.extend([i as f64 * 0.25, 0.5]);
                r
            })
            .collect();
        let soup = ply::read(&file(&s, "", &rows, &[vec![3, 1, 0]])).unwrap();
        let uvs: Vec<Option<DVec2>> = (0..3).map(|c| soup.corner_uv(FaceId::new(0), c)).collect();
        assert_eq!(
            uvs,
            [
                Some(DVec2::new(0.75, 0.5)),
                Some(DVec2::new(0.25, 0.5)),
                Some(DVec2::new(0.0, 0.5)),
            ],
            "{u}/{v}"
        );
    }
}

#[test]
fn a_file_without_texture_coordinates_has_none() {
    let soup = ply::read(&file(&spec(Enc::Ascii), "", &square(), &[vec![0, 1, 2]])).unwrap();
    assert_eq!(soup.corner_uv(FaceId::new(0), 0), None);
}

#[test]
fn comments_other_properties_and_other_elements_are_skipped() {
    let props = [
        ("float", "x"),
        ("uchar", "red"),
        ("float", "y"),
        ("float", "nx"),
        ("float", "z"),
    ];
    let s = Spec {
        enc: Enc::Little,
        vertex: &props,
        face: ("uchar", "int", "vertex_indices"),
    };
    let rows: Vec<Vec<f64>> = square()
        .into_iter()
        .map(|r| vec![r[0], 200.0, r[1], 0.5, r[2]])
        .collect();
    let mut bytes = file(
        &s,
        "comment made by hand\nobj_info scale 2\n",
        &rows,
        &[vec![0, 1, 2]],
    );
    // An element after the faces, with a list property, is read past.
    let header_end = bytes
        .windows(11)
        .position(|w| w == b"end_header\n")
        .unwrap();
    bytes.splice(
        header_end..header_end,
        b"element edge 1\nproperty list uchar int junk\n"
            .iter()
            .copied(),
    );
    bytes.extend([2, 7, 0, 0, 0, 9, 0, 0, 0]);
    let soup = ply::read(&bytes).unwrap();
    assert_eq!(
        soup.position(VertexId::new(2)),
        Some(DVec3::new(1.0, 1.0, 0.0))
    );
    assert_eq!(triangles(&soup), [[0, 1, 2]]);
}

#[test]
fn a_header_with_crlf_line_ends_is_read() {
    let bytes = file(&spec(Enc::Ascii), "", &square(), &[vec![0, 1, 2]]);
    let text = String::from_utf8(bytes).unwrap().replace('\n', "\r\n");
    assert_eq!(triangles(&ply::read(text.as_bytes()).unwrap()), [[0, 1, 2]]);
}

#[test]
fn a_file_with_no_vertices_or_faces_is_an_empty_soup() {
    let soup = ply::read(&file(&spec(Enc::Ascii), "", &[], &[])).unwrap();
    assert_eq!(soup.triangle_count(), 0);
}

#[test]
fn non_finite_vertices_are_all_named() {
    let mut rows = square();
    rows[1][0] = f64::NAN;
    rows[3][2] = f64::INFINITY;
    let bytes = file(&spec(Enc::Little), "", &rows, &[vec![0, 1, 2]]);
    match ply::read(&bytes) {
        Err(IoError::NonFiniteInput { vertices }) => assert_eq!(vertices, [src(1), src(3)]),
        other => panic!("{other:?}"),
    }
}

#[test]
fn header_faults_are_refused() {
    let good = file(&spec(Enc::Ascii), "", &square(), &[vec![0, 1, 2]]);
    let text = String::from_utf8(good).unwrap();
    let cases = [
        ("no magic", text.replacen("ply", "plx", 1)),
        ("unknown format", text.replace("ascii", "ebcdic")),
        ("no end_header", text.replace("end_header\n", "")),
        ("no x", text.replace("property float x\n", "")),
        (
            "unknown type",
            text.replace("property float y", "property quad y"),
        ),
        (
            "no vertex element",
            text.replace("element vertex", "element thing"),
        ),
    ];
    for (what, case) in cases {
        assert!(parse_detail(case.as_bytes()).is_some(), "{what}");
    }
}

#[test]
fn a_face_before_the_vertex_element_is_refused() {
    let text = "ply\nformat ascii 1.0\nelement face 1\nproperty list uchar int vertex_indices\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nend_header\n3 0 1 2\n0 0 0\n1 0 0\n0 1 0\n";
    assert!(parse_detail(text.as_bytes()).is_some());
}

#[test]
fn body_faults_are_refused() {
    let tri = [vec![0, 1, 2]];
    let with = |faces: &[Vec<u32>]| file(&spec(Enc::Ascii), "", &square(), faces);
    assert!(
        parse_detail(&with(&[vec![0, 1, 9]])).is_some(),
        "index past the end"
    );
    assert!(parse_detail(&with(&[vec![0, 1]])).is_some(), "two corners");
    assert!(
        parse_detail(&with(&[vec![0, 2, 1, 3]])).is_some(),
        "bow tie"
    );
    let mut cut = file(&spec(Enc::Little), "", &square(), &tri);
    cut.truncate(cut.len() - 3);
    assert!(parse_detail(&cut).is_some(), "truncated binary");
    let text = String::from_utf8(with(&tri))
        .unwrap()
        .replace("1 1 0", "1 one 0");
    let detail = parse_detail(text.as_bytes()).unwrap();
    assert!(detail.starts_with("line "), "{detail}");
}
