//! What repair step 7's fill reads from the mesh around a loop: the faces across its edges,
//! the edges the mesh already has between its vertices, the split edges along it, and the
//! ring of vertices that fixes the patch's slope.

use std::collections::BTreeSet;

use openplush_geom::DVec3;
use openplush_geom::epsilon::ModelTolerances;
use openplush_mesh::{
    FaceId, LoopRef, ManifoldMesh, MeshError, RepairPlan, SourceCorner, TriangleSoup, into_manifold,
};

type Setup = Box<dyn std::error::Error>;

fn corner(vertex: u32) -> SourceCorner {
    SourceCorner {
        primitive: 0,
        vertex,
    }
}

/// Every corner's source vertex is its own position index, so a loop is named by its lowest.
fn soup(positions: Vec<DVec3>, triangles: &[[u32; 3]]) -> Result<TriangleSoup, Setup> {
    let corners = triangles.iter().flatten().map(|&v| corner(v)).collect();
    Ok(TriangleSoup::new(
        positions,
        triangles.to_vec(),
        corners,
        None,
    )?)
}

fn filling(loop_ref: LoopRef) -> RepairPlan {
    RepairPlan {
        fill_holes: BTreeSet::from([loop_ref]),
        ..RepairPlan::default()
    }
}

/// Fills the loop named by vertex `lowest` and returns the mesh and the added faces'
/// corner positions.
fn fill(soup: &TriangleSoup, lowest: u32) -> Result<(ManifoldMesh, Vec<[DVec3; 3]>), Setup> {
    let loop_ref = LoopRef::new(corner(lowest));
    let (m, report) = into_manifold(soup, &filling(loop_ref))?;
    let hole = report
        .holes()
        .iter()
        .find(|h| h.loop_ref() == loop_ref)
        .ok_or("the loop is not reported")?;
    let corners = hole
        .added_faces()
        .iter()
        .map(|&f: &FaceId| {
            let h = m.face_half_edge(f);
            [h, m.next(h), m.prev(h)].map(|k| m.position(m.origin(k)))
        })
        .collect();
    let diagonal = soup.bbox_diagonal().ok_or("no extent")?;
    m.debug_validate(&ModelTolerances::for_diagonal(diagonal)?)?;
    Ok((m, corners))
}

/// The square loop (0,0,0), (1,0,0), (1,1,1), (0,1,0): a chord 0-2 or 1-3 closes it.
fn bent_square() -> Vec<DVec3> {
    vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(1., 1., 1.),
        DVec3::new(0., 1., 0.),
    ]
}

#[test]
fn the_chord_follows_the_faces_across_the_loop_not_the_loop_s_own_corners() -> Result<(), Setup> {
    // The bent square, with the faces across edges 1-2, 2-3 and 3-0 in the planes of the
    // two halves chord 0-2 makes, and the face across 0-1 falling away steeply. Chord 0-2
    // creases by 0.5; chord 1-3 leaves a flat half that meets that steep face at 0.55. The
    // loop's own corner normals would favour chord 1-3 instead.
    let mut positions = bent_square();
    positions.extend([
        DVec3::new(0.5, -1.5, -3.),
        DVec3::new(3., 0.5, 0.5),
        DVec3::new(0.5, 3., 0.5),
        DVec3::new(-2., 0.5, -2.),
    ]);
    let ring = soup(
        positions,
        &[
            [1, 0, 4],
            [2, 1, 5],
            [3, 2, 6],
            [0, 3, 7],
            [1, 4, 5],
            [2, 5, 6],
            [3, 6, 7],
            [0, 7, 4],
        ],
    )?;
    let (_, added) = fill(&ring, 0)?;
    let square = bent_square();
    assert_eq!(added.len(), 2, "{added:?}");
    for face in &added {
        assert!(
            face.contains(&square[0]) && face.contains(&square[2]),
            "{face:?} is not on chord 0-2"
        );
    }
    Ok(())
}

#[test]
fn an_edge_the_mesh_already_has_is_not_made_again_as_a_chord() -> Result<(), Setup> {
    // A closed surface but for the flat square 0-1-2-3: below it, edge 1-3 parts two
    // tents with apexes 4 and 5. Both chords fill the square flat, but 1-3 would make a
    // second edge between those vertices.
    let p = DVec3::new;
    let positions = vec![
        p(0., 0., 0.),
        p(1., 0., 0.),
        p(1., 1., 0.),
        p(0., 1., 0.),
        p(0.3, 0.3, -1.),
        p(0.7, 0.7, -1.),
    ];
    let tents = soup(
        positions.clone(),
        &[
            [1, 0, 4],
            [0, 3, 4],
            [3, 1, 4],
            [3, 2, 5],
            [2, 1, 5],
            [1, 3, 5],
        ],
    )?;
    let (m, added) = fill(&tents, 0)?;
    assert!(m.boundary_loops().is_empty());
    assert_eq!(added.len(), 2, "{added:?}");
    for face in &added {
        assert!(
            face.contains(&positions[0]) && face.contains(&positions[2]),
            "{face:?} is not on chord 0-2"
        );
    }
    Ok(())
}

#[test]
fn every_loop_along_a_split_edge_is_refused_wherever_the_split_edge_falls_on_it()
-> Result<(), Setup> {
    // Three faces on edge 0-1, which repair splits; each face is listed from its third
    // corner, so a loop's split edge need not be its first.
    let p = DVec3::new;
    let book = soup(
        vec![
            p(0., 0., 0.),
            p(1., 0., 0.),
            p(0., 1., 0.),
            p(0., -1., 0.),
            p(0., 0., 1.),
        ],
        &[[2, 0, 1], [3, 1, 0], [4, 0, 1]],
    )?;
    let (_, report) = into_manifold(&book, &RepairPlan::default())?;
    assert!(!report.holes().is_empty());
    for hole in report.holes() {
        let rim = hole.loop_ref();
        assert_eq!(
            into_manifold(&book, &filling(rim)).err(),
            Some(MeshError::HoleOnSplit { loop_ref: rim })
        );
    }
    Ok(())
}

#[test]
fn a_patch_that_fairing_flattens_into_a_sliver_is_not_fillable() -> Result<(), Setup> {
    // A flat triangular hole a, b, c large enough against its rim's edges to take one
    // centroid vertex q. With one unknown the bilaplacian puts q at the centroid less a
    // twelfth of the sum of the outer ring's offsets from their rim vertices. Vertex b's
    // long fan, and e_bc set to make that sum exact, land q on line c-a.
    let p = |x: f64, y: f64| DVec3::new(x, y, 0.0);
    let positions = vec![
        p(-1.0, 0.0),
        p(1.0, 0.0),
        p(0.0, 1.75),
        p(0.5, -1.0),
        p(1.5, 1.1875),
        p(-0.75, 1.0),
        p(-1.25, -0.5),
        p(0.25, 2.25),
        p(1.5, -1.0),
        p(2.0, -0.75),
        p(2.25, -0.25),
        p(2.25, 0.5),
    ];
    let (a, b, c, e_ab, e_bc, e_ca, a1, c1) = (0, 1, 2, 3, 4, 5, 6, 7);
    let (b0, b1, b2, b3) = (8, 9, 10, 11);
    let sheet = soup(
        positions,
        &[
            [b, a, e_ab],
            [b, e_ab, b0],
            [b, b0, b1],
            [b, b1, b2],
            [b, b2, b3],
            [b, b3, e_bc],
            [c, b, e_bc],
            [c, e_bc, c1],
            [c, c1, e_ca],
            [a, c, e_ca],
            [a, e_ca, a1],
            [a, a1, e_ab],
        ],
    )?;
    let rim = LoopRef::new(corner(a));
    assert_eq!(
        into_manifold(&sheet, &filling(rim)).err(),
        Some(MeshError::HoleNotFillable { loop_ref: rim })
    );
    Ok(())
}
