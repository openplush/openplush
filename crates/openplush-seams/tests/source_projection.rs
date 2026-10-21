//! Projection points read from the source model: an interior vertex with at least five faces,
//! all with UVs, whose corner UVs there are pairwise more than `UV_CUT` apart, as an automatic
//! projection leaves at a sphere's poles. Most meshes are bipyramids: `n` points round the
//! equator, each with one UV shared by its corners, and a pole above and below.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use std::collections::BTreeMap;
use std::f64::consts::TAU;

use common::{Setup, edge_between, repaired_with_uvs, vertex_at};
use openplush_geom::epsilon::UV_CUT;
use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{EdgeId, ManifoldMesh, VertexId};
use openplush_seams::{SeamKind, SeamSet, uv_cuts};

const TOP: DVec3 = DVec3::new(0.0, 0.0, 10.0);
const BOTTOM: DVec3 = DVec3::new(0.0, 0.0, -10.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pole {
    Top,
    Bottom,
}

fn equator(n: u32, i: u32) -> DVec3 {
    let around = TAU * f64::from(i) / f64::from(n);
    DVec3::new(10.0 * around.cos(), 10.0 * around.sin(), 0.0)
}

/// Face `2i` is the top face over equator edge `i`, face `2i + 1` the bottom one; each pole
/// is its face's third corner.
fn bipyramid_triangles(n: u32) -> Vec<[u32; 3]> {
    (0..n)
        .flat_map(|i| {
            let j = (i + 1) % n;
            [[i, j, n], [j, i, n + 1]]
        })
        .collect()
}

/// Corner UVs in triangle order then corner order: an equator vertex's shared UV, and
/// `pole(which, i)` at the pole corner of the face over equator edge `i`.
fn bipyramid_uvs(n: u32, pole: impl Fn(Pole, u32) -> DVec2) -> Vec<Option<DVec2>> {
    let shared = |v: u32| DVec2::new(f64::from(v) / f64::from(n), 0.5);
    (0..n)
        .flat_map(|i| {
            let j = (i + 1) % n;
            [
                shared(i),
                shared(j),
                pole(Pole::Top, i),
                shared(j),
                shared(i),
                pole(Pole::Bottom, i),
            ]
        })
        .map(Some)
        .collect()
}

/// A pole UV of its own for each face round it.
fn own(_: Pole, i: u32) -> DVec2 {
    DVec2::new(0.1 * f64::from(i), 0.0)
}

/// One UV for every face round a pole.
fn shared(pole: Pole, _: u32) -> DVec2 {
    match pole {
        Pole::Top => DVec2::new(0.5, 1.0),
        Pole::Bottom => DVec2::new(0.5, 0.0),
    }
}

/// Own UVs round the top, one shared UV round the bottom.
fn own_top(pole: Pole, i: u32) -> DVec2 {
    match pole {
        Pole::Top => own(pole, i),
        Pole::Bottom => shared(pole, i),
    }
}

fn bipyramid(n: u32, uvs: Vec<Option<DVec2>>) -> Result<ManifoldMesh, Setup> {
    let positions = (0..n).map(|i| equator(n, i)).chain([TOP, BOTTOM]).collect();
    Ok(repaired_with_uvs(positions, bipyramid_triangles(n), Some(uvs))?.0)
}

/// The edges from `pole` to each equator point, ascending.
fn spokes(mesh: &ManifoldMesh, n: u32, pole: DVec3) -> Result<Vec<EdgeId>, Setup> {
    let mut edges = (0..n)
        .map(|i| edge_between(mesh, pole, equator(n, i)))
        .collect::<Result<Vec<_>, _>>()?;
    edges.sort();
    Ok(edges)
}

fn points(mesh: &ManifoldMesh) -> Result<Vec<VertexId>, Setup> {
    Ok(uv_cuts(mesh, &SeamSet::new())?.projection_points().to_vec())
}

#[test]
fn twelve_faces_whose_pole_corners_all_differ_make_a_projection_point() -> Result<(), Setup> {
    let mesh = bipyramid(12, bipyramid_uvs(12, own_top))?;
    assert_eq!(points(&mesh)?, [vertex_at(&mesh, TOP)?]);
    Ok(())
}

#[test]
fn four_faces_are_too_few() -> Result<(), Setup> {
    let mesh = bipyramid(4, bipyramid_uvs(4, own))?;
    let cuts = uv_cuts(&mesh, &SeamSet::new())?;
    let mut every_spoke = [spokes(&mesh, 4, TOP)?, spokes(&mesh, 4, BOTTOM)?].concat();
    every_spoke.sort();
    assert_eq!(cuts.edges(), every_spoke, "every pole edge is still a cut");
    assert_eq!(cuts.projection_points(), &[] as &[VertexId]);
    Ok(())
}

#[test]
fn five_faces_are_enough_unless_two_corners_agree() -> Result<(), Setup> {
    let mesh = bipyramid(5, bipyramid_uvs(5, own_top))?;
    assert_eq!(points(&mesh)?, [vertex_at(&mesh, TOP)?]);
    // Adjacent faces and faces apart, so agreeing neighbours are not the only test.
    for (a, b) in [(0, 1), (0, 2), (1, 4)] {
        let agreeing = |pole: Pole, i: u32| own_top(pole, if i == b { a } else { i });
        let mesh = bipyramid(5, bipyramid_uvs(5, agreeing))?;
        assert_eq!(points(&mesh)?, [] as [VertexId; 0], "{a} and {b} agree");
    }
    Ok(())
}

#[test]
fn pole_corners_up_to_uv_cut_apart_agree_and_just_beyond_it_differ() -> Result<(), Setup> {
    // Faces 0 and 1 at UV (0, 0) and that plus `gap`, so 0 + UV_CUT - 0 is exactly UV_CUT.
    let with_gap = |gap: DVec2| {
        move |pole: Pole, i: u32| match (pole, i) {
            (Pole::Top, 1) => own(pole, 0) + gap,
            _ => own_top(pole, i),
        }
    };
    for d in [DVec2::X, DVec2::Y, -DVec2::X, -DVec2::Y] {
        let at = bipyramid(5, bipyramid_uvs(5, with_gap(d * UV_CUT)))?;
        assert_eq!(points(&at)?, [] as [VertexId; 0], "{d:?}");
        let beyond = bipyramid(5, bipyramid_uvs(5, with_gap(d * UV_CUT * 1.01)))?;
        assert_eq!(points(&beyond)?, [vertex_at(&beyond, TOP)?], "{d:?}");
    }
    Ok(())
}

#[test]
fn a_face_without_uvs_round_the_vertex_rules_it_out() -> Result<(), Setup> {
    // Face 6 is the top face over equator edge 3: bare entirely, then bare only at an equator
    // corner, which still leaves it a face without UVs.
    for bare in [18..21, 18..19] {
        let mut uvs = bipyramid_uvs(12, own_top);
        uvs[bare.clone()].fill(None);
        let mesh = bipyramid(12, uvs)?;
        assert_eq!(points(&mesh)?, [] as [VertexId; 0], "{bare:?}");
    }
    Ok(())
}

#[test]
fn a_pole_whose_corners_share_one_uv_is_none() -> Result<(), Setup> {
    let mesh = bipyramid(12, bipyramid_uvs(12, shared))?;
    let cuts = uv_cuts(&mesh, &SeamSet::new())?;
    assert_eq!(cuts.edges(), &[] as &[EdgeId]);
    assert_eq!(cuts.projection_points(), &[] as &[VertexId]);
    Ok(())
}

#[test]
fn a_vertex_on_the_surface_boundary_is_none() -> Result<(), Setup> {
    // The top half of a 12-point bipyramid with one face left out: the pole keeps 11 faces,
    // each with its own UV there, but lies on the opening.
    let n = 12;
    let positions = (0..n).map(|i| equator(n, i)).chain([TOP]).collect();
    let triangles: Vec<[u32; 3]> = (0..n - 1).map(|i| [i, i + 1, n]).collect();
    let uvs = (0..n - 1)
        .flat_map(|i| {
            [
                DVec2::new(0.0, 0.0),
                DVec2::new(1.0, 0.0),
                own(Pole::Top, i),
            ]
        })
        .map(Some)
        .collect();
    let (mesh, _) = repaired_with_uvs(positions, triangles, Some(uvs))?;
    assert!(mesh.is_boundary_vertex(vertex_at(&mesh, TOP)?));
    assert_eq!(points(&mesh)?, [] as [VertexId; 0]);
    Ok(())
}

/// A disc of five sectors round the origin, each its own UV island laid out flat, so five
/// hand-marked seams run from the centre to the rim: an inner ring of five points, an outer
/// ring of five, and three faces per sector.
fn five_sectors() -> Result<(ManifoldMesh, Vec<DVec3>), Setup> {
    let ring = |radius: f64, k: u32| {
        let around = TAU * f64::from(k % 5) / 5.0;
        DVec3::new(radius * around.cos(), radius * around.sin(), 0.0)
    };
    let mut positions = vec![DVec3::ZERO];
    positions.extend((0..5).map(|k| ring(10.0, k)));
    positions.extend((0..5).map(|k| ring(20.0, k)));
    let (inner, outer) = (|k: u32| 1 + k % 5, |k: u32| 6 + k % 5);
    let mut triangles = Vec::new();
    let mut uvs = Vec::new();
    for k in 0..5 {
        let sector = [
            [0, inner(k), inner(k + 1)],
            [inner(k), outer(k), outer(k + 1)],
            [inner(k), outer(k + 1), inner(k + 1)],
        ];
        let island = DVec2::new(10.0 * f64::from(k), 0.0);
        for corner in sector.iter().flatten() {
            let p = positions[*corner as usize];
            uvs.push(Some(DVec2::new(p.x, p.y) / 100.0 + island));
        }
        triangles.extend(sector);
    }
    let (mesh, _) = repaired_with_uvs(positions.clone(), triangles, Some(uvs))?;
    Ok((mesh, positions))
}

#[test]
fn five_hand_marked_seams_meeting_make_a_point_too() -> Result<(), Setup> {
    // A known false positive: the corners round the centre are pairwise apart exactly as at a
    // projected pole. The hint's "may" wording allows for it.
    let (mesh, positions) = five_sectors()?;
    let cuts = uv_cuts(&mesh, &SeamSet::new())?;
    let mut seams = Vec::new();
    for k in 1..=5 {
        seams.push(edge_between(&mesh, positions[0], positions[k])?);
        seams.push(edge_between(&mesh, positions[k], positions[k + 5])?);
    }
    seams.sort();
    assert_eq!(cuts.edges(), seams, "the five seams are the only cuts");
    assert_eq!(cuts.projection_points(), [vertex_at(&mesh, DVec3::ZERO)?]);
    Ok(())
}

#[test]
fn projection_points_come_out_ascending() -> Result<(), Setup> {
    let mesh = bipyramid(12, bipyramid_uvs(12, own))?;
    let mut both = [vertex_at(&mesh, TOP)?, vertex_at(&mesh, BOTTOM)?];
    both.sort();
    assert_eq!(points(&mesh)?, both);
    Ok(())
}

#[test]
fn the_hint_changes_no_cut() -> Result<(), Setup> {
    // Both poles are projection points, and every pole edge is still a cut, proposed as one.
    let mesh = bipyramid(12, bipyramid_uvs(12, own))?;
    let cuts = uv_cuts(&mesh, &SeamSet::new())?;
    assert_eq!(cuts.projection_points().len(), 2);
    let mut every_spoke = [spokes(&mesh, 12, TOP)?, spokes(&mesh, 12, BOTTOM)?].concat();
    every_spoke.sort();
    assert_eq!(cuts.edges(), every_spoke);
    let proposal = cuts.proposal()?.ok_or("the cuts are proposed")?;
    let add: BTreeMap<EdgeId, SeamKind> = every_spoke.iter().map(|&e| (e, SeamKind::Cut)).collect();
    assert_eq!(proposal.add(), &add);
    Ok(())
}
