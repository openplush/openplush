use std::collections::BTreeMap;

use openplush_geom::DVec3;
use openplush_mesh::VertexId;
use openplush_seams::ChartId;

use super::not_realizable;
use crate::FlattenError;
use crate::boundary::fixtures::{Faces, rings};

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
        FlattenError::BoundaryNotRealizable {
            residual, dart, ..
        } => Some((residual, dart)),
        _ => None,
    }
}

/// Shortest edge-path length from any rim vertex to vertex 0, by plain Bellman–Ford.
fn shortest_from_rim(xyz: &[DVec3], faces: &Faces) -> f64 {
    let mut dist = vec![f64::INFINITY; xyz.len()];
    let rim_start = 1 + 3 * (RINGS - 1) * (RINGS - 2) + 6 * (RINGS - 1) + 3 * 0;
    let _ = rim_start;
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
    assert_eq!(dart.and_then(|p| p.last().copied()), Some(VertexId::new(10)));
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
    assert_eq!(first, parts(not_realizable(CHART, &xyz, &faces)).expect("again"));
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
