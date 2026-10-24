//! How a chart's refusal becomes the stage's: a dart's chart vertices become mesh edges; and
//! how a chart's stop becomes a warning on its piece.
#![allow(clippy::expect_used, clippy::panic, reason = "test setup")]

use std::collections::BTreeMap;

use openplush_io::obj;
use openplush_mesh::{EdgeId, FaceId, ManifoldMesh, RepairPlan, VertexId, into_manifold};
use openplush_seams::{ChartId, ChartMesh, ProposalKind, SeamKind, SeamSet};

use openplush_flatten::StoppedBy;
use openplush_pattern::PieceId;

use super::{chart_edge, compression_warning, refusal, stop_warning};
use crate::{EngineError, FlattenError, Remedy, WarningTarget, chart, flatten};

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

#[test]
fn every_side_of_every_chart_face_is_found_from_either_end() {
    let (mesh, c) = fan();
    let to_global = c.vtx_to_global();
    for (_, corners) in c.faces() {
        for i in 0..3 {
            let (a, b) = (corners[i], corners[(i + 1) % 3]);
            let side = edge(
                &mesh,
                to_global[a.index() as usize],
                to_global[b.index() as usize],
            );
            assert_eq!(chart_edge(&mesh, &c, a, b), Some(side), "{a:?} to {b:?}");
            assert_eq!(chart_edge(&mesh, &c, b, a), Some(side), "{b:?} to {a:?}");
        }
    }
}

/// A 3 by 3 grid of points with its eight triangles on both sides: the rim edges join the
/// layers, and an inside edge between two rim points comes back as two parallel copies.
const TWO_LAYERS: &str = "v 0 0 0\nv 10 0 0\nv 20 0 0\nv 0 10 0\nv 10 10 0\nv 20 10 0\n\
v 0 20 0\nv 10 20 0\nv 20 20 0\n\
f 1 2 5\nf 1 5 4\nf 2 3 6\nf 2 6 5\nf 4 5 8\nf 4 8 7\nf 5 6 9\nf 5 9 8\n\
f 1 5 2\nf 1 4 5\nf 2 6 3\nf 2 5 6\nf 4 8 5\nf 4 7 8\nf 5 9 6\nf 5 8 9\n";

/// The chart vertex at `(x, y)`.
fn at(chart: &ChartMesh, x: f64, y: f64) -> VertexId {
    let i = chart
        .xyz()
        .iter()
        .position(|p| (p.x, p.y) == (x, y))
        .expect("a chart vertex there");
    VertexId::new(u32::try_from(i).expect("small"))
}

/// The mesh edge between `a` and `b` that bounds one of `chart`'s faces.
fn edge_of(mesh: &ManifoldMesh, chart: &ChartMesh, a: VertexId, b: VertexId) -> EdgeId {
    let inside = |f| chart.faces().iter().any(|&(g, _)| Some(g) == f);
    mesh.outgoing(a)
        .filter(|&h| mesh.dest(h) == b)
        .find(|&h| inside(mesh.face(h)) || inside(mesh.face(h.twin())))
        .map(|h| h.edge())
        .expect("an edge of the chart")
}

#[test]
fn a_dart_over_a_parallel_edge_names_the_copy_inside_its_own_chart() {
    let soup = obj::read(TWO_LAYERS.as_bytes()).expect("the sheet reads");
    let (mesh, _) = into_manifold(&soup, &RepairPlan::default()).expect("the sheet repairs");
    let rim = [
        (0., 0.),
        (10., 0.),
        (20., 0.),
        (20., 10.),
        (20., 20.),
        (10., 20.),
    ];
    let rim = [&rim[..], &[(0., 20.), (0., 10.), (0., 0.)]].concat();
    let mut seams = SeamSet::new();
    for pair in rim.windows(2) {
        let (a, b) = (
            global(&mesh, pair[0].0, pair[0].1),
            global(&mesh, pair[1].0, pair[1].1),
        );
        seams
            .insert(edge(&mesh, a, b), SeamKind::Cut)
            .expect("a seam");
    }
    let (charts, _) = chart(&mesh, &seams).expect("two layers").value;
    let mut copies = Vec::new();
    for c in &charts {
        let path = vec![at(c, 20., 10.), at(c, 10., 0.), at(c, 10., 10.)];
        let to_global = |v: VertexId| c.vtx_to_global()[v.index() as usize];
        let EngineError::BoundaryNotRealizable { proposals, .. } =
            refusal(&mesh, c, not_realizable(c.chart(), Some(path.clone())))
        else {
            panic!("still BoundaryNotRealizable");
        };
        let [dart] = &proposals[..] else {
            panic!("one proposal: {proposals:?}");
        };
        assert_eq!(*dart.kind(), ProposalKind::Dart { chart: c.chart() });
        let copy = edge_of(&mesh, c, to_global(path[0]), to_global(path[1]));
        let expected: BTreeMap<EdgeId, SeamKind> = [
            (copy, SeamKind::Dart),
            (
                edge_of(&mesh, c, to_global(path[1]), to_global(path[2])),
                SeamKind::Dart,
            ),
        ]
        .into();
        assert_eq!(*dart.add(), expected, "chart {:?}", c.chart());
        copies.push(copy);
    }
    assert_eq!(charts.len(), 2);
    assert_ne!(copies[0], copies[1], "the layers share no copy of the edge");
}

#[test]
fn a_stop_at_the_cap_warns_on_the_piece_numbered_as_its_chart() {
    let w = stop_warning(ChartId::new(4), StoppedBy::IterationCap).expect("a warning");
    assert_eq!(w.code().as_str(), "flatten-iteration-cap");
    assert_eq!(w.target(), WarningTarget::Piece(PieceId::new(4)));
    assert_eq!(
        w.explanation(),
        "This piece stopped flattening at the iteration limit before it settled, so the \
         distortion shown may be more than the fabric must take."
    );
    let Remedy::Advisory { what_to_change } = w.remedy() else {
        panic!("advisory: {:?}", w.remedy());
    };
    assert_eq!(
        what_to_change.as_str(),
        "Add a seam or a dart in this piece, or keep it as it is: the layout is valid, only \
         less relaxed than it could be."
    );
}

#[test]
fn a_stop_that_settles_or_finds_no_descent_warns_of_nothing() {
    for stopped in [StoppedBy::Settled, StoppedBy::NoDescent] {
        assert_eq!(stop_warning(ChartId::new(4), stopped), None, "{stopped:?}");
    }
}

#[test]
fn the_stage_raises_nothing_for_a_chart_that_settles() {
    let (mesh, one) = fan();
    let staged = flatten(&mesh, &[one]).expect("the fan flattens");
    assert_eq!(staged.warnings.iter().count(), 0);
}

#[test]
fn a_piece_compressed_past_the_limit_warns_with_its_share_and_one_at_it_does_not() {
    assert_eq!(compression_warning(ChartId::new(4), 0.15), None);
    let w = compression_warning(ChartId::new(4), 0.364).expect("a warning");
    assert_eq!(w.code().as_str(), "piece-compressed");
    assert_eq!(w.target(), WarningTarget::Piece(PieceId::new(4)));
    assert_eq!(
        w.explanation(),
        "This piece is squeezed by about 36% to lie flat, so it will sew up flatter than the \
         model."
    );
    let Remedy::Advisory { what_to_change } = w.remedy() else {
        panic!("advisory: {:?}", w.remedy());
    };
    assert_eq!(
        what_to_change.as_str(),
        "Add a dart or a seam in this piece."
    );
}
