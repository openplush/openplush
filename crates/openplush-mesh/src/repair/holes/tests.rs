//! Classifying boundary loops: an opening unless the plan lists it, and a listed loop the mesh
//! does not have is a typed error, never a skipped entry.

use openplush_geom::DVec3;

use super::classify;
use crate::{LoopRef, ManifoldMesh, MeshError, RepairPlan, SourceCorner, TriangleSoup};

fn corner(vertex: u32) -> SourceCorner {
    SourceCorner {
        primitive: 0,
        vertex,
    }
}

fn p(x: f64, y: f64, z: f64) -> DVec3 {
    DVec3::new(x, y, z)
}

/// Corner `i` has source vertex `i`.
fn soup(positions: Vec<DVec3>, triangles: &[[u32; 3]]) -> TriangleSoup {
    let corners = (0..triangles.len() * 3).map(|i| corner(i as u32)).collect();
    TriangleSoup::new(positions, triangles.to_vec(), corners, None).unwrap()
}

/// An open tube: a strip of four quads closed into a ring, so its two rims are two loops.
fn tube() -> ManifoldMesh {
    let mut positions: Vec<DVec3> = Vec::new();
    for r in 0..2 {
        for c in 0..4 {
            let a = std::f64::consts::TAU * f64::from(c) / 4.0;
            positions.push(p(libm::cos(a), libm::sin(a), f64::from(r)));
        }
    }
    let mut triangles = Vec::new();
    for c in 0..4u32 {
        let (a, b, d, e) = (c, (c + 1) % 4, (c + 1) % 4 + 4, c + 4);
        triangles.push([a, b, d]);
        triangles.push([a, d, e]);
    }
    crate::into_manifold(&soup(positions, &triangles), &RepairPlan::default())
        .unwrap()
        .0
}

#[test]
fn every_loop_is_an_opening_unless_the_plan_lists_it() {
    let mesh = tube();
    let holes = classify(&mesh, &RepairPlan::default()).unwrap();
    assert_eq!(holes.len(), 2);
    assert!(holes.iter().all(|h| !h.accidental));
    // Each rim's name is its lowest corner: the first corner of face 0 for the bottom rim, and
    // the first corner at a top vertex for the top rim.
    let names: Vec<LoopRef> = holes.iter().map(|h| h.loop_ref).collect();
    assert!(names.contains(&LoopRef::new(corner(0))), "{names:?}");
    assert!(
        names.iter().all(|&n| n != LoopRef::new(corner(1))),
        "{names:?}"
    );
}

#[test]
fn a_listed_loop_is_accidental_and_the_others_stay_openings() {
    let mesh = tube();
    let mut plan = RepairPlan::default();
    plan.fill_holes.insert(LoopRef::new(corner(0)));
    let holes = classify(&mesh, &plan).unwrap();
    let flags: Vec<(LoopRef, bool)> = holes.iter().map(|h| (h.loop_ref, h.accidental)).collect();
    assert_eq!(
        flags.iter().filter(|(_, accidental)| *accidental).count(),
        1,
        "{flags:?}"
    );
    assert!(
        flags.contains(&(LoopRef::new(corner(0)), true)),
        "{flags:?}"
    );
}

#[test]
fn a_listed_loop_the_mesh_does_not_have_is_refused_by_name_lowest_first() {
    let mesh = tube();
    let mut plan = RepairPlan::default();
    plan.fill_holes.insert(LoopRef::new(corner(0)));
    plan.fill_holes.insert(LoopRef::new(corner(40)));
    plan.fill_holes.insert(LoopRef::new(corner(30)));
    assert_eq!(
        classify(&mesh, &plan).unwrap_err(),
        MeshError::UnknownHoleLoop {
            loop_ref: LoopRef::new(corner(30))
        }
    );
}

#[test]
fn a_closed_mesh_has_no_holes() {
    let mesh = crate::into_manifold(
        &soup(
            vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(0., 0., 1.)],
            &[[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]],
        ),
        &RepairPlan::default(),
    )
    .unwrap()
    .0;
    assert!(classify(&mesh, &RepairPlan::default()).unwrap().is_empty());
}

#[test]
fn a_name_two_loops_share_is_refused_and_neither_is_filled() {
    // Two triangles touching at vertex 0, with every corner of a vertex sharing its source, as
    // an indexed file has it: the bowtie split keeps the sources, so both loops get one name.
    let positions = vec![
        p(0., 0., 0.),
        p(1., 0., 0.),
        p(0., 1., 0.),
        p(-1., 0., 0.),
        p(0., -1., 0.),
    ];
    let triangles = [[0, 1, 2], [0, 3, 4]];
    let corners = triangles.iter().flatten().map(|&v| corner(v)).collect();
    let soup = TriangleSoup::new(positions, triangles.to_vec(), corners, None).unwrap();
    let shared = LoopRef::new(corner(0));
    let mut plan = RepairPlan::default();
    plan.fill_holes.insert(shared);
    assert_eq!(
        crate::into_manifold(&soup, &plan)
            .err()
            .map(|e| e.to_string()),
        Some(format!(
            "the plan fills the loop {shared:?}, but more than one hole has that name, so none was filled"
        ))
    );
}
