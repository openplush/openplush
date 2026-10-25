//! Fixtures the stage tests share.

use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;

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

/// A spherical cap of radius 10 out to 60° from its pole, in `rings` rings of 16, with the rim
/// vertex at angle 0 first in its ring.
pub(crate) fn cap(rings: u32) -> String {
    let n = 16;
    let mut s = String::from("v 0 0 10\n");
    for k in 1..=rings {
        let polar = f64::from(k) * (PI / 3.0) / f64::from(rings);
        for j in 0..n {
            let a = f64::from(j) * TAU / f64::from(n);
            let (x, y, z) = (
                libm::sin(polar) * libm::cos(a),
                libm::sin(polar) * libm::sin(a),
                libm::cos(polar),
            );
            let _ = writeln!(s, "v {} {} {}", 10.0 * x, 10.0 * y, 10.0 * z);
        }
    }
    let at = |k: u32, j: u32| if k == 0 { 1 } else { 2 + (k - 1) * n + j % n };
    for j in 0..n {
        let _ = writeln!(s, "f {} {} {}", at(0, 0), at(1, j), at(1, j + 1));
    }
    for k in 1..rings {
        for j in 0..n {
            let (a, b, c, d) = (at(k, j), at(k + 1, j), at(k + 1, j + 1), at(k, j + 1));
            let _ = writeln!(s, "f {a} {b} {c}\nf {a} {c} {d}");
        }
    }
    s
}
