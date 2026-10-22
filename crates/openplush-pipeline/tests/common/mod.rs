//! Fixtures the stage tests share.

use openplush_pipeline::{
    Fabric, FabricCategory, FabricId, FabricSpec, ManifoldMesh, Mm, SeamKind, SeamSet, VertexId,
};

pub(crate) type Setup = Box<dyn std::error::Error>;

pub(crate) const CAPSULE_OBJ: &str = include_str!("../../../../fixtures/meshes/capsule.obj");
pub(crate) const CAPSULE_SEAMS: &str = include_str!("../../../../fixtures/seams/capsule.json");
pub(crate) const CUBE_OBJ: &str = include_str!("../../../../fixtures/meshes/cube.obj");
pub(crate) const CUBE_SEAMS: &str = include_str!("../../../../fixtures/seams/cube.json");

/// A woven fabric with `allowance_mm` as its default seam allowance.
pub(crate) fn fabric(allowance_mm: f64) -> Result<Fabric, Setup> {
    Ok(Fabric::new(FabricSpec {
        id: FabricId::new(0),
        name: "cotton".into(),
        category: FabricCategory::Woven,
        stretch_grain_pct: 0.0,
        stretch_cross_pct: 0.0,
        stretch_bias_pct: 0.0,
        nap: false,
        pile_len: Mm::try_new(0.0)?,
        thickness: Mm::try_new(0.3)?,
        frays: true,
        default_allowance: Mm::try_new(allowance_mm)?,
        roll_width: Mm::try_new(1500.0)?,
    })?)
}

/// The capsule's reference seams on `mesh`, the capsule repaired.
pub(crate) fn capsule_seams(mesh: &ManifoldMesh) -> Result<SeamSet, Setup> {
    reference_seams(CAPSULE_OBJ, CAPSULE_SEAMS, mesh)
}

/// The seam set `seams` of the OBJ `obj` on `mesh`, that OBJ repaired. Seam edges name OBJ
/// vertices by file order, matched to the mesh by position.
pub(crate) fn reference_seams(
    obj: &str,
    seams: &str,
    mesh: &ManifoldMesh,
) -> Result<SeamSet, Setup> {
    let positions: Vec<[f64; 3]> = obj
        .lines()
        .filter_map(|line| line.strip_prefix("v "))
        .map(|rest| {
            let c: Vec<f64> = rest
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<_, _>>()?;
            Ok([c[0], c[1], c[2]])
        })
        .collect::<Result<_, std::num::ParseFloatError>>()?;
    let vertex = |i: usize| -> Result<VertexId, Setup> {
        (0..u32::try_from(mesh.vertex_count())?)
            .map(VertexId::new)
            .find(|&v| {
                let p = mesh.position(v);
                [p.x, p.y, p.z] == positions[i]
            })
            .ok_or_else(|| format!("no vertex for OBJ index {i}").into())
    };
    let mut set = SeamSet::new();
    for pair in seams.split("\"edge\": [").skip(1) {
        let (a, b) = pair
            .split(']')
            .next()
            .and_then(|s| s.split_once(','))
            .ok_or("malformed edge")?;
        let (a, b) = (vertex(a.trim().parse()?)?, vertex(b.trim().parse()?)?);
        let edge = mesh
            .outgoing(a)
            .find(|&h| mesh.dest(h) == b)
            .ok_or("no such edge")?
            .edge();
        set.insert(edge, SeamKind::Cut)?;
    }
    Ok(set)
}
