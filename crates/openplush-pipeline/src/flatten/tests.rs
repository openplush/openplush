//! How a chart's refusal becomes the stage's: a dart's chart vertices become mesh edges.
#![allow(clippy::expect_used, clippy::panic, reason = "test setup")]

use std::collections::BTreeMap;

use openplush_io::obj;
use openplush_mesh::{EdgeId, FaceId, ManifoldMesh, RepairPlan, VertexId, into_manifold};
use openplush_seams::{ChartId, ChartMesh, ProposalKind, SeamKind, SeamSet};

use super::refusal;
use crate::{EngineError, FlattenError, chart};

/// A square of side 10 split into four triangles round its centre, the one interior vertex.
const FAN: &str = "v 0 0 0\nv 10 0 0\nv 10 10 0\nv 0 10 0\nv 5 5 0\n\
f 1 2 5\nf 2 3 5\nf 3 4 5\nf 4 1 5\n";

fn fan() -> (ManifoldMesh, ChartMesh) {
    let soup = obj::read(FAN.as_bytes()).expect("the fan reads");
    let (mesh, _) = into_manifold(&soup, &RepairPlan::default()).expect("the fan repairs");
    let (mut charts, _) = chart(&mesh, &SeamSet::new()).expect("one chart").value;
    let one = charts.pop().expect("one chart");
    (mesh, one)
}

fn global(mesh: &ManifoldMesh, x: f64, y: f64) -> VertexId {
    (0..mesh.vertex_count())
        .map(|v| VertexId::new(u32::try_from(v).expect("small")))
        .find(|&v| (mesh.position(v).x, mesh.position(v).y) == (x, y))
        .expect("a vertex there")
}

fn local(chart: &ChartMesh, v: VertexId) -> VertexId {
    let i = chart
        .vtx_to_global()
        .iter()
        .position(|&g| g == v)
        .expect("in the chart");
    VertexId::new(u32::try_from(i).expect("small"))
}

fn edge(mesh: &ManifoldMesh, a: VertexId, b: VertexId) -> EdgeId {
    mesh.outgoing(a)
        .find(|&h| mesh.dest(h) == b)
        .map(|h| h.edge())
        .expect("an edge")
}

fn not_realizable(chart: ChartId, dart: Option<Vec<VertexId>>) -> FlattenError {
    FlattenError::BoundaryNotRealizable {
        chart,
        residual: 0.5,
        dart,
    }
}

#[test]
fn a_dart_path_becomes_one_dart_proposal_on_the_mesh_edges_it_walks() {
    let (mesh, c) = fan();
    let (corner, centre, far) = (
        global(&mesh, 0., 0.),
        global(&mesh, 5., 5.),
        global(&mesh, 10., 10.),
    );
    let path = [corner, centre, far].map(|v| local(&c, v)).to_vec();
    let EngineError::BoundaryNotRealizable {
        chart,
        residual,
        proposals,
    } = refusal(&mesh, &c, not_realizable(c.chart(), Some(path)))
    else {
        panic!("still BoundaryNotRealizable");
    };
    assert_eq!((chart, residual), (c.chart(), 0.5));
    let [dart] = &proposals[..] else {
        panic!("one proposal: {proposals:?}");
    };
    assert_eq!(*dart.kind(), ProposalKind::Dart { chart: c.chart() });
    let expected: BTreeMap<EdgeId, SeamKind> = [
        (edge(&mesh, corner, centre), SeamKind::Dart),
        (edge(&mesh, centre, far), SeamKind::Dart),
    ]
    .into();
    assert_eq!(*dart.add(), expected);
    assert!(!dart.explanation().is_empty());
}

#[test]
fn a_refusal_without_a_dart_or_with_a_lone_vertex_offers_no_proposal() {
    let (mesh, c) = fan();
    let lone = vec![local(&c, global(&mesh, 0., 0.))];
    for dart in [None, Some(lone), Some(Vec::new())] {
        let EngineError::BoundaryNotRealizable { proposals, .. } =
            refusal(&mesh, &c, not_realizable(c.chart(), dart))
        else {
            panic!("still BoundaryNotRealizable");
        };
        assert!(proposals.is_empty(), "{proposals:?}");
    }
}

#[test]
fn a_flip_keeps_its_faces_and_any_other_refusal_is_carried_whole() {
    let (mesh, c) = fan();
    let faces = vec![FaceId::new(2), FaceId::new(0)];
    assert_eq!(
        refusal(
            &mesh,
            &c,
            FlattenError::Flip {
                chart: c.chart(),
                faces: faces.clone(),
            }
        ),
        EngineError::Flip {
            chart: c.chart(),
            faces,
        }
    );
    let degenerate = FlattenError::Degenerate {
        chart: c.chart(),
        face: FaceId::new(1),
    };
    assert_eq!(
        refusal(&mesh, &c, degenerate.clone()),
        EngineError::Flatten(degenerate)
    );
}
