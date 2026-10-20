//! The plan's defaults and ordering, and that each report accessor reads its own field.

use std::collections::BTreeSet;

use super::{HoleReport, LoopRef, RepairPlan, RepairReport};
use crate::{
    Component, DroppedCopy, EdgeId, FaceId, PairedEdge, SourceCorner, VertexCopy, VertexId,
};

fn f(i: u32) -> FaceId {
    FaceId::new(i)
}

fn v(i: u32) -> VertexId {
    VertexId::new(i)
}

fn corner(primitive: u32, vertex: u32) -> SourceCorner {
    SourceCorner { primitive, vertex }
}

#[test]
fn a_default_plan_runs_every_step_and_fills_nothing() {
    let plan = RepairPlan::default();
    assert!(plan.weld);
    assert!(plan.drop_degenerates);
    assert!(plan.split_non_manifold_edges);
    assert!(plan.split_bowties);
    assert!(plan.fix_orientation);
    assert!(plan.fill_holes.is_empty());
}

#[test]
fn loops_order_by_primitive_then_vertex_and_keep_their_corner() {
    let loops: BTreeSet<LoopRef> = [corner(1, 0), corner(0, 9), corner(0, 2), corner(1, 0)]
        .into_iter()
        .map(LoopRef::new)
        .collect();
    let in_order: Vec<SourceCorner> = loops.iter().map(|l| l.corner()).collect();
    assert_eq!(in_order, [corner(0, 2), corner(0, 9), corner(1, 0)]);
}

#[test]
fn an_empty_report_lists_nothing() {
    let r = RepairReport::default();
    assert!(r.welded().is_empty());
    assert!(r.repeated_index_faces().is_empty());
    assert!(r.duplicate_faces().is_empty());
    assert!(r.sheets().is_empty());
    assert!(r.stitched_caps().is_empty());
    assert!(r.retriangulated_faces().is_empty());
    assert!(r.paired_edges().is_empty());
    assert!(r.vertex_copies().is_empty());
    assert!(r.flipped_faces().is_empty());
    assert!(r.orientation_conflicts().is_empty());
    assert!(r.components().is_empty());
    assert!(r.holes().is_empty());
    assert!(r.mandatory_seams().is_empty());
    assert!(r.edge_copies().is_empty());
}

#[test]
fn each_accessor_returns_its_own_field() {
    let hole = HoleReport {
        loop_ref: LoopRef::new(corner(0, 3)),
        filled: true,
        added_faces: vec![f(40), f(41)],
    };
    let report = RepairReport {
        welded: vec![vec![v(1), v(2)]],
        repeated_index_faces: vec![f(3)],
        duplicate_faces: vec![DroppedCopy {
            face: f(4),
            of: f(5),
        }],
        sheets: vec![[f(6), f(7)]],
        stitched_caps: vec![f(8)],
        retriangulated_faces: vec![f(9)],
        paired_edges: vec![PairedEdge {
            vertices: [v(10), v(11)],
            pairs: vec![[f(12), f(13)]],
            unpaired: Some(f(14)),
        }],
        vertex_copies: vec![VertexCopy {
            of: v(15),
            copy: v(16),
        }],
        flipped_faces: vec![f(17)],
        orientation_conflicts: vec![[v(18), v(19)]],
        components: vec![Component {
            faces: vec![f(20)],
            detail_candidate: true,
        }],
        holes: vec![hole.clone()],
        mandatory_seams: vec![[v(21), v(22)]],
        edge_copies: vec![vec![EdgeId::new(23), EdgeId::new(24)]],
    };
    assert_eq!(report.welded(), [vec![v(1), v(2)]]);
    assert_eq!(report.repeated_index_faces(), [f(3)]);
    let dropped = &report.duplicate_faces()[0];
    assert_eq!((dropped.face, dropped.of), (f(4), f(5)));
    assert_eq!(report.sheets(), [[f(6), f(7)]]);
    assert_eq!(report.stitched_caps(), [f(8)]);
    assert_eq!(report.retriangulated_faces(), [f(9)]);
    assert_eq!(report.paired_edges()[0].unpaired, Some(f(14)));
    let copy = &report.vertex_copies()[0];
    assert_eq!((copy.of, copy.copy), (v(15), v(16)));
    assert_eq!(report.flipped_faces(), [f(17)]);
    assert_eq!(report.orientation_conflicts(), [[v(18), v(19)]]);
    assert!(report.components()[0].detail_candidate);
    assert_eq!(report.holes(), [hole]);
    assert_eq!(report.mandatory_seams(), [[v(21), v(22)]]);
    assert_eq!(
        report.edge_copies(),
        [vec![EdgeId::new(23), EdgeId::new(24)]]
    );
}

#[test]
fn a_hole_report_reads_its_own_fields() {
    let open = HoleReport {
        loop_ref: LoopRef::new(corner(2, 5)),
        filled: false,
        added_faces: Vec::new(),
    };
    assert_eq!(open.loop_ref().corner(), corner(2, 5));
    assert!(!open.filled());
    assert!(open.added_faces().is_empty());
    let filled = HoleReport {
        filled: true,
        added_faces: vec![f(1)],
        ..open
    };
    assert!(filled.filled());
    assert_eq!(filled.added_faces(), [f(1)]);
}
