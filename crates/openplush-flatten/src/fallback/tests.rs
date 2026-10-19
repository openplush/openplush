use std::collections::BTreeMap;

use openplush_geom::DVec3;
use openplush_mesh::VertexId;
use openplush_seams::ChartId;

use super::{dart, not_realizable};
use crate::FlattenError;
use crate::boundary::Open;
use crate::boundary::fixtures::{Faces, faces as chart_faces, rings};

const CHART: ChartId = ChartId::new(4);
const RINGS: u32 = 6;

/// A cone of height 0.8 over the unit disk, apex at vertex 0.
fn cone() -> (Vec<DVec3>, Faces) {
    rings(RINGS, |r, a| {
        DVec3::new(r * a.cos(), r * a.sin(), 0.8 * (1.0 - r))
    })
}

fn edge_length(xyz: &[DVec3], a: VertexId, b: VertexId) -> f64 {
    (xyz[a.index() as usize] - xyz[b.index() as usize]).length()
}

fn parts(error: FlattenError) -> Option<(f64, Option<Vec<VertexId>>)> {
    match error {
        FlattenError::BoundaryNotRealizable { residual, dart, .. } => Some((residual, dart)),
        _ => None,
    }
}

/// Shortest edge-path length from any rim vertex to vertex 0, by plain Bellman–Ford.
fn shortest_from_rim(xyz: &[DVec3], faces: &Faces) -> f64 {
    let mut dist = vec![f64::INFINITY; xyz.len()];
    let first_rim = (1 + 3 * RINGS * (RINGS - 1)) as usize;
    dist[first_rim..].fill(0.0);
    for _ in 0..xyz.len() {
        for (_, t) in faces {
            for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                let (u, v) = (t[a], t[b]);
                let w = edge_length(xyz, u, v);
                let (iu, iv) = (u.index() as usize, v.index() as usize);
                dist[iu] = dist[iu].min(dist[iv] + w);
                dist[iv] = dist[iv].min(dist[iu] + w);
            }
        }
    }
    dist[0]
}

#[test]
fn the_dart_runs_the_shortest_way_from_the_rim_to_the_cone_tip() {
    let (xyz, faces) = cone();
    let (_, dart) = parts(not_realizable(CHART, &xyz, &faces)).expect("BoundaryNotRealizable");
    let path = dart.expect("a chart with an interior vertex has a dart");
    let first_rim = 1 + 3 * RINGS * (RINGS - 1);
    assert!(path.first().is_some_and(|v| v.index() >= first_rim));
    assert_eq!(path.last(), Some(&VertexId::new(0)));
    // Every step is an edge of some face.
    let edges: BTreeMap<(u32, u32), ()> = faces
        .iter()
        .flat_map(|(_, t)| {
            let t = t.map(VertexId::index);
            [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])].map(|(a, b)| ((a.min(b), a.max(b)), ()))
        })
        .collect();
    for pair in path.windows(2) {
        let (a, b) = (pair[0].index(), pair[1].index());
        assert!(edges.contains_key(&(a.min(b), a.max(b))), "{a}-{b}");
    }
    let length: f64 = path.windows(2).map(|p| edge_length(&xyz, p[0], p[1])).sum();
    assert!((length - shortest_from_rim(&xyz, &faces)).abs() <= 1e-12);
}

#[test]
fn the_dart_heads_for_the_vertex_with_the_most_angle_defect() {
    // A flat disk with one raised vertex off the middle: that vertex is the only cone.
    let (mut xyz, faces) = rings(RINGS, |r, a| DVec3::new(r * a.cos(), r * a.sin(), 0.0));
    xyz[10].z = 0.3;
    let (_, dart) = parts(not_realizable(CHART, &xyz, &faces)).expect("BoundaryNotRealizable");
    assert_eq!(
        dart.and_then(|p| p.last().copied()),
        Some(VertexId::new(10))
    );
}

#[test]
fn a_chart_with_no_interior_vertex_has_nothing_to_cut_toward() {
    let xyz = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(3.0, 0.0, 0.0),
        DVec3::new(3.0, 0.0, 4.0),
    ];
    let faces = crate::boundary::fixtures::faces(&[[0, 1, 2]]);
    let (_, dart) = parts(not_realizable(CHART, &xyz, &faces)).expect("BoundaryNotRealizable");
    assert_eq!(dart, None);
}

#[test]
fn the_residual_is_the_natural_gap_in_model_units_and_repeats_bit_for_bit() {
    let (mut xyz, faces) = cone();
    xyz[3].x += 0.08;
    let first = parts(not_realizable(CHART, &xyz, &faces)).expect("BoundaryNotRealizable");
    assert!(first.0 > 1e-4);
    assert_eq!(
        first,
        parts(not_realizable(CHART, &xyz, &faces)).expect("again")
    );
    let big: Vec<DVec3> = xyz.iter().map(|&p| p * 1000.0).collect();
    let (scaled, _) = parts(not_realizable(CHART, &big, &faces)).expect("scaled");
    assert!((scaled / 1000.0 - first.0).abs() <= 1e-9 * first.0);
}

#[test]
fn a_chart_that_cannot_be_measured_is_refused_as_such_not_as_unclosed() {
    let (xyz, _) = cone();
    assert_eq!(
        not_realizable(CHART, &xyz, &[]),
        FlattenError::EmptyChart { chart: CHART }
    );
}

/// Vertices at `points` (index = vertex) and the triangles `tris`; with `open` listing the rim
/// and the angle sum of every vertex a triangle uses.
fn graph(
    points: &[[f64; 3]],
    tris: &[[u32; 3]],
    rim: &[u32],
    angles: &[(u32, f64)],
) -> (Vec<DVec3>, Faces, Open) {
    let xyz = points
        .iter()
        .map(|p| DVec3::new(p[0], p[1], p[2]))
        .collect();
    let open = Open {
        ring: rim.to_vec(),
        gap: 0.0,
        angle_sum: angles.iter().copied().collect(),
    };
    (xyz, chart_faces(tris), open)
}

fn ids(path: Option<Vec<VertexId>>) -> Option<Vec<u32>> {
    path.map(|p| p.iter().map(|v| v.index()).collect())
}

/// Vertex 0 is the tip. Rim vertex 1 reaches it by 0.1 + 2.0 and rim vertex 2 by 1.2 + 1.2;
/// vertex 9 is far off and only supplies triangles.
fn two_routes() -> (Vec<DVec3>, Faces, Open) {
    let mut points = vec![[0.0; 3]; 10];
    points[3] = [2.0, 0.0, 0.0];
    points[1] = [2.0, 0.1, 0.0];
    points[4] = [0.0, 1.2, 0.0];
    points[2] = [1.2, 1.2, 0.0];
    points[9] = [0.0, 0.0, 1000.0];
    let angles = [(0, 1.0), (1, 3.0), (2, 3.0), (3, 6.0), (4, 6.0), (9, 6.0)];
    graph(
        &points,
        &[[1, 3, 9], [3, 0, 9], [2, 4, 9], [4, 0, 9]],
        &[1, 2],
        &angles,
    )
}

#[test]
fn the_dart_follows_true_edge_lengths_not_their_squares() {
    let (xyz, faces, open) = two_routes();
    assert_eq!(ids(dart(&xyz, &faces, &open)), Some(vec![1, 3, 0]));
}

#[test]
fn equal_angle_defects_end_the_dart_at_the_smaller_vertex() {
    let (xyz, faces, mut open) = two_routes();
    open.angle_sum.insert(3, 1.0);
    assert_eq!(
        ids(dart(&xyz, &faces, &open)).and_then(|p| p.last().copied()),
        Some(0)
    );
}

#[test]
fn equal_routes_keep_the_one_found_first() {
    // Mirror-image routes 1 -> 3 -> 0 and 2 -> 4 -> 0, each 0.1 + 1.0 long.
    let mut points = vec![[0.0; 3]; 10];
    points[3] = [1.0, 0.0, 0.0];
    points[1] = [1.0, 0.1, 0.0];
    points[4] = [-1.0, 0.0, 0.0];
    points[2] = [-1.0, 0.1, 0.0];
    points[9] = [0.0, 0.0, 1000.0];
    let angles = [(0, 1.0), (1, 3.0), (2, 3.0), (3, 6.0), (4, 6.0), (9, 6.0)];
    let (xyz, faces, open) = graph(
        &points,
        &[[1, 3, 9], [3, 0, 9], [2, 4, 9], [4, 0, 9]],
        &[1, 2],
        &angles,
    );
    assert_eq!(ids(dart(&xyz, &faces, &open)), Some(vec![1, 3, 0]));
}

#[test]
fn the_message_states_the_gap() {
    let (xyz, faces) = cone();
    let error = not_realizable(CHART, &xyz, &faces);
    let (residual, _) = parts(error.clone()).expect("BoundaryNotRealizable");
    assert!(
        error
            .to_string()
            .contains(&format!("misses closing by {residual:e}"))
    );
}
