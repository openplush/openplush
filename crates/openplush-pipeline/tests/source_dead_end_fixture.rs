//! The dead-end fixture's darts, imported and repaired as a shell would, as glTF and as OBJ: one
//! per tip in `dead-end.tips.json`, matched by position, in ascending tip order, each a two-edge
//! cut, so the cut that ends on the material boundary and the opening gives none; and no dart
//! edge among the source seams. Here rather than in `openplush-seams` because reading the files
//! needs `openplush-io`, a sibling of that crate.

use std::collections::BTreeSet;

use openplush_geom::DVec3;
use openplush_pipeline::{
    EdgeId, ImportFile, ImportFiles, ManifoldMesh, ProposalKind, RepairPlan, SeamKind, SeamReason,
    SeamSet, VertexId, import, repair,
};
use openplush_seams::uv_cuts;

type Setup = Box<dyn std::error::Error>;

const DEAD_END_GLB: &[u8] = include_bytes!("../../../fixtures/meshes/source-seams/dead-end.glb");
const DEAD_END_OBJ: &[u8] = include_bytes!("../../../fixtures/meshes/source-seams/dead-end.obj");
/// The tips, as 0-based vertex indices into `DEAD_END_OBJ`.
const TIPS: &str = include_str!("../../../fixtures/seams/non-disk/dead-end.tips.json");

/// Matches a fixture vertex to the mesh's: well above `f32` rounding of the glTF positions
/// (millimetres, under 100 in size) and far below the capsule's shortest edge.
const SAME_POINT_MM: f64 = 1e-3;

fn mesh_of(name: &str, bytes: &[u8]) -> Result<ManifoldMesh, Setup> {
    let soup = import(&ImportFiles {
        primary: ImportFile {
            name: name.into(),
            bytes: bytes.to_vec(),
        },
        others: Vec::new(),
    })?
    .value;
    Ok(repair(&soup, &RepairPlan::default())?.value.0)
}

/// The OBJ's `v` positions, in file order.
fn obj_positions() -> Result<Vec<DVec3>, Setup> {
    std::str::from_utf8(DEAD_END_OBJ)?
        .lines()
        .filter_map(|line| line.strip_prefix("v "))
        .map(|rest| {
            let c = rest
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<Vec<f64>, _>>()?;
            match c[..] {
                [x, y, z] => Ok(DVec3::new(x, y, z)),
                _ => Err(format!("not a position: {rest:?}").into()),
            }
        })
        .collect()
}

/// The recorded tips as OBJ vertex indices.
fn tip_indices() -> Result<Vec<usize>, Setup> {
    assert!(TIPS.contains(r#""format": "openplush-dead-ends""#));
    assert!(TIPS.contains(r#""vertexIndexBase": 0"#));
    let (_, rest) = TIPS.split_once(r#""tips": ["#).ok_or("a tips list")?;
    let inside = rest.split(']').next().ok_or("a closed tips list")?;
    let tips = inside
        .split(',')
        .map(|n| n.trim().parse())
        .collect::<Result<Vec<usize>, _>>()?;
    assert_eq!(tips.len(), 2, "the fixture records two tips");
    Ok(tips)
}

/// The one mesh vertex at `p`.
fn vertex_at(mesh: &ManifoldMesh, p: DVec3) -> Result<VertexId, Setup> {
    let near: Vec<VertexId> = (0..u32::try_from(mesh.vertex_count())?)
        .map(VertexId::new)
        .filter(|&v| (mesh.position(v) - p).length() <= SAME_POINT_MM)
        .collect();
    match near[..] {
        [v] => Ok(v),
        _ => Err(format!("{} vertices at {p:?}", near.len()).into()),
    }
}

/// How many of `edges` have `v` at an end.
fn touching(mesh: &ManifoldMesh, edges: &BTreeSet<EdgeId>, v: VertexId) -> usize {
    edges
        .iter()
        .filter(|e| {
            let h = e.half_edge();
            mesh.origin(h) == v || mesh.dest(h) == v
        })
        .count()
}

fn assert_one_dart_per_tip(name: &str, bytes: &[u8]) -> Result<(), Setup> {
    let mesh = mesh_of(name, bytes)?;
    let positions = obj_positions()?;
    let mut tips = tip_indices()?
        .into_iter()
        .map(|i| {
            vertex_at(
                &mesh,
                *positions.get(i).ok_or("a tip past the OBJ's vertices")?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    tips.sort();
    let found = uv_cuts(&mesh, &SeamSet::new())?;
    let darts = found.darts()?;
    assert_eq!(darts.len(), tips.len(), "{name}: one dart per tip");
    let mut dart_edges = BTreeSet::new();
    for (dart, &tip) in darts.iter().zip(&tips) {
        let kind = ProposalKind::Seam {
            reason: SeamReason::SourceDart,
        };
        assert_eq!(dart.kind(), &kind, "{name}");
        assert!(dart.add().values().all(|&k| k == SeamKind::Dart), "{name}");
        let edges: BTreeSet<EdgeId> = dart.add().keys().copied().collect();
        assert_eq!(
            edges.len(),
            2,
            "{name}: each cut that stops short is two edges"
        );
        assert_eq!(
            touching(&mesh, &edges, tip),
            1,
            "{name}: {tip:?} ends its dart"
        );
        let named = format!("vertex {} stops partway", tip.index());
        assert!(
            dart.explanation().contains(&named),
            "{name}: {}",
            dart.explanation()
        );
        dart_edges.extend(edges);
    }
    let seams: BTreeSet<EdgeId> = found
        .proposal()?
        .ok_or("the other cuts and the boundary are proposed")?
        .add()
        .keys()
        .copied()
        .collect();
    assert!(seams.is_disjoint(&dart_edges), "{name}");
    let every: BTreeSet<EdgeId> = found
        .edges()
        .iter()
        .chain(found.material_boundaries())
        .copied()
        .collect();
    assert_eq!(&seams | &dart_edges, every, "{name}");
    Ok(())
}

#[test]
fn the_gltf_gives_one_dart_per_recorded_tip() -> Result<(), Setup> {
    assert_one_dart_per_tip("dead-end.glb", DEAD_END_GLB)
}

#[test]
fn the_obj_gives_one_dart_per_recorded_tip() -> Result<(), Setup> {
    assert_one_dart_per_tip("dead-end.obj", DEAD_END_OBJ)
}
