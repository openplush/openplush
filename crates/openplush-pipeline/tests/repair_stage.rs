//! The validate and repair stages: the mesh crate's report and repair, with every refusal
//! carried as `EngineError::Mesh`. Soups are read from OBJ text, as a shell would.

use std::collections::BTreeSet;

use openplush_io::obj;
use openplush_mesh::into_manifold;
use openplush_pipeline::{
    EngineError, ErrorLocations, LoopRef, MeshError, RepairPlan, SourceCorner, TriangleSoup,
    VertexId, repair, validate,
};

type Setup = Box<dyn std::error::Error>;

fn soup(text: &str) -> Result<TriangleSoup, Setup> {
    Ok(obj::read(text.as_bytes())?)
}

const TETRA: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 1 4 3\n";

/// Two triangles sharing an edge, one open boundary loop.
const PAIR: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n";

/// Three triangles on one edge, 1-2.
const BOOK: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nv 0 0 1\nf 1 2 3\nf 2 1 4\nf 1 2 5\n";

/// The tetrahedron with its last face wound the wrong way.
const FLIPPED: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 1 3 4\n";
/// The tetrahedron with its first vertex given twice, the copy used by the last face.
const UNWELDED: &str =
    "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nv 0 0 0\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 5 4 3\n";

#[test]
fn validate_is_the_mesh_crates_report() -> Result<(), Setup> {
    for text in [TETRA, PAIR, BOOK, FLIPPED, UNWELDED] {
        let s = soup(text)?;
        assert_eq!(validate(&s).value, openplush_mesh::validate(&s), "{text}");
    }
    Ok(())
}

#[test]
fn repair_follows_every_step_the_plan_turns_off() -> Result<(), Setup> {
    let off: [fn(&mut RepairPlan); 5] = [
        |p| p.weld = false,
        |p| p.drop_degenerates = false,
        |p| p.split_non_manifold_edges = false,
        |p| p.split_bowties = false,
        |p| p.fix_orientation = false,
    ];
    for text in [TETRA, PAIR, BOOK, FLIPPED, UNWELDED] {
        let s = soup(text)?;
        for turn_off in off {
            let mut plan = RepairPlan::default();
            turn_off(&mut plan);
            let staged = repair(&s, &plan).map(|staged| staged.value);
            assert_eq!(
                staged,
                into_manifold(&s, &plan).map_err(EngineError::Mesh),
                "{text} {plan:?}"
            );
        }
    }
    // The plan reaches repair: with orientation left alone, the flipped face is refused.
    let plan = RepairPlan {
        fix_orientation: false,
        ..RepairPlan::default()
    };
    let err = repair(&soup(FLIPPED)?, &plan)
        .err()
        .ok_or("a flipped face is refused")?;
    assert!(
        matches!(
            err,
            EngineError::Mesh(MeshError::InconsistentOrientation { .. })
        ),
        "{err:?}"
    );
    Ok(())
}

#[test]
fn validate_reports_and_never_warns() -> Result<(), Setup> {
    let clean = validate(&soup(TETRA)?);
    assert_eq!(clean.warnings.iter().count(), 0);
    assert!(clean.value.non_manifold_edges().is_empty());
    let book = validate(&soup(BOOK)?);
    assert_eq!(book.value.non_manifold_edges().len(), 1);
    assert_eq!(book.warnings.iter().count(), 0);
    Ok(())
}

#[test]
fn repair_is_the_mesh_crates_repair() -> Result<(), Setup> {
    for text in [TETRA, PAIR, BOOK] {
        let s = soup(text)?;
        let plan = RepairPlan::default();
        let staged = repair(&s, &plan)?;
        assert_eq!(staged.warnings.iter().count(), 0, "{text}");
        assert_eq!(staged.value, into_manifold(&s, &plan)?, "{text}");
    }
    Ok(())
}

#[test]
fn a_listed_loop_the_mesh_lacks_is_refused_as_mesh_naming_it_by_its_source_corner()
-> Result<(), Setup> {
    let corner = SourceCorner {
        primitive: 0,
        vertex: 9,
    };
    let plan = RepairPlan {
        fill_holes: BTreeSet::from([LoopRef::new(corner)]),
        ..RepairPlan::default()
    };
    let err = repair(&soup(PAIR)?, &plan)
        .err()
        .ok_or("a fill is refused")?;
    assert_eq!(
        err,
        EngineError::Mesh(MeshError::UnknownHoleLoop {
            loop_ref: LoopRef::new(corner)
        })
    );
    assert_eq!(err.code(), "Mesh");
    assert_eq!(
        err.locations(),
        ErrorLocations {
            source_vertices: vec![corner],
            ..ErrorLocations::default()
        }
    );
    Ok(())
}

#[test]
fn an_edge_repair_may_not_split_is_refused_naming_its_two_vertices() -> Result<(), Setup> {
    let plan = RepairPlan {
        split_non_manifold_edges: false,
        ..RepairPlan::default()
    };
    let err = repair(&soup(BOOK)?, &plan)
        .err()
        .ok_or("the book is refused")?;
    let EngineError::Mesh(MeshError::NonManifoldEdge { a, b }) = err else {
        panic!("{err:?}");
    };
    assert_eq!(
        err.locations().vertices,
        if a < b { vec![a, b] } else { vec![b, a] }
    );
    assert_eq!(
        [a, b]
            .map(VertexId::index)
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([0, 1])
    );
    Ok(())
}

#[test]
fn a_soup_with_no_triangles_locates_nothing() {
    let err = EngineError::Mesh(MeshError::NoTriangles);
    assert_eq!(err.locations(), ErrorLocations::default());
    assert_eq!(err.to_string(), "the soup has no triangles");
}
