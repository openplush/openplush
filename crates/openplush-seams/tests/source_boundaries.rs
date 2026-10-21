//! Material boundaries read from the source model: an interior edge whose two faces both come
//! from the file and differ in material, a face with no material counting as one more. They are
//! proposed beside the UV cuts as `FabricBoundary` seams, which win on an edge that is both.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{Setup, edge_between, soup_with_materials};
use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{
    EdgeId, ManifoldMesh, MaterialIndex, RepairPlan, SourceCorner, SourceMaterials, TriangleSoup,
    into_manifold,
};
use openplush_seams::{ProposalKind, SeamKind, SeamReason, SeamSet, uv_cuts};

const CORNERS: [DVec3; 4] = [
    DVec3::new(0.0, 0.0, 0.0),
    DVec3::new(10.0, 0.0, 0.0),
    DVec3::new(10.0, 10.0, 0.0),
    DVec3::new(0.0, 10.0, 0.0),
];

/// `names`, and each source primitive's material by index into them.
fn materials(names: &[&str], by_primitive: &[Option<u32>]) -> SourceMaterials {
    SourceMaterials::new(
        names.iter().map(|&n| n.to_owned()).collect(),
        by_primitive
            .iter()
            .map(|m| m.map(MaterialIndex::new))
            .collect(),
    )
}

/// A square of triangles (0, 1, 2) and (0, 2, 3), drawn by primitives 0 and 1, whose only
/// interior edge is the diagonal; `uvs` its corner UVs, in triangle order then corner order.
fn square(
    materials: SourceMaterials,
    uvs: Option<Vec<Option<DVec2>>>,
) -> Result<(ManifoldMesh, EdgeId), Setup> {
    let soup = soup_with_materials(
        CORNERS.to_vec(),
        vec![[0, 1, 2], [0, 2, 3]],
        uvs,
        &[0, 1],
        materials,
    )?;
    let (mesh, _) = into_manifold(&soup, &RepairPlan::default())?;
    let diagonal = edge_between(&mesh, CORNERS[0], CORNERS[2])?;
    Ok((mesh, diagonal))
}

/// Corner UVs for the square that differ across the diagonal at corner 0.
fn cut_square_uvs() -> Option<Vec<Option<DVec2>>> {
    let uvs = [
        (0.0, 0.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (0.5, 0.0),
        (1.0, 1.0),
        (0.0, 1.0),
    ];
    Some(uvs.map(|(u, v)| Some(DVec2::new(u, v))).to_vec())
}

fn boundaries_of(materials: SourceMaterials) -> Result<(Vec<EdgeId>, EdgeId), Setup> {
    let (mesh, diagonal) = square(materials, None)?;
    let found = uv_cuts(&mesh, &SeamSet::new())?;
    Ok((found.material_boundaries().to_vec(), diagonal))
}

#[test]
fn an_edge_between_two_materials_is_a_boundary() -> Result<(), Setup> {
    let (mesh, diagonal) = square(materials(&["Minky", "Fleece"], &[Some(0), Some(1)]), None)?;
    let found = uv_cuts(&mesh, &SeamSet::new())?;
    // The four rim edges each have one face, so they are never boundaries.
    assert_eq!(found.material_boundaries(), &[diagonal]);
    assert_eq!(found.edges(), &[] as &[EdgeId]);
    assert_eq!(found.shared_edges(), 0);
    let proposal = found.proposal()?.ok_or("the boundary is proposed")?;
    assert_eq!(
        proposal.kind(),
        &ProposalKind::Seam {
            reason: SeamReason::SourceModel
        }
    );
    assert_eq!(
        proposal.add(),
        &BTreeMap::from([(diagonal, SeamKind::FabricBoundary)])
    );
    assert_eq!(
        proposal.explanation(),
        "Seams along the 1 edge between the model's materials."
    );
    Ok(())
}

#[test]
fn faces_of_one_material_have_no_boundary_even_from_two_primitives() -> Result<(), Setup> {
    let (mesh, _) = square(materials(&["Minky"], &[Some(0), Some(0)]), None)?;
    let found = uv_cuts(&mesh, &SeamSet::new())?;
    assert_eq!(found.material_boundaries(), &[] as &[EdgeId]);
    assert_eq!(found.proposal()?, None);
    Ok(())
}

#[test]
fn a_source_face_with_no_material_next_to_one_with_a_material_counts() -> Result<(), Setup> {
    for by_primitive in [[Some(0), None], [None, Some(0)]] {
        let (found, diagonal) = boundaries_of(materials(&["Minky"], &by_primitive))?;
        assert_eq!(found, [diagonal], "{by_primitive:?}");
    }
    let (found, _) = boundaries_of(materials(&["Minky"], &[None, None]))?;
    assert_eq!(found, [] as [EdgeId; 0]);
    Ok(())
}

#[test]
fn a_face_s_material_is_its_first_corner_s() -> Result<(), Setup> {
    // The first triangle's first corner is from primitive 0 and its others from primitive 1,
    // as is all of the second triangle: only the first corner tells the faces apart.
    let corners = [0, 1, 1, 1, 1, 1]
        .into_iter()
        .zip(0..)
        .map(|(primitive, vertex)| SourceCorner { primitive, vertex })
        .collect();
    let soup = TriangleSoup::new(CORNERS.to_vec(), vec![[0, 1, 2], [0, 2, 3]], corners, None)?
        .with_materials(materials(&["Minky", "Fleece"], &[Some(0), Some(1)]), 0)?;
    let (mesh, _) = into_manifold(&soup, &RepairPlan::default())?;
    let found = uv_cuts(&mesh, &SeamSet::new())?;
    assert_eq!(
        found.material_boundaries(),
        &[edge_between(&mesh, CORNERS[0], CORNERS[2])?]
    );
    Ok(())
}

#[test]
fn two_materials_that_may_map_to_one_fabric_still_give_a_boundary() -> Result<(), Setup> {
    for names in [["Minky", " minky "], ["Minky", "Minky"]] {
        let (found, diagonal) = boundaries_of(materials(&names, &[Some(0), Some(1)]))?;
        assert_eq!(found, [diagonal], "{names:?}");
    }
    Ok(())
}

/// A cube of side 10 missing its top, its bottom on `CORNERS`, drawn by primitive 0 except the
/// bottom's two faces, drawn by primitive 1, with the top hole filled by repair.
fn filled_cube(materials: SourceMaterials) -> Result<ManifoldMesh, Setup> {
    let p = |x: f64, y: f64, z: f64| DVec3::new(10.0 * x, 10.0 * y, 10.0 * z);
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
    let triangles = vec![
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
    let primitives = [1, 1, 0, 0, 0, 0, 0, 0, 0, 0];
    let soup = soup_with_materials(positions, triangles, None, &primitives, materials)?;
    let (_, open) = into_manifold(&soup, &RepairPlan::default())?;
    let [hole] = open.holes() else {
        return Err("the open cube has one hole".into());
    };
    let plan = RepairPlan {
        fill_holes: BTreeSet::from([hole.loop_ref()]),
        ..RepairPlan::default()
    };
    let (mesh, report) = into_manifold(&soup, &plan)?;
    assert!(
        report.holes().iter().all(|h| h.filled()),
        "the top is filled"
    );
    assert!(mesh.boundary_loops().is_empty());
    Ok(mesh)
}

#[test]
fn an_edge_touching_a_repair_filled_face_is_never_a_boundary() -> Result<(), Setup> {
    // With one material, or none, on every source face, nothing is a boundary: the fill has no
    // material of its own and makes no seam.
    for by_primitive in [[Some(0), Some(0)], [None, None]] {
        let mesh = filled_cube(materials(&["Minky"], &by_primitive))?;
        let found = uv_cuts(&mesh, &SeamSet::new())?;
        assert_eq!(
            found.material_boundaries(),
            &[] as &[EdgeId],
            "{by_primitive:?}"
        );
    }
    // With a second material on the bottom, its rim of four is the only boundary.
    let mesh = filled_cube(materials(&["Minky", "Felt"], &[Some(0), Some(1)]))?;
    let found = uv_cuts(&mesh, &SeamSet::new())?;
    let mut bottom = (0..4)
        .map(|i| edge_between(&mesh, CORNERS[i], CORNERS[(i + 1) % 4]))
        .collect::<Result<Vec<_>, _>>()?;
    bottom.sort();
    assert_eq!(found.material_boundaries(), bottom);
    let proposal = found.proposal()?.ok_or("the boundary is proposed")?;
    assert_eq!(
        proposal.explanation(),
        "Seams along the 4 edges between the model's materials."
    );
    Ok(())
}

/// A fan of four triangles round the origin, drawn by primitives 0, 1, 1, 0, each its own UV
/// island except the last two, which share one. Of its three interior edges, the first is a
/// cut and a boundary, the second a cut only, the third a boundary only.
fn mixed_fan() -> Result<(ManifoldMesh, [EdgeId; 3]), Setup> {
    let positions = vec![
        DVec3::ZERO,
        DVec3::new(10.0, 0.0, 0.0),
        DVec3::new(7.0, 7.0, 0.0),
        DVec3::new(0.0, 10.0, 0.0),
        DVec3::new(-7.0, 7.0, 0.0),
        DVec3::new(-10.0, 0.0, 0.0),
    ];
    let triangles: Vec<[u32; 3]> = (1..5).map(|i| [0, i, i + 1]).collect();
    let islands = [0.0, 1.0, 2.0, 2.0];
    let uvs = triangles
        .iter()
        .zip(islands)
        .flat_map(|(tri, island)| {
            tri.map(|v| {
                let p = positions[v as usize];
                Some(DVec2::new(p.x, p.y) / 100.0 + DVec2::new(10.0 * island, 0.0))
            })
        })
        .collect();
    let soup = soup_with_materials(
        positions.clone(),
        triangles,
        Some(uvs),
        &[0, 1, 1, 0],
        materials(&["Minky", "Fleece"], &[Some(0), Some(1)]),
    )?;
    let (mesh, _) = into_manifold(&soup, &RepairPlan::default())?;
    let spoke = |i: usize| edge_between(&mesh, positions[0], positions[i]);
    let edges = [spoke(2)?, spoke(3)?, spoke(4)?];
    Ok((mesh, edges))
}

#[test]
fn an_edge_both_cut_and_boundary_is_one_fabric_boundary_counted_as_shared() -> Result<(), Setup> {
    let (mesh, [both, cut, boundary]) = mixed_fan()?;
    let found = uv_cuts(&mesh, &SeamSet::new())?;
    let sorted = |mut pair: [EdgeId; 2]| {
        pair.sort();
        pair
    };
    assert_eq!(found.edges(), sorted([both, cut]));
    assert_eq!(found.material_boundaries(), sorted([both, boundary]));
    assert_eq!(found.shared_edges(), 1);
    let proposal = found.proposal()?.ok_or("the seams are proposed")?;
    assert_eq!(
        proposal.add(),
        &BTreeMap::from([
            (both, SeamKind::FabricBoundary),
            (cut, SeamKind::Cut),
            (boundary, SeamKind::FabricBoundary),
        ])
    );
    assert_eq!(
        proposal.explanation(),
        "Seams along the 1 edge where the model's UV map is cut, as marked in the modelling \
         tool, and the 2 edges between its materials."
    );
    assert_eq!(uv_cuts(&mesh, &SeamSet::new())?, found, "repeatable");
    Ok(())
}

#[test]
fn a_cut_on_a_boundary_is_proposed_once() -> Result<(), Setup> {
    let materials = materials(&["Minky", "Fleece"], &[Some(0), Some(1)]);
    let (mesh, diagonal) = square(materials, cut_square_uvs())?;
    let found = uv_cuts(&mesh, &SeamSet::new())?;
    assert_eq!(found.edges(), &[diagonal]);
    assert_eq!(found.material_boundaries(), &[diagonal]);
    assert_eq!(found.shared_edges(), 1);
    let proposal = found.proposal()?.ok_or("the seam is proposed")?;
    assert_eq!(
        proposal.add(),
        &BTreeMap::from([(diagonal, SeamKind::FabricBoundary)])
    );
    assert_eq!(
        proposal.explanation(),
        "Seams along the 1 edge between the model's materials."
    );
    Ok(())
}

#[test]
fn a_boundary_already_a_seam_is_left_out() -> Result<(), Setup> {
    let two = || materials(&["Minky", "Fleece"], &[Some(0), Some(1)]);
    let (mesh, diagonal) = square(two(), None)?;
    let mut elsewhere = SeamSet::new();
    elsewhere.insert(
        edge_between(&mesh, CORNERS[0], CORNERS[1])?,
        SeamKind::Opening,
    )?;
    assert_eq!(
        uv_cuts(&mesh, &elsewhere)?.material_boundaries(),
        &[diagonal]
    );
    for kind in [SeamKind::Cut, SeamKind::Dart, SeamKind::FabricBoundary] {
        let mut seams = SeamSet::new();
        seams.insert(diagonal, kind)?;
        let found = uv_cuts(&mesh, &seams)?;
        assert_eq!(found.material_boundaries(), &[] as &[EdgeId], "{kind:?}");
        assert_eq!(found.proposal()?, None, "{kind:?}");
    }
    let (mesh, diagonal) = square(two(), cut_square_uvs())?;
    let mut seams = SeamSet::new();
    seams.insert(diagonal, SeamKind::Cut)?;
    assert_eq!(uv_cuts(&mesh, &seams)?.shared_edges(), 0);
    Ok(())
}
