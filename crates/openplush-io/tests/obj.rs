//! `obj::read`: OBJ bytes to a `TriangleSoup`, with provenance, UVs and located errors.

use openplush_geom::{DVec2, DVec3};
use openplush_io::{Format, IoError, obj};
use openplush_mesh::{FaceId, SourceCorner, TriangleSoup, VertexId};
use openplush_testkit::fixtures::{self, Kind};

fn read(text: &str) -> Result<TriangleSoup, IoError> {
    obj::read(text.as_bytes())
}

fn parse_detail(text: &[u8]) -> Option<String> {
    match obj::read(text) {
        Err(IoError::Parse {
            format: Format::Obj,
            detail,
        }) => Some(detail),
        _ => None,
    }
}

fn triangles(soup: &TriangleSoup) -> Vec<[u32; 3]> {
    (0..soup.triangle_count())
        .filter_map(|f| soup.triangle(FaceId::new(f as u32)))
        .map(|tri| tri.map(|v| v.index()))
        .collect()
}

fn sources(soup: &TriangleSoup, f: u32) -> Option<[SourceCorner; 3]> {
    let corner = |c| soup.corner_source(FaceId::new(f), c);
    Some([corner(0)?, corner(1)?, corner(2)?])
}

fn src(primitive: u32, vertex: u32) -> SourceCorner {
    SourceCorner { primitive, vertex }
}

const SQUARE: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\n";

#[test]
fn reads_a_triangle_with_its_provenance() {
    let soup = read("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").expect("reads");
    assert_eq!(soup.vertex_count(), 3);
    assert_eq!(triangles(&soup), [[0, 1, 2]]);
    assert_eq!(
        soup.position(VertexId::new(1)),
        Some(DVec3::new(1.0, 0.0, 0.0))
    );
    assert_eq!(sources(&soup, 0), Some([src(0, 0), src(0, 1), src(0, 2)]));
    assert_eq!(soup.corner_uv(FaceId::new(0), 0), None);
}

#[test]
fn keeps_unused_vertices_in_file_order() {
    let soup = read("v 9 9 9\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 2 3 4\n").expect("reads");
    assert_eq!(soup.vertex_count(), 4);
    assert_eq!(triangles(&soup), [[1, 2, 3]]);
    assert_eq!(
        soup.position(VertexId::new(0)),
        Some(DVec3::new(9.0, 9.0, 9.0))
    );
}

#[test]
fn resolves_negative_indices_against_the_vertices_read_so_far() {
    let soup = read("v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\nv 5 5 5\nf -4 -3 -1\n").expect("reads");
    assert_eq!(triangles(&soup), [[0, 1, 2], [0, 1, 3]]);
}

#[test]
fn reads_every_corner_form_and_keeps_uvs_per_corner() {
    let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0.25 0.5\nvt 0.75\nvn 0 0 1\n\
                f 1/1 2/2/1 3//1\n";
    let soup = read(text).expect("reads");
    let f = FaceId::new(0);
    assert_eq!(soup.corner_uv(f, 0), Some(DVec2::new(0.25, 0.5)));
    assert_eq!(soup.corner_uv(f, 1), Some(DVec2::new(0.75, 0.0)));
    assert_eq!(soup.corner_uv(f, 2), None);
}

#[test]
fn splits_primitives_at_object_group_and_material_lines() {
    let text = format!(
        "{SQUARE}o a\nf 1 2 3\nf 1 3 4\ng b\ng c\nusemtl red\nf 1 2 4\nusemtl blue\nf 2 3 4\n"
    );
    let soup = read(&text).expect("reads");
    let primitives: Vec<u32> = (0..4)
        .map(|f| sources(&soup, f).expect("in range")[0].primitive)
        .collect();
    assert_eq!(primitives, [0, 0, 1, 2]);
}

#[test]
fn ignores_comments_unknown_statements_and_joins_continued_lines() {
    let text = "# header\nmtllib a.mtl\ns 1\nv 0 0 0 # origin\nv 1 0 0\nv 0 1 0 \\\n  \nl 1 2\n\
                f 1 \\\n 2 3\n";
    let soup = read(text).expect("reads");
    assert_eq!(triangles(&soup), [[0, 1, 2]]);
}

#[test]
fn takes_the_first_three_vertex_numbers_when_more_are_given() {
    let soup = read("v 0 0 0 1\nv 1 0 0 0.5 0.5 0.5\nv 0 1 0\nf 1 2 3\n").expect("reads");
    assert_eq!(
        soup.position(VertexId::new(1)),
        Some(DVec3::new(1.0, 0.0, 0.0))
    );
}

#[test]
fn refuses_non_finite_vertices_naming_every_one_used_or_not() {
    let text = "o a\nv nan 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\no b\nv 0 inf 0\nv 0 0 1e400\nf 2 3 4\n";
    match obj::read(text.as_bytes()) {
        Err(IoError::NonFiniteInput { vertices }) => {
            assert_eq!(vertices, [src(0, 0), src(1, 3), src(1, 4)]);
        }
        other => panic!("expected NonFiniteInput, got {other:?}"),
    }
}

#[test]
fn refuses_malformed_input_naming_the_line() {
    let cases: &[(&[u8], &str)] = &[
        (b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 0\n", "line 4"),
        (b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 4\n", "line 4"),
        (b"v 0 0 0\nv 1 0 0\nf 1 2 -3\nv 0 1 0\n", "line 3"),
        (b"v 0 0 0\nv 1 0 0\nf 1 2\n", "line 3"),
        (b"v 0 0 x\n", "line 1"),
        (b"\nv 0 0\n", "line 2"),
        (b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/1 2 3\n", "line 4"),
        (b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/2/3/4 2 3\n", "line 4"),
        (b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/a 2 3\n", "line 4"),
        (b"vt nan 0\n", "line 1"),
        (b"vt\n", "line 1"),
        (b"cstype bspline\ncurv 0 1 1 2\n", "line 1"),
        (b"surf 0 1 0 1 1 2 3 4\n", "line 1"),
    ];
    for (text, line) in cases {
        let detail = parse_detail(text).unwrap_or_default();
        assert!(
            detail.starts_with(line),
            "{:?}: {detail:?} does not start with {line:?}",
            String::from_utf8_lossy(text)
        );
    }
}

#[test]
fn refuses_text_that_is_not_utf8() {
    let detail = parse_detail(b"v 0 0 0\nv \xff 0 0\n").unwrap_or_default();
    assert!(detail.contains("UTF-8"), "{detail:?}");
}

// Oracles independent of the triangulator: every output triangle turns the same way
// as the polygon (its normal agrees with the polygon's Newell normal), and the
// triangle areas add up to the polygon's area.
fn newell(points: &[DVec3]) -> DVec3 {
    let mut n = DVec3::new(0.0, 0.0, 0.0);
    for (i, p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        n += DVec3::new(
            (p.y - q.y) * (p.z + q.z),
            (p.z - q.z) * (p.x + q.x),
            (p.x - q.x) * (p.y + q.y),
        );
    }
    n
}

fn assert_tiles(soup: &TriangleSoup, polygon: &[DVec3]) {
    let normal = newell(polygon);
    assert_eq!(soup.triangle_count(), polygon.len() - 2);
    let mut area = 0.0;
    for tri in triangles(soup) {
        let corners = tri.map(|v| soup.position(VertexId::new(v)));
        let [Some(a), Some(b), Some(c)] = corners else {
            unreachable!("soup triangles index their vertices")
        };
        let n = (b - a).cross(c - a);
        assert!(
            n.dot(normal) > 0.0,
            "triangle {tri:?} turns against the polygon"
        );
        area += n.length() / 2.0;
    }
    let expected = normal.length() / 2.0;
    assert!(
        (area - expected).abs() < 1e-12 * expected.max(1.0),
        "{area} != {expected}"
    );
}

fn polygon_text(points: &[DVec3]) -> String {
    let mut text: String = points
        .iter()
        .map(|p| format!("v {} {} {}\n", p.x, p.y, p.z))
        .collect();
    text.push('f');
    for i in 1..=points.len() {
        text.push_str(&format!(" {i}"));
    }
    text.push('\n');
    text
}

#[test]
fn splits_a_convex_quad_on_its_shorter_diagonal() {
    let soup = read("v 0 0 0\nv 3 0 0\nv 4 1 0\nv 1 1 0\nf 1 2 3 4\n").expect("reads");
    assert_eq!(triangles(&soup), [[0, 1, 3], [1, 2, 3]]);
    let soup = read("v 0 0 0\nv 3 0 0\nv 2 1 0\nv -1 1 0\nf 1 2 3 4\n").expect("reads");
    assert_eq!(triangles(&soup), [[0, 1, 2], [0, 2, 3]]);
}

#[test]
fn splits_a_concave_quad_on_its_inside_diagonal_even_when_longer() {
    let points = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.5, 0.0),
        DVec3::new(2.0, 0.0, 0.0),
        DVec3::new(1.0, 5.0, 0.0),
    ];
    let soup = read(&polygon_text(&points)).expect("reads");
    assert_eq!(triangles(&soup), [[0, 1, 3], [1, 2, 3]]);
    assert_tiles(&soup, &points);
}

#[test]
fn ear_clips_a_concave_polygon_in_any_plane() {
    let l_shape = [
        (0.0, 0.0),
        (2.0, 0.0),
        (2.0, 1.0),
        (1.0, 1.0),
        (1.0, 2.0),
        (0.0, 2.0),
    ];
    let planes: [fn(f64, f64) -> DVec3; 3] = [
        |a, b| DVec3::new(a, b, 0.0),
        |a, b| DVec3::new(0.0, a, b),
        |a, b| DVec3::new(b, 1.0, a),
    ];
    for plane in planes {
        let points: Vec<DVec3> = l_shape.iter().map(|&(a, b)| plane(a, b)).collect();
        assert_tiles(&read(&polygon_text(&points)).expect("reads"), &points);
        let reversed: Vec<DVec3> = points.iter().rev().copied().collect();
        assert_tiles(&read(&polygon_text(&reversed)).expect("reads"), &reversed);
    }
}

#[test]
fn keeps_a_degenerate_polygon_for_validation_to_report() {
    let soup = read("v 0 0 0\nv 1 0 0\nv 2 0 0\nv 3 0 0\nv 4 0 0\nf 1 2 3 4 5\n").expect("reads");
    assert_eq!(soup.triangle_count(), 3);
}

#[test]
fn carries_each_corner_through_triangulation() {
    let text = format!("{SQUARE}vt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\ng sq\nf 1/1 2/2 3/3 4/4\n");
    let soup = read(&text).expect("reads");
    for f in 0..2 {
        let face = FaceId::new(f);
        for (c, s) in sources(&soup, f).expect("in range").iter().enumerate() {
            assert_eq!(s.primitive, 0);
            let p = soup.position(VertexId::new(s.vertex)).expect("in range");
            assert_eq!(soup.corner_uv(face, c), Some(DVec2::new(p.x, p.y)));
        }
    }
}

#[test]
fn reads_every_fixture_vertex_for_vertex() {
    for kind in [Kind::Mesh, Kind::Defect] {
        for name in fixtures::names(kind).expect("fixture directory") {
            let bytes = fixtures::load(kind, &name).expect("fixture");
            let soup = obj::read(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
            let text = String::from_utf8(bytes).expect("fixtures are UTF-8");
            let lines = |key: &'static str| {
                text.lines()
                    .filter_map(move |l| l.strip_prefix(key))
                    .map(|rest| rest.split_whitespace().collect::<Vec<_>>())
            };
            let positions: Vec<DVec3> = lines("v ")
                .map(|n| {
                    let n: Vec<f64> = n.iter().map(|x| x.parse().expect("number")).collect();
                    DVec3::new(n[0], n[1], n[2])
                })
                .collect();
            let faces: Vec<[u32; 3]> = lines("f ")
                .map(|n| [0, 1, 2].map(|i| n[i].parse::<u32>().expect("index") - 1))
                .collect();
            assert_eq!(soup.positions(), positions.as_slice(), "{name}");
            assert_eq!(triangles(&soup), faces, "{name}");
        }
    }
}

fn points_xy(xy: &[(f64, f64)]) -> Vec<DVec3> {
    xy.iter().map(|&(x, y)| DVec3::new(x, y, 0.0)).collect()
}

#[test]
fn strips_a_byte_order_mark() {
    let soup = read("\u{feff}v 9 9 9\nv 0 0 0\nv 1 0 0\nf 1 2 3\n").expect("reads");
    assert_eq!(soup.vertex_count(), 3);
    assert_eq!(
        soup.position(VertexId::new(0)),
        Some(DVec3::new(9.0, 9.0, 9.0))
    );
}

#[test]
fn a_group_line_alone_starts_a_primitive() {
    let soup = read(&format!("{SQUARE}f 1 2 3\ng b\nf 1 3 4\n")).expect("reads");
    assert_eq!(sources(&soup, 1).expect("in range")[0].primitive, 1);
}

#[test]
fn refuses_each_free_form_statement() {
    for text in [
        "curv 0 1 1 2\n",
        "curv2 1 2\n",
        "cstype bspline\n",
        "surf 0 1 0 1 1\n",
    ] {
        let detail = parse_detail(text.as_bytes()).unwrap_or_default();
        assert!(detail.starts_with("line 1"), "{text:?}: {detail:?}");
    }
}

#[test]
fn joins_continued_lines_even_without_a_space_or_a_next_line() {
    let soup = read("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1\\\n2 3\n").expect("reads");
    assert_eq!(triangles(&soup), [[0, 1, 2]]);
    let soup = read("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3 \\\n").expect("reads");
    assert_eq!(triangles(&soup), [[0, 1, 2]]);
    let soup = read("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 \\  \n3\n").expect("reads");
    assert_eq!(triangles(&soup), [[0, 1, 2]]);
    let detail = parse_detail(b"v 0 0 0\nv 1 0 \\\n x\n").unwrap_or_default();
    assert!(detail.starts_with("line 2"), "{detail:?}");
}

#[test]
fn reads_three_number_texture_coordinates_and_negative_uv_indices() {
    let soup = read("v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0.25 0.5 0\nf 1/1 2/1 3/1\n").expect("reads");
    assert_eq!(
        soup.corner_uv(FaceId::new(0), 0),
        Some(DVec2::new(0.25, 0.5))
    );
    let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 5 5 5\nvt 0.1 0\nvt 0.2 0\nvt 0.3 0\n\
                f 1/-3 2/-2 3/-1\n";
    let soup = read(text).expect("reads");
    let uvs: Vec<_> = (0..3).map(|c| soup.corner_uv(FaceId::new(0), c)).collect();
    let expected = [0.1, 0.2, 0.3].map(|u| Some(DVec2::new(u, 0.0)));
    assert_eq!(uvs, expected);
}

#[test]
fn refuses_malformed_corners_and_texture_coordinates() {
    let cases: &[(&[u8], &str)] = &[
        (
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nf 1/-2 2/1 3/1\n",
            "line 5",
        ),
        (
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nf 1/1/1/1 2/1 3/1\n",
            "line 5",
        ),
        (b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1//x 2 3\n", "line 4"),
        (b"vt 0 0 0 0\n", "line 1"),
    ];
    for (text, line) in cases {
        let detail = parse_detail(text).unwrap_or_default();
        assert!(
            detail.starts_with(line),
            "{detail:?} does not start with {line:?}"
        );
    }
    assert_eq!(
        parse_detail(b"v \xff 0 0\n").as_deref(),
        Some("byte 2: not UTF-8 text")
    );
}

#[test]
fn error_messages_read_as_documented() {
    let e = obj::read(b"v nan 0 0\n").expect_err("non-finite");
    assert_eq!(
        e.to_string(),
        "source vertices with a NaN or infinite coordinate: 1"
    );
    let e = obj::read(b"vt\n").expect_err("malformed");
    assert_eq!(
        e.to_string(),
        "could not read the OBJ file: line 1: a texture coordinate needs one to three numbers"
    );
}

#[test]
fn clips_from_a_reflex_or_straight_first_corner() {
    for xy in [
        &[
            (1.0, 1.0),
            (1.0, 2.0),
            (0.0, 2.0),
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 1.0),
        ][..],
        &[(1.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (0.0, 0.0)][..],
    ] {
        let points = points_xy(xy);
        assert_tiles(&read(&polygon_text(&points)).expect("reads"), &points);
    }
}

#[test]
fn triangulates_deterministically() {
    let l_shape = "v 0 0 0\nv 2 0 0\nv 2 1 0\nv 1 1 0\nv 1 2 0\nv 0 2 0\nf 1 2 3 4 5 6\n";
    let cases: [(&str, &[[u32; 3]]); 4] = [
        (l_shape, &[[0, 1, 2], [0, 2, 3], [5, 0, 3], [3, 4, 5]]),
        (
            "v 0 0 0\nv 1 0 0\nv 2 0 0\nv 3 0 0\nv 4 0 0\nf 1 2 3 4 5\n",
            &[[0, 1, 2], [0, 2, 3], [0, 3, 4]],
        ),
        (
            &*format!("{SQUARE}f 1 2 3 4\n").leak(),
            &[[0, 1, 2], [0, 2, 3]],
        ),
        (
            "v 0 0 0\nv 1 0 0\nv 3 0 0\nv 2 0 0\nf 1 2 3 4\n",
            &[[0, 1, 3], [1, 2, 3]],
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(triangles(&read(text).expect("reads")), expected, "{text:?}");
    }
}

// A square hole cut into a square, its boundary joined to the outline by a bridge
// edge walked twice: the shape some exporters write for a face with a hole.
const KEYHOLE: [(f64, f64); 10] = [
    (0.0, 0.0),
    (10.0, 0.0),
    (10.0, 10.0),
    (0.0, 10.0),
    (0.0, 0.0),
    (3.0, 3.0),
    (3.0, 7.0),
    (7.0, 7.0),
    (7.0, 3.0),
    (3.0, 3.0),
];

#[test]
fn clips_a_keyhole_polygon_around_its_hole() {
    let points = points_xy(&KEYHOLE);
    assert_tiles(&read(&polygon_text(&points)).expect("reads"), &points);

    // The same outline with the bridge's ends written once and indexed twice.
    let mut text: String = points[..4]
        .iter()
        .chain(&points[5..9])
        .map(|p| format!("v {} {} {}\n", p.x, p.y, p.z))
        .collect();
    text.push_str("f 1 2 3 4 1 5 6 7 8 5\n");
    assert_tiles(&read(&text).expect("reads"), &points);
}

#[test]
fn refuses_a_polygon_that_crosses_itself() {
    let bowtie = points_xy(&[(0.0, 0.0), (1.0, 1.0), (1.0, 0.0), (0.0, 1.0)]);
    let figure_eight = points_xy(&[
        (0.0, 0.0),
        (2.0, 2.0),
        (4.0, 0.0),
        (4.0, 2.0),
        (2.0, 0.0),
        (0.0, 2.0),
    ]);
    for points in [bowtie, figure_eight] {
        let detail = parse_detail(polygon_text(&points).as_bytes()).unwrap_or_default();
        let line = format!("line {}:", points.len() + 1);
        assert!(detail.starts_with(&line), "{detail:?}");
    }
}

fn polygon(corners: usize) -> String {
    let mut text = String::new();
    for i in 0..corners {
        let a = std::f64::consts::TAU * i as f64 / corners as f64;
        text += &format!("v {} {} 0\n", a.cos(), a.sin());
    }
    text += "f";
    (1..=corners).for_each(|i| text += &format!(" {i}"));
    text
}

#[test]
fn a_polygon_over_the_corner_cap_is_refused_at_its_line_and_one_at_the_cap_is_read() {
    let big = polygon(1001);
    let line = big.lines().count();
    assert_eq!(
        parse_detail(big.as_bytes()).unwrap(),
        format!("line {line}: a face has 1001 corners, over the 1000 the reader triangulates")
    );
    assert_eq!(read(&polygon(1000)).unwrap().triangle_count(), 998);
}
