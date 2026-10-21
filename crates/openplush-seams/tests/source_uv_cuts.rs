//! UV cuts read from the source model: an interior edge whose corner UVs differ across it, at
//! either endpoint, by more than `UV_CUT`. Each mesh is a unit square of two triangles whose only
//! interior edge is the diagonal from corner 0 to corner 2.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use std::collections::BTreeMap;

use common::{Setup, repaired_with_uvs};
use openplush_geom::epsilon::UV_CUT;
use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{EdgeId, ManifoldMesh, VertexId};
use openplush_seams::{ProposalKind, SeamKind, SeamReason, SeamSet, SeamsError, uv_cuts};

const CORNERS: [DVec3; 4] = [
    DVec3::new(0.0, 0.0, 0.0),
    DVec3::new(10.0, 0.0, 0.0),
    DVec3::new(10.0, 10.0, 0.0),
    DVec3::new(0.0, 10.0, 0.0),
];

/// Corner UVs of triangles (0, 1, 2) then (0, 2, 3) that agree across the diagonal.
const AGREEING: [DVec2; 6] = [
    DVec2::new(0.0, 0.0),
    DVec2::new(1.0, 0.0),
    DVec2::new(1.0, 1.0),
    DVec2::new(0.0, 0.0),
    DVec2::new(1.0, 1.0),
    DVec2::new(0.0, 1.0),
];

/// The second triangle's corners at square corners 0 and 2, the diagonal's two ends.
const SECOND_AT_0: usize = 3;
const SECOND_AT_2: usize = 4;

/// The square with corner UVs `uvs`, in triangle order then corner order.
fn square(uvs: Option<Vec<Option<DVec2>>>) -> Result<ManifoldMesh, Setup> {
    Ok(repaired_with_uvs(CORNERS.to_vec(), vec![[0, 1, 2], [0, 2, 3]], uvs)?.0)
}

/// The square with every corner's UV from `uvs`.
fn square_uv(uvs: [DVec2; 6]) -> Result<ManifoldMesh, Setup> {
    square(Some(uvs.into_iter().map(Some).collect()))
}

/// `AGREEING` with corner `corner` moved by `by`.
fn moved(corner: usize, by: DVec2) -> [DVec2; 6] {
    moved_from(AGREEING, corner, by)
}

fn moved_from(mut uvs: [DVec2; 6], corner: usize, by: DVec2) -> [DVec2; 6] {
    uvs[corner] += by;
    uvs
}

fn vertex_at(mesh: &ManifoldMesh, p: DVec3) -> Result<VertexId, Setup> {
    (0..u32::try_from(mesh.vertex_count())?)
        .map(VertexId::new)
        .find(|&v| mesh.position(v) == p)
        .ok_or_else(|| format!("no vertex at {p:?}").into())
}

/// The edge between the points `p` and `q`, found by position.
fn edge_between(mesh: &ManifoldMesh, p: DVec3, q: DVec3) -> Result<EdgeId, Setup> {
    let (from, to) = (vertex_at(mesh, p)?, vertex_at(mesh, q)?);
    mesh.outgoing(from)
        .find(|&h| mesh.dest(h) == to)
        .map(|h| h.edge())
        .ok_or_else(|| format!("no edge {p:?} to {q:?}").into())
}

fn diagonal(mesh: &ManifoldMesh) -> Result<EdgeId, Setup> {
    edge_between(mesh, CORNERS[0], CORNERS[2])
}

#[test]
fn uvs_that_agree_across_an_edge_make_no_cut() -> Result<(), Setup> {
    let mesh = square_uv(AGREEING)?;
    let cuts = uv_cuts(&mesh, &SeamSet::new())?;
    assert_eq!(cuts.edges(), &[] as &[EdgeId]);
    assert_eq!(cuts.faces_without_uv(), 0);
    assert_eq!(cuts.proposal()?, None);
    Ok(())
}

#[test]
fn a_difference_at_either_endpoint_alone_makes_a_cut() -> Result<(), Setup> {
    for corner in [SECOND_AT_0, SECOND_AT_2] {
        let mesh = square_uv(moved(corner, DVec2::new(0.25, 0.0)))?;
        let cuts = uv_cuts(&mesh, &SeamSet::new())?;
        assert_eq!(cuts.edges(), &[diagonal(&mesh)?], "corner {corner}");
    }
    Ok(())
}

#[test]
fn a_difference_up_to_uv_cut_is_no_cut_and_just_beyond_it_is_one() -> Result<(), Setup> {
    // Both ends of the diagonal at UV (0, 0), so 0 + UV_CUT - 0 is exactly UV_CUT.
    let mut origin = AGREEING;
    for corner in [0, 2, SECOND_AT_0, SECOND_AT_2] {
        origin[corner] = DVec2::ZERO;
    }
    let above = UV_CUT * 1.01;
    for corner in [SECOND_AT_0, SECOND_AT_2] {
        for d in [DVec2::X, DVec2::Y, -DVec2::X, -DVec2::Y] {
            let at = square_uv(moved_from(origin, corner, d * UV_CUT))?;
            assert_eq!(uv_cuts(&at, &SeamSet::new())?.edges(), &[] as &[EdgeId]);
            let beyond = square_uv(moved_from(origin, corner, d * above))?;
            let cuts = uv_cuts(&beyond, &SeamSet::new())?;
            assert_eq!(cuts.edges(), &[diagonal(&beyond)?], "{corner} {d:?}");
        }
    }
    Ok(())
}

#[test]
fn each_coordinate_is_compared_on_its_own() -> Result<(), Setup> {
    // 0.8 UV_CUT in both coordinates is 1.13 UV_CUT apart, but neither coordinate is beyond it.
    let mesh = square_uv(moved(SECOND_AT_0, DVec2::new(0.8, 0.8) * UV_CUT))?;
    assert_eq!(uv_cuts(&mesh, &SeamSet::new())?.edges(), &[] as &[EdgeId]);
    Ok(())
}

#[test]
fn a_side_without_uvs_is_never_a_cut_and_its_face_is_counted() -> Result<(), Setup> {
    let cut = moved(SECOND_AT_0, DVec2::new(0.25, 0.0));
    let mut second_bare: Vec<_> = cut.into_iter().map(Some).collect();
    second_bare[3..].fill(None);
    let mut third_corner_bare: Vec<_> = cut.into_iter().map(Some).collect();
    third_corner_bare[5] = None;
    for uvs in [second_bare, third_corner_bare] {
        let cuts = uv_cuts(&square(Some(uvs.clone()))?, &SeamSet::new())?;
        assert_eq!(cuts.edges(), &[] as &[EdgeId], "{uvs:?}");
        assert_eq!(cuts.faces_without_uv(), 1, "{uvs:?}");
    }
    let cuts = uv_cuts(&square(None)?, &SeamSet::new())?;
    assert_eq!(cuts.edges(), &[] as &[EdgeId]);
    assert_eq!(cuts.faces_without_uv(), 2);
    assert_eq!(cuts.proposal()?, None);
    Ok(())
}

#[test]
fn a_boundary_edge_is_never_a_cut() -> Result<(), Setup> {
    let apart = [
        DVec2::new(0.0, 0.0),
        DVec2::new(1.0, 0.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(5.0, 5.0),
        DVec2::new(6.0, 6.0),
        DVec2::new(5.0, 6.0),
    ];
    let mesh = square_uv(apart)?;
    assert_eq!(mesh.edge_count(), 5);
    assert_eq!(
        uv_cuts(&mesh, &SeamSet::new())?.edges(),
        &[diagonal(&mesh)?]
    );
    Ok(())
}

#[test]
fn vertices_split_only_for_shading_make_no_cut() -> Result<(), Setup> {
    // A hard crease, exported with each face's own copy of the shared corners, as a glTF writer
    // splits them for flat normals. The UVs agree, so repair's weld leaves no cut.
    let folded = DVec3::new(0.0, 0.0, 10.0);
    let positions = vec![
        CORNERS[0], CORNERS[1], CORNERS[2], CORNERS[0], CORNERS[2], folded,
    ];
    let uvs = AGREEING.into_iter().map(Some).collect();
    let (mesh, _) = repaired_with_uvs(positions, vec![[0, 1, 2], [3, 4, 5]], Some(uvs))?;
    assert_eq!(mesh.vertex_count(), 4, "the split corners weld");
    assert_eq!(uv_cuts(&mesh, &SeamSet::new())?.edges(), &[] as &[EdgeId]);
    Ok(())
}

#[test]
fn a_non_finite_uv_reads_as_a_cut() -> Result<(), Setup> {
    let mut uvs = AGREEING;
    uvs[SECOND_AT_0] = DVec2::new(f64::NAN, 0.0);
    let mesh = square_uv(uvs)?;
    assert_eq!(
        uv_cuts(&mesh, &SeamSet::new())?.edges(),
        &[diagonal(&mesh)?]
    );
    Ok(())
}

#[test]
fn an_edge_already_a_seam_is_left_out() -> Result<(), Setup> {
    let mesh = square_uv(moved(SECOND_AT_0, DVec2::new(0.25, 0.0)))?;
    let cut = diagonal(&mesh)?;
    let mut elsewhere = SeamSet::new();
    elsewhere.insert(
        edge_between(&mesh, CORNERS[0], CORNERS[1])?,
        SeamKind::Opening,
    )?;
    assert_eq!(uv_cuts(&mesh, &elsewhere)?.edges(), &[cut]);
    for kind in [SeamKind::Cut, SeamKind::Dart, SeamKind::FabricBoundary] {
        let mut seams = SeamSet::new();
        seams.insert(cut, kind)?;
        let cuts = uv_cuts(&mesh, &seams)?;
        assert_eq!(cuts.edges(), &[] as &[EdgeId], "{kind:?}");
        assert_eq!(cuts.proposal()?, None, "{kind:?}");
    }
    Ok(())
}

#[test]
fn a_seam_on_an_edge_the_mesh_lacks_is_refused() -> Result<(), Setup> {
    let mesh = square_uv(AGREEING)?;
    let mut seams = SeamSet::new();
    seams.insert(EdgeId::new(99), SeamKind::Cut)?;
    seams.insert(EdgeId::new(7), SeamKind::Cut)?;
    assert_eq!(
        uv_cuts(&mesh, &seams),
        Err(SeamsError::UnknownEdge {
            edge: EdgeId::new(7)
        })
    );
    Ok(())
}

#[test]
fn the_cuts_are_one_proposal_of_ordinary_seams() -> Result<(), Setup> {
    let mesh = square_uv(moved(SECOND_AT_2, DVec2::new(0.0, 0.5)))?;
    let proposal = uv_cuts(&mesh, &SeamSet::new())?
        .proposal()?
        .ok_or("the cut is proposed")?;
    assert_eq!(
        proposal.kind(),
        &ProposalKind::Seam {
            reason: SeamReason::SourceModel
        }
    );
    assert_eq!(
        proposal.add(),
        &BTreeMap::from([(diagonal(&mesh)?, SeamKind::Cut)])
    );
    assert_eq!(
        proposal.explanation(),
        "Seams along the 1 edge where the model's UV map is cut, as marked in the modelling tool."
    );
    Ok(())
}

#[test]
fn two_cuts_are_one_proposal_explained_in_the_plural() -> Result<(), Setup> {
    // A fan of three triangles round corner 0 of a pentagon, each with its own UVs, so both
    // interior edges are cuts.
    let positions = vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(10.0, 0.0, 0.0),
        DVec3::new(10.0, 10.0, 0.0),
        DVec3::new(0.0, 15.0, 0.0),
        DVec3::new(-10.0, 10.0, 0.0),
    ];
    let uvs = (0..9u8)
        .map(|i| Some(DVec2::new(f64::from(i), 0.0)))
        .collect();
    let (mesh, _) = repaired_with_uvs(
        positions.clone(),
        vec![[0, 1, 2], [0, 2, 3], [0, 3, 4]],
        Some(uvs),
    )?;
    let cuts = uv_cuts(&mesh, &SeamSet::new())?;
    let mut both = [
        edge_between(&mesh, positions[0], positions[2])?,
        edge_between(&mesh, positions[0], positions[3])?,
    ];
    both.sort();
    assert_eq!(cuts.edges(), &both);
    let proposal = cuts.proposal()?.ok_or("the cuts are proposed")?;
    assert!(proposal.add().keys().eq(&both));
    assert_eq!(
        proposal.explanation(),
        "Seams along the 2 edges where the model's UV map is cut, as marked in the modelling tool."
    );
    Ok(())
}
