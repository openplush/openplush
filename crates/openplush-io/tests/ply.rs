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
        _ => Vec::new(),
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
        .replace("1 1 0", "1 zzz 0");
    let detail = parse_detail(text.as_bytes()).unwrap();
    assert!(detail.starts_with("line "), "{detail}");
}

fn ascii_with(header: &str, body: &str) -> Vec<u8> {
    format!("ply\nformat ascii 1.0\n{header}end_header\n{body}").into_bytes()
}

const TRI_HEADER: &str = "element vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\n";
const TRI_VERTICES: &str = "0 0 0\n1 0 0\n0 1 0\n";

#[test]
fn list_types_must_be_integers() {
    for list in ["float int", "uchar float", "double double"] {
        let header = TRI_HEADER.replace("uchar int", list);
        let detail = parse_detail(&ascii_with(&header, &format!("{TRI_VERTICES}3 0 1 2\n")));
        assert!(detail.is_some_and(|d| d.starts_with("line 8:")), "{list}");
    }
}

#[test]
fn an_ascii_integer_must_fit_its_declared_type() {
    let big = TRI_HEADER.replace(
        "property list uchar int",
        "property uchar 8\nproperty list uchar int",
    );
    let cases = [
        (
            big.replace("8", "tag"),
            "0 0 0\n1 0 0\n0 1 0\n300 3 0 1 2\n",
        ),
        (
            TRI_HEADER.replace("uchar int", "uchar uint"),
            "0 0 0\n1 0 0\n0 1 0\n3 0 -1 2\n",
        ),
        (
            TRI_HEADER.replace("uchar int", "char int"),
            "0 0 0\n1 0 0\n0 1 0\n-3 0 1 2\n",
        ),
    ];
    for (header, body) in cases {
        assert!(parse_detail(&ascii_with(&header, body)).is_some());
    }
}

#[test]
fn an_ascii_float_is_rounded_to_f32_as_binary_is() {
    let ascii_text = ascii_with(TRI_HEADER, "0.1 0 0\n1 0 0\n0 1 0\n3 0 1 2\n");
    let from_ascii = ply::read(&ascii_text).unwrap();
    let rows = vec![
        vec![0.1, 0.0, 0.0],
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
    ];
    let from_binary = ply::read(&file(&spec(Enc::Little), "", &rows, &[vec![0, 1, 2]])).unwrap();
    assert_eq!(from_ascii, from_binary);
}

#[test]
fn a_second_vertex_or_face_element_is_refused() {
    for dup in [
        "element vertex 0\nproperty float x\n",
        "element face 0\nproperty list uchar int vertex_indices\n",
    ] {
        let header = format!("{TRI_HEADER}{dup}");
        let detail = parse_detail(&ascii_with(&header, &format!("{TRI_VERTICES}3 0 1 2\n")));
        assert!(detail.is_some_and(|d| d.contains("more than one")));
    }
}

#[test]
fn a_polygon_with_too_many_corners_is_refused_rather_than_clipped_for_minutes() {
    let n = 1001;
    let mut body = String::new();
    for i in 0..n {
        let a = std::f64::consts::TAU * f64::from(i) / f64::from(n);
        body += &format!("{} {} 0\n", a.cos(), a.sin());
    }
    body += &format!("{n}");
    (0..n).for_each(|i| body += &format!(" {i}"));
    let header = TRI_HEADER
        .replace("vertex 3", &format!("vertex {n}"))
        .replace("uchar int", "ushort int");
    let detail = parse_detail(&ascii_with(&header, &body)).unwrap();
    assert!(detail.contains("1000"), "{detail}");
    // At the cap it still reads.
    let ok_body: String = body
        .lines()
        .take(1000)
        .map(|l| format!("{l}\n"))
        .collect::<String>()
        + "1000"
        + &(0..1000).map(|i| format!(" {i}")).collect::<String>();
    let header = TRI_HEADER
        .replace("vertex 3", "vertex 1000")
        .replace("uchar int", "ushort int");
    assert_eq!(
        ply::read(&ascii_with(&header, &ok_body))
            .unwrap()
            .triangle_count(),
        998
    );
}

const TYPE_NAMES: [&str; 16] = [
    "char", "int8", "uchar", "uint8", "short", "int16", "ushort", "uint16", "int", "int32", "uint",
    "uint32", "float", "float32", "double", "float64",
];

#[test]
fn every_type_name_reads_its_value_in_every_encoding() {
    for ty in TYPE_NAMES {
        for enc in [Enc::Ascii, Enc::Little, Enc::Big] {
            let s = Spec {
                enc,
                vertex: &[(ty, "x"), (ty, "y"), (ty, "z")],
                face: ("uchar", "int", "vertex_indices"),
            };
            let soup = ply::read(&file(&s, "", &[vec![3.0, 4.0, 5.0]], &[])).unwrap();
            assert_eq!(
                soup.position(VertexId::new(0)),
                Some(DVec3::new(3.0, 4.0, 5.0)),
                "{ty}"
            );
        }
    }
}

#[test]
fn signed_and_unsigned_types_decode_their_full_range() {
    let cases = [
        (["char", "short", "int"], [-2.0, -300.0, -70_000.0]),
        (
            ["uchar", "ushort", "uint"],
            [200.0, 40_000.0, 3_000_000_000.0],
        ),
    ];
    for enc in [Enc::Little, Enc::Big] {
        for (types, row) in &cases {
            let s = Spec {
                enc,
                vertex: &[(types[0], "x"), (types[1], "y"), (types[2], "z")],
                face: ("uchar", "int", "vertex_indices"),
            };
            let soup = ply::read(&file(&s, "", &[row.to_vec()], &[])).unwrap();
            assert_eq!(
                soup.position(VertexId::new(0)),
                Some(DVec3::new(row[0], row[1], row[2]))
            );
        }
    }
}

#[test]
fn a_quad_gives_exactly_these_triangles_and_every_position_is_where_it_was_written() {
    let soup = ply::read(&file(&spec(Enc::Ascii), "", &square(), &[vec![0, 1, 2, 3]])).unwrap();
    assert_eq!(triangles(&soup), [[0, 1, 2], [0, 2, 3]]);
    let at = |i| soup.position(VertexId::new(i));
    assert_eq!(at(0), Some(DVec3::new(0.0, 0.0, 0.0)));
    assert_eq!(at(1), Some(DVec3::new(1.0, 0.0, 0.0)));
    assert_eq!(at(3), Some(DVec3::new(0.0, 1.0, 0.0)));
    let tall = vec![
        vec![0.0, 0.0, 7.0],
        vec![1.0, 0.0, 7.0],
        vec![1.0, 2.0, 7.0],
        vec![0.0, 2.0, 7.0],
    ];
    let soup = ply::read(&file(&spec(Enc::Ascii), "", &tall, &[vec![0, 1, 2, 3]])).unwrap();
    assert_eq!(
        soup.position(VertexId::new(2)),
        Some(DVec3::new(1.0, 2.0, 7.0))
    );
}

#[test]
fn non_integers_in_integer_slots_are_refused() {
    for body in ["3 0.5 1 2\n", "2.5 0 1 2\n", "3 -1.5 1 2\n"] {
        let detail =
            parse_detail(&ascii_with(TRI_HEADER, &format!("{TRI_VERTICES}{body}"))).unwrap();
        assert!(detail.contains("not an integer"), "{detail}");
    }
}

#[test]
fn more_header_faults_are_refused_and_blank_header_lines_are_not() {
    let good =
        String::from_utf8(ascii_with(TRI_HEADER, &format!("{TRI_VERTICES}3 0 1 2\n"))).unwrap();
    for (what, case) in [
        ("no format", good.replace("format ascii 1.0\n", "")),
        (
            "unknown keyword",
            good.replace("element face 1", "bogus 1\nelement face 1"),
        ),
        (
            "bad count",
            good.replace("element vertex 3", "element vertex abc"),
        ),
        ("ply with extras", good.replacen("ply\n", "ply junk\n", 1)),
        (
            "scalar index list",
            good.replace(
                "property list uchar int vertex_indices",
                "property int vertex_indices",
            ),
        ),
    ] {
        assert!(parse_detail(case.as_bytes()).is_some(), "{what}");
    }
    let blank = good.replace("element face 1", "\n\nelement face 1");
    assert_eq!(
        triangles(&ply::read(blank.as_bytes()).unwrap()),
        [[0, 1, 2]]
    );
}

#[test]
fn a_file_ending_at_end_header_without_a_newline_is_an_empty_soup() {
    let text = "ply\nformat ascii 1.0\nelement vertex 0\nproperty float x\nproperty float y\nproperty float z\nend_header";
    assert_eq!(ply::read(text.as_bytes()).unwrap().vertex_count(), 0);
}

#[test]
fn faces_may_lead_with_scalars_and_a_vertex_may_carry_lists() {
    let header = "element vertex 3\nproperty float x\nproperty list uchar int tags\nproperty float y\nproperty float z\nelement edge 2\nproperty int a\nproperty int b\nelement face 2\nproperty uchar flags\nproperty list uchar int vertex_indices\nproperty list uchar int more\nelement junk 18446744073709551615\n";
    let body = "0 2 5 6 0 0\n1 0 0 0\n0 1 9 1 0\n0 1\n1 2\n7 3 0 1 2 1 9\n9 3 0 1 2 0\n";
    let soup = ply::read(&ascii_with(header, body)).unwrap();
    assert_eq!(triangles(&soup), [[0, 1, 2], [0, 1, 2]]);
    assert_eq!(
        soup.position(VertexId::new(1)),
        Some(DVec3::new(1.0, 0.0, 0.0))
    );
    assert_eq!(
        soup.position(VertexId::new(2)),
        Some(DVec3::new(0.0, 1.0, 0.0))
    );
}

#[test]
fn a_face_index_equal_to_the_vertex_count_or_negative_is_refused() {
    assert!(parse_detail(&file(&spec(Enc::Ascii), "", &square(), &[vec![0, 1, 4]])).is_some());
    assert!(ply::read(&file(&spec(Enc::Ascii), "", &square(), &[vec![0, 1, 3]])).is_ok());
    let mut bytes = file(&spec(Enc::Little), "", &square(), &[vec![0, 1, 2]]);
    let end = bytes.len();
    bytes[end - 8..end - 4].copy_from_slice(&(-1i32).to_le_bytes());
    assert!(parse_detail(&bytes).is_some());
}

#[test]
fn a_face_before_the_vertex_element_says_so() {
    let text = "ply\nformat ascii 1.0\nelement face 1\nproperty list uchar int vertex_indices\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nend_header\n3 0 1 2\n0 0 0\n1 0 0\n0 1 0\n";
    assert!(
        parse_detail(text.as_bytes())
            .unwrap()
            .contains("comes before")
    );
}

#[test]
fn a_body_fault_names_its_exact_line_with_lf_or_crlf_and_comments() {
    for (header, nl) in [("", "\n"), ("comment one\ncomment two\n", "\r\n")] {
        let text = String::from_utf8(file(&spec(Enc::Ascii), header, &square(), &[vec![0, 1, 2]]))
            .unwrap()
            .replace("1 1 0", "1 zzz 0")
            .replace('\n', nl);
        let line = text.lines().position(|l| l.contains("zzz")).unwrap() + 1;
        assert_eq!(
            parse_detail(text.as_bytes()).unwrap(),
            format!("line {line}: `zzz` is not a number")
        );
    }
}

#[test]
fn a_non_text_ascii_body_names_the_byte() {
    let mut bytes = file(&spec(Enc::Ascii), "", &square(), &[vec![0, 1, 2]]);
    let start = bytes
        .windows(11)
        .position(|w| w == b"end_header\n")
        .unwrap()
        + 11;
    bytes[start + 4] = 0xff;
    assert_eq!(
        parse_detail(&bytes).unwrap(),
        format!("byte {}: the body is not text", start + 4)
    );
}

#[test]
fn a_non_finite_texture_coordinate_is_refused_naming_its_vertex() {
    let props = [
        ("float", "x"),
        ("float", "y"),
        ("float", "z"),
        ("float", "s"),
        ("float", "t"),
    ];
    for bad in [f64::NAN, f64::INFINITY, 1e50] {
        for enc in [Enc::Ascii, Enc::Little, Enc::Big] {
            let s = Spec {
                enc,
                vertex: &props,
                face: ("uchar", "int", "vertex_indices"),
            };
            let mut rows: Vec<Vec<f64>> = square()
                .into_iter()
                .map(|mut r| {
                    r.extend([0.5, 0.5]);
                    r
                })
                .collect();
            rows[2][4] = bad;
            let detail = parse_detail(&file(&s, "", &rows, &[vec![0, 1, 2]])).unwrap();
            assert!(
                detail.contains("vertex 2") && detail.contains("texture"),
                "{bad} {detail}"
            );
        }
    }
}
