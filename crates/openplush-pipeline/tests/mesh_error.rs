//! `MeshError`'s messages, which `EngineError::Mesh` prints unchanged, are pinned.

use openplush_pipeline::{
    CornerArray, Counted, FaceId, HalfEdgeId, LoopRef, MaterialFault, MeshElement, MeshError,
    SourceCorner, VertexId,
};

#[test]
fn messages_about_the_soup_as_a_whole_are_pinned() {
    for (e, message) in [
        (
            MeshError::CornerCountMismatch {
                array: CornerArray::Uvs,
                expected: 6,
                found: 3,
            },
            "expected 6 per-corner Uvs (three per triangle), found 3",
        ),
        (
            MeshError::TooLarge {
                what: Counted::Triangles,
                count: 5,
            },
            "5 Triangles exceed the u32 index space",
        ),
        (
            MeshError::InvalidMaterial {
                primitive: 2,
                fault: MaterialFault::OutOfRange,
            },
            "source primitive 2's material entry is invalid: OutOfRange",
        ),
        (MeshError::NoTriangles, "the soup has no triangles"),
        (
            MeshError::UnusableExtent,
            "the soup's extent is zero or too large to derive tolerances from",
        ),
    ] {
        assert_eq!(e.to_string(), message);
    }
}

#[test]
fn messages_naming_faces_are_pinned() {
    let (v, f) = (VertexId::new(1), FaceId::new(7));
    for (e, message) in [
        (
            MeshError::NonFiniteVertex { vertex: v },
            "vertex VertexId(1) has a non-finite coordinate",
        ),
        (
            MeshError::IndexOutOfRange {
                triangle: f,
                index: 9,
                vertex_count: 3,
            },
            "triangle FaceId(7) uses vertex index 9, but there are only 3 vertices",
        ),
        (
            MeshError::DegenerateTriangle { triangle: f },
            "triangle FaceId(7) repeats a vertex",
        ),
        (
            MeshError::ZeroAreaTriangle { triangle: f },
            "triangle FaceId(7) has (near-)zero area",
        ),
        (
            MeshError::DuplicateTriangle {
                triangle: f,
                first: FaceId::new(2),
            },
            "triangle FaceId(7) repeats the vertices of triangle FaceId(2)",
        ),
    ] {
        assert_eq!(e.to_string(), message);
    }
}

#[test]
fn messages_naming_vertices_are_pinned() {
    let (v, w, f) = (VertexId::new(1), VertexId::new(4), FaceId::new(7));
    for (e, message) in [
        (
            MeshError::IsolatedVertex { vertex: v },
            "vertex VertexId(1) is not used by any triangle",
        ),
        (
            MeshError::NonManifoldEdge { a: v, b: w },
            "edge between VertexId(1) and VertexId(4) is shared by more than two triangles",
        ),
        (
            MeshError::InconsistentOrientation { a: v, b: w },
            "triangles sharing the edge between VertexId(1) and VertexId(4) have opposite orientations",
        ),
        (
            MeshError::PairingMismatch {
                face: f,
                a: v,
                b: w,
            },
            "a pairing on the edge between VertexId(1) and VertexId(4) names triangle FaceId(7) wrongly",
        ),
        (
            MeshError::NonManifoldVertex { vertex: v },
            "vertex VertexId(1) is non-manifold (its triangles form more than one fan)",
        ),
        (
            MeshError::BrokenInvariant {
                invariant: 3,
                element: MeshElement::HalfEdge(HalfEdgeId::new(6)),
            },
            "half-edge invariant 3 is broken at HalfEdge(HalfEdgeId(6))",
        ),
    ] {
        assert_eq!(e.to_string(), message);
    }
}

#[test]
fn messages_naming_a_loop_are_pinned() {
    let loop_ref = LoopRef::new(SourceCorner {
        primitive: 0,
        vertex: 3,
    });
    assert_eq!(
        MeshError::UnknownHoleLoop { loop_ref }.to_string(),
        "the plan fills the loop LoopRef(SourceCorner { primitive: 0, vertex: 3 }), which the mesh does not have"
    );
    assert_eq!(
        MeshError::AmbiguousHoleLoop { loop_ref }.to_string(),
        "the plan fills the loop LoopRef(SourceCorner { primitive: 0, vertex: 3 }), but more than one hole has that name, so none was filled"
    );
    assert_eq!(
        MeshError::HoleNotFillable { loop_ref }.to_string(),
        "the loop LoopRef(SourceCorner { primitive: 0, vertex: 3 }) has no fill: every triangulation of it reuses an edge, has no area, or does not solve"
    );
    assert_eq!(
        MeshError::HoleOnSplit { loop_ref }.to_string(),
        "the loop LoopRef(SourceCorner { primitive: 0, vertex: 3 }) runs along a split edge, so it is one side of a seam, not a hole"
    );
    assert_eq!(
        MeshError::HoleTooLarge {
            loop_ref,
            vertices: 600
        }
        .to_string(),
        "the loop LoopRef(SourceCorner { primitive: 0, vertex: 3 }) has 600 vertices, more than the 512 a fill takes"
    );
}
