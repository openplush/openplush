//! UV cuts on the source-seam fixtures, imported and repaired as a shell would, against the
//! reference layout matched by position: the capsule as glTF and as OBJ, and its hard-shaded twin,
//! whose vertices are also split where only the normals differ. Here rather than in
//! `openplush-seams` because reading the files needs `openplush-io`, a sibling of that crate.

use std::collections::BTreeSet;

use openplush_geom::DVec3;
use openplush_pipeline::{
    EdgeId, ImportFile, ImportFiles, ManifoldMesh, RepairPlan, SeamSet, VertexId, import, repair,
};
use openplush_seams::uv_cuts;

type Setup = Box<dyn std::error::Error>;

const CAPSULE_GLB: &[u8] = include_bytes!("../../../fixtures/meshes/source-seams/capsule-uv.glb");
const CAPSULE_OBJ: &[u8] = include_bytes!("../../../fixtures/meshes/source-seams/capsule-uv.obj");
const HARDSHADE_GLB: &[u8] =
    include_bytes!("../../../fixtures/meshes/source-seams/capsule-hardshade.glb");
/// The reference cuts, as pairs of 0-based vertex indices into `CAPSULE_OBJ`.
const LAYOUT: &str = include_str!("../../../fixtures/seams/capsule-uv.json");

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
    std::str::from_utf8(CAPSULE_OBJ)?
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

/// The layout's seams as OBJ vertex index pairs, after checking each is a cut.
fn layout_pairs() -> Result<Vec<(usize, usize)>, Setup> {
    assert!(LAYOUT.contains(r#""vertexIndexBase": 0"#));
    let pairs: Vec<(usize, usize)> = LAYOUT
        .lines()
        .filter_map(|line| line.split_once(r#""edge": ["#))
        .map(|(_, rest)| {
            let inside = rest.split(']').next().unwrap_or_default();
            let (a, b) = inside.split_once(',').ok_or("an edge has two ends")?;
            Ok::<_, Setup>((a.trim().parse()?, b.trim().parse()?))
        })
        .collect::<Result<_, _>>()?;
    assert_eq!(LAYOUT.matches(r#""kind": "cut""#).count(), pairs.len());
    assert!(!pairs.is_empty());
    Ok(pairs)
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

/// The reference layout as edges of `mesh`.
fn layout_on(mesh: &ManifoldMesh) -> Result<BTreeSet<EdgeId>, Setup> {
    let positions = obj_positions()?;
    layout_pairs()?
        .into_iter()
        .map(|(a, b)| {
            let (pa, pb) = (positions.get(a), positions.get(b));
            let (pa, pb) = pa.zip(pb).ok_or("a layout index past the OBJ's vertices")?;
            let (from, to) = (vertex_at(mesh, *pa)?, vertex_at(mesh, *pb)?);
            mesh.outgoing(from)
                .find(|&h| mesh.dest(h) == to)
                .map(|h| h.edge())
                .ok_or_else(|| format!("no edge {a} to {b}").into())
        })
        .collect()
}

fn assert_cuts_are_the_layout(name: &str, bytes: &[u8]) -> Result<(), Setup> {
    let mesh = mesh_of(name, bytes)?;
    let cuts = uv_cuts(&mesh, &SeamSet::new())?;
    assert_eq!(cuts.faces_without_uv(), 0, "{name}");
    let found: BTreeSet<EdgeId> = cuts.edges().iter().copied().collect();
    assert_eq!(found, layout_on(&mesh)?, "{name}");
    Ok(())
}

#[test]
fn the_capsule_gltf_cuts_are_exactly_the_reference_layout() -> Result<(), Setup> {
    assert_cuts_are_the_layout("capsule-uv.glb", CAPSULE_GLB)
}

#[test]
fn the_capsule_obj_cuts_are_exactly_the_reference_layout() -> Result<(), Setup> {
    assert_cuts_are_the_layout("capsule-uv.obj", CAPSULE_OBJ)
}

#[test]
fn hard_shading_splits_add_no_cut_to_the_reference_layout() -> Result<(), Setup> {
    assert_cuts_are_the_layout("capsule-hardshade.glb", HARDSHADE_GLB)
}
