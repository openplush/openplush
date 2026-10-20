//! Changing repair settings keeps every seam on the same edge: a key written against one
//! repair of a model resolves, against another repair of it, to the edges between the same
//! two points. The weld may instead leave a key unresolved, but never on a wrong edge.

use openplush_io::obj;
use openplush_mesh::{EdgeId, ManifoldMesh, RepairPlan, VertexId, into_manifold};
use openplush_project::Provenance;
use openplush_testkit::fixtures::{self, Kind};

type Setup = Box<dyn std::error::Error>;

fn repaired(kind: Kind, name: &str, plan: &RepairPlan) -> Result<ManifoldMesh, Setup> {
    let soup = obj::read(&fixtures::load(kind, name)?)?;
    Ok(into_manifold(&soup, plan)?.0)
}

fn ends(m: &ManifoldMesh, e: EdgeId) -> [[u64; 3]; 2] {
    let h = e.half_edge();
    let bits = |v: VertexId| {
        let p = m.position(v);
        [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
    };
    let mut ends = [bits(m.origin(h)), bits(m.dest(h))];
    ends.sort();
    ends
}

fn edges(m: &ManifoldMesh) -> Result<Vec<EdgeId>, Setup> {
    Ok((0..u32::try_from(m.edge_count())?)
        .map(EdgeId::new)
        .collect())
}

/// Every key written against `from` that resolves in `to` lands on edges with the same two
/// ends; returns how many resolved.
fn land_on_the_same_edges(from: &ManifoldMesh, to: &ManifoldMesh) -> Result<usize, Setup> {
    let (written, read) = (Provenance::new(from), Provenance::new(to));
    let mut resolved = 0;
    for e in edges(from)? {
        let key = written.edge_key(e).ok_or("an edge has no key")?;
        let landed = read.resolve_edge(key);
        for &l in &landed {
            assert_eq!(ends(to, l), ends(from, e), "{key} moved");
        }
        resolved += usize::from(!landed.is_empty());
    }
    Ok(resolved)
}

fn each_step_off() -> Vec<RepairPlan> {
    let on = RepairPlan::default();
    vec![
        RepairPlan {
            weld: false,
            ..on.clone()
        },
        RepairPlan {
            drop_degenerates: false,
            ..on.clone()
        },
        RepairPlan {
            split_non_manifold_edges: false,
            ..on.clone()
        },
        RepairPlan {
            split_bowties: false,
            ..on.clone()
        },
        RepairPlan {
            fix_orientation: false,
            ..on
        },
    ]
}

#[test]
fn on_a_clean_model_every_step_toggle_keeps_every_seam() -> Result<(), Setup> {
    for name in fixtures::names(Kind::Mesh)? {
        let base = repaired(Kind::Mesh, &name, &RepairPlan::default())?;
        for plan in each_step_off() {
            let other = repaired(Kind::Mesh, &name, &plan)?;
            assert_eq!(
                land_on_the_same_edges(&base, &other)?,
                base.edge_count(),
                "{name} {plan:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_weld_toggle_never_moves_a_seam() -> Result<(), Setup> {
    let welded = repaired(
        Kind::Defect,
        "unwelded-split-vertices",
        &RepairPlan::default(),
    )?;
    let plan = RepairPlan {
        weld: false,
        ..RepairPlan::default()
    };
    let unwelded = repaired(Kind::Defect, "unwelded-split-vertices", &plan)?;
    assert_ne!(welded.vertex_count(), unwelded.vertex_count());
    assert!(land_on_the_same_edges(&welded, &unwelded)? > 0);
    assert!(land_on_the_same_edges(&unwelded, &welded)? > 0);
    Ok(())
}
