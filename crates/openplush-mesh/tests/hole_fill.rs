//! Repair step 7's fill: a listed loop is triangulated, refined to its rim's density, faired,
//! and appended; a loop it cannot fill is refused, named.

use std::collections::BTreeSet;

use openplush_geom::DVec3;
use openplush_geom::epsilon::ModelTolerances;
use openplush_mesh::{
    FaceId, LoopRef, ManifoldMesh, MeshError, RepairPlan, RepairReport, SourceCorner, TriangleSoup,
    VertexId, into_manifold,
};

type Setup = Box<dyn std::error::Error>;

fn corner(vertex: u32) -> SourceCorner {
    SourceCorner {
        primitive: 0,
        vertex,
    }
}

/// Every corner's source vertex is its own position index.
fn soup(positions: Vec<DVec3>, triangles: &[[u32; 3]]) -> Result<TriangleSoup, Setup> {
    let corners = triangles.iter().flatten().map(|&v| corner(v)).collect();
    Ok(TriangleSoup::new(
        positions,
        triangles.to_vec(),
        corners,
        None,
    )?)
}

fn filling(loops: &[LoopRef]) -> RepairPlan {
    RepairPlan {
        fill_holes: loops.iter().copied().collect::<BTreeSet<_>>(),
        ..RepairPlan::default()
    }
}

fn tolerances(m: &ManifoldMesh) -> Result<ModelTolerances, Setup> {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for v in 0..u32::try_from(m.vertex_count())? {
        let p = m.position(VertexId::new(v));
        for (i, c) in [p.x, p.y, p.z].into_iter().enumerate() {
            lo[i] = lo[i].min(c);
            hi[i] = hi[i].max(c);
        }
    }
    let span = DVec3::new(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]);
    Ok(ModelTolerances::for_diagonal(span.length())?)
}

/// A unit cube missing its top face (two triangles), so the top rim, corners 4 to 7, is a hole.
fn open_cube() -> Result<TriangleSoup, Setup> {
    let p = |x: f64, y: f64, z: f64| DVec3::new(x, y, z);
    let positions = vec![
        p(0., 0., 0.),
        p(1., 0., 0.),
        p(1., 1., 0.),
        p(0., 1., 0.),
        p(0., 0., 1.),
        p(1., 0., 1.),
        p(1., 1., 1.),
        p(0., 1., 1.),
    ];
    let faces = [
        [0, 2, 1],
        [0, 3, 2],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    soup(positions, &faces)
}

fn filled_loop(report: &RepairReport, loop_ref: LoopRef) -> Result<Vec<FaceId>, Setup> {
    let hole = report
        .holes()
        .iter()
        .find(|h| h.loop_ref() == loop_ref)
        .ok_or("the loop is not reported")?;
    assert!(hole.filled(), "{loop_ref:?} not filled");
    Ok(hole.added_faces().to_vec())
}

#[test]
fn a_listed_hole_is_closed_by_faces_appended_after_the_mesh_s_own() -> Result<(), Setup> {
    let rim = LoopRef::new(corner(4));
    let (open, _) = into_manifold(&open_cube()?, &RepairPlan::default())?;
    let (m, report) = into_manifold(&open_cube()?, &filling(&[rim]))?;
    m.debug_validate(&tolerances(&m)?)?;
    assert!(m.boundary_loops().is_empty());
    let added = filled_loop(&report, rim)?;
    assert!(added.len() >= 2, "{added:?}");
    let first = u32::try_from(open.face_count())?;
    assert_eq!(added[0], FaceId::new(first));
    assert_eq!(m.face_count(), open.face_count() + added.len());
    for &f in &added {
        let h = m.face_half_edge(f);
        assert_eq!(m.corner_source(h), None);
        assert_eq!(m.corner_uv(h), None);
    }
    // The mesh's own faces keep their IDs and provenance.
    for f in 0..first {
        let h = m.face_half_edge(FaceId::new(f));
        assert!(m.corner_source(h).is_some());
    }
    Ok(())
}

#[test]
fn a_flat_square_hole_is_filled_flat() -> Result<(), Setup> {
    let rim = LoopRef::new(corner(4));
    let (m, report) = into_manifold(&open_cube()?, &filling(&[rim]))?;
    for f in filled_loop(&report, rim)? {
        let h = m.face_half_edge(f);
        for k in [h, m.next(h), m.prev(h)] {
            assert!((m.position(m.origin(k)).z - 1.0).abs() < 1e-12);
        }
    }
    Ok(())
}

/// A UV sphere of radius 1 with `rings` latitude bands of `segments` quads, minus the faces
/// of its top `cut` bands, so its top is one hole.
fn capped_sphere(rings: u32, segments: u32, cut: u32) -> Result<TriangleSoup, Setup> {
    let mut positions = vec![DVec3::new(0., 0., -1.)];
    for r in 1..rings {
        let theta = std::f64::consts::PI * f64::from(r) / f64::from(rings);
        for s in 0..segments {
            let phi = std::f64::consts::TAU * f64::from(s) / f64::from(segments);
            positions.push(DVec3::new(
                theta.sin() * phi.cos(),
                theta.sin() * phi.sin(),
                -theta.cos(),
            ));
        }
    }
    let ring = |r: u32, s: u32| 1 + (r - 1) * segments + s % segments;
    let mut faces = Vec::new();
    for s in 0..segments {
        faces.push([0, ring(1, s + 1), ring(1, s)]);
    }
    for r in 1..rings - 1 - cut {
        for s in 0..segments {
            let (a, b, c, d) = (
                ring(r, s),
                ring(r, s + 1),
                ring(r + 1, s + 1),
                ring(r + 1, s),
            );
            faces.push([a, b, c]);
            faces.push([a, c, d]);
        }
    }
    soup(positions, &faces)
}

#[test]
fn a_faired_patch_follows_the_curved_surface_around_it() -> Result<(), Setup> {
    // 16 bands of 24 with the top 4 cut: the rim is a circle of 24 vertices below the top of
    // the sphere, and a flat fill would sit in its plane. Fairing with the surface's slope
    // fixed bulges the patch out towards the missing cap.
    let soup = capped_sphere(16, 24, 4)?;
    let (open, open_report) = into_manifold(&soup, &RepairPlan::default())?;
    let rim = open_report.holes()[0].loop_ref();
    let rim_z = m_rim_z(&open)?;
    let cap = 1.0 - rim_z;
    let (m, report) = into_manifold(&soup, &filling(&[rim]))?;
    m.debug_validate(&tolerances(&m)?)?;
    assert!(m.boundary_loops().is_empty());
    filled_loop(&report, rim)?;
    let added: Vec<DVec3> = (open.vertex_count()..m.vertex_count())
        .map(|v| Ok(m.position(VertexId::new(u32::try_from(v)?))))
        .collect::<Result<_, Setup>>()?;
    assert!(!added.is_empty(), "the patch was not refined");
    let mean_height = added.iter().map(|p| p.z - rim_z).sum::<f64>() / added.len() as f64;
    assert!(
        mean_height > 0.3 * cap,
        "mean height {mean_height} of a {cap} cap"
    );
    for p in &added {
        assert!(p.length() < 1.1, "{p:?} bulges past the sphere");
    }
    Ok(())
}

/// The height of the open mesh's one rim.
fn m_rim_z(m: &ManifoldMesh) -> Result<f64, Setup> {
    let loops = m.boundary_loops();
    let h = loops.first().and_then(|l| l.first()).ok_or("no rim")?;
    Ok(m.position(m.origin(*h)).z)
}

#[test]
fn only_the_listed_loop_is_filled() -> Result<(), Setup> {
    // An open tube: two rims.
    let mut positions = Vec::new();
    for z in [0.0, 1.0] {
        for c in 0..6 {
            let a = std::f64::consts::TAU * f64::from(c) / 6.0;
            positions.push(DVec3::new(a.cos(), a.sin(), z));
        }
    }
    let mut faces = Vec::new();
    for c in 0..6u32 {
        let (a, b) = (c, (c + 1) % 6);
        faces.push([a, b, b + 6]);
        faces.push([a, b + 6, a + 6]);
    }
    let tube = soup(positions, &faces)?;
    let bottom = LoopRef::new(corner(0));
    let (m, report) = into_manifold(&tube, &filling(&[bottom]))?;
    assert_eq!(m.boundary_loops().len(), 1);
    filled_loop(&report, bottom)?;
    let open: Vec<bool> = report.holes().iter().map(|h| h.filled()).collect();
    assert_eq!(open.iter().filter(|f| !**f).count(), 1);
    Ok(())
}

/// A flat disc of `n` triangles fanned from its centre, so its rim is one loop of `n`.
fn fan(n: u32) -> Result<TriangleSoup, Setup> {
    let mut positions = vec![DVec3::ZERO];
    for k in 0..n {
        let a = std::f64::consts::TAU * f64::from(k) / f64::from(n);
        positions.push(DVec3::new(a.cos(), a.sin(), 0.0));
    }
    let faces: Vec<[u32; 3]> = (0..n).map(|k| [0, 1 + k, 1 + (k + 1) % n]).collect();
    soup(positions, &faces)
}

#[test]
fn a_loop_of_exactly_512_vertices_is_filled() -> Result<(), Setup> {
    let fan = fan(512)?;
    let (_, report) = into_manifold(&fan, &RepairPlan::default())?;
    let rim = report.holes()[0].loop_ref();
    let (m, report) = into_manifold(&fan, &filling(&[rim]))?;
    filled_loop(&report, rim)?;
    assert!(m.boundary_loops().is_empty());
    Ok(())
}

#[test]
fn a_loop_of_more_than_512_vertices_is_too_large() -> Result<(), Setup> {
    let fan = fan(513)?;
    let (_, report) = into_manifold(&fan, &RepairPlan::default())?;
    let rim = report.holes()[0].loop_ref();
    assert_eq!(
        into_manifold(&fan, &filling(&[rim])).err(),
        Some(MeshError::HoleTooLarge {
            loop_ref: rim,
            vertices: 513
        })
    );
    Ok(())
}

#[test]
fn a_loop_along_a_split_edge_is_a_seam_not_a_hole() -> Result<(), Setup> {
    // Three faces on edge 0-1: repair splits it, and the copies bound the loops.
    let p = |x: f64, y: f64, z: f64| DVec3::new(x, y, z);
    let book = soup(
        vec![
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(0., 1., 0.),
            p(0., -1., 0.),
            p(0., 0., 1.),
        ],
        &[[0, 1, 2], [1, 0, 3], [0, 1, 4]],
    )?;
    let (_, report) = into_manifold(&book, &RepairPlan::default())?;
    let rim = report.holes()[0].loop_ref();
    assert_eq!(
        into_manifold(&book, &filling(&[rim])).err(),
        Some(MeshError::HoleOnSplit { loop_ref: rim })
    );
    Ok(())
}

#[test]
fn a_loop_whose_only_triangle_has_no_area_is_not_fillable() -> Result<(), Setup> {
    // A flat sheet with a hairline triangular hole a, b, c: its one triangle is a sliver.
    let p = |x: f64, y: f64| DVec3::new(x, y, 0.0);
    let positions = vec![
        p(-1., -1.),
        p(3., -1.),
        p(3., 1.),
        p(-1., 1.),
        p(0., 0.),
        p(1., 0.),
        p(2., 1e-14),
    ];
    let (o0, o1, o2, o3, a, b, c) = (0, 1, 2, 3, 4, 5, 6);
    let faces = [
        [o0, o1, b],
        [o0, b, a],
        [o1, c, b],
        [o1, o2, c],
        [o2, a, c],
        [o2, o3, a],
        [o3, o0, a],
    ];
    let sheet = soup(positions, &faces)?;
    let rim = LoopRef::new(corner(a));
    assert_eq!(
        into_manifold(&sheet, &filling(&[rim])).err(),
        Some(MeshError::HoleNotFillable { loop_ref: rim })
    );
    Ok(())
}

#[test]
fn the_fill_refusals_name_the_loop() {
    let loop_ref = LoopRef::new(corner(3));
    assert_eq!(
        MeshError::HoleNotFillable { loop_ref }.to_string(),
        "the loop LoopRef(SourceCorner { primitive: 0, vertex: 3 }) has no fill: every \
         triangulation of it reuses an edge, has no area, or does not solve"
    );
    assert_eq!(
        MeshError::HoleOnSplit { loop_ref }.to_string(),
        "the loop LoopRef(SourceCorner { primitive: 0, vertex: 3 }) runs along a split edge, \
         so it is one side of a seam, not a hole"
    );
    assert_eq!(
        MeshError::HoleTooLarge {
            loop_ref,
            vertices: 600
        }
        .to_string(),
        "the loop LoopRef(SourceCorner { primitive: 0, vertex: 3 }) has 600 vertices, more than \
         the 512 a fill takes"
    );
}
