//! The triangulation's choice of chords, against hand-computed loops.

use openplush_geom::DVec3;
use openplush_geom::epsilon::ModelTolerances;

use super::{Rim, triangulate};

fn tol() -> ModelTolerances {
    ModelTolerances::for_diagonal(2.0).unwrap()
}

fn area(points: &[DVec3], tris: &[[usize; 3]]) -> f64 {
    tris.iter()
        .map(|&[a, b, c]| {
            0.5 * (points[b] - points[a])
                .cross(points[c] - points[a])
                .length()
        })
        .sum()
}

#[test]
fn a_flat_convex_loop_is_covered_exactly_once() {
    let points: Vec<DVec3> = (0..6)
        .map(|k| {
            let a = std::f64::consts::TAU * f64::from(k) / 6.0;
            DVec3::new(libm::cos(a), libm::sin(a), 0.0)
        })
        .collect();
    let across = vec![DVec3::Z; 6];
    let rim = Rim {
        points: &points,
        across: &across,
        joined: &|_, _| false,
    };
    let tris = triangulate(&rim, &tol()).unwrap();
    assert_eq!(tris.len(), 4);
    let hexagon = 1.5 * 3f64.sqrt();
    assert!((area(&points, &tris) - hexagon).abs() < 1e-12);
    for t in &tris {
        let n = (points[t[1]] - points[t[0]]).cross(points[t[2]] - points[t[0]]);
        assert!(n.z > 0.0, "{t:?} is wound against the loop");
    }
}

#[test]
fn a_bent_loop_takes_the_chord_that_keeps_it_flattest() {
    // A unit square in z = 0 with vertex 2 lifted to z = 1, in a flat surround. Chord 0-2
    // makes two tilted halves with a crease of 1 - 1/2 between them; chord 1-3 keeps one
    // half flat and creases by 1 - 1/sqrt(3), less, against the flat surround as well.
    let points = vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(1.0, 1.0, 1.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let across = vec![DVec3::Z; 4];
    let rim = Rim {
        points: &points,
        across: &across,
        joined: &|_, _| false,
    };
    let mut tris = triangulate(&rim, &tol()).unwrap();
    tris.sort();
    assert_eq!(tris, vec![[0, 1, 3], [1, 2, 3]]);
}

#[test]
fn a_chord_the_mesh_already_has_is_not_used() {
    let points = vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(1.0, 1.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let across = vec![DVec3::Z; 4];
    let joined = |i: usize, k: usize| (i, k) == (1, 3);
    let rim = Rim {
        points: &points,
        across: &across,
        joined: &joined,
    };
    let mut tris = triangulate(&rim, &tol()).unwrap();
    tris.sort();
    assert_eq!(tris, vec![[0, 1, 2], [0, 2, 3]]);
    let both = |_: usize, _: usize| true;
    let rim = Rim {
        joined: &both,
        ..rim
    };
    assert_eq!(triangulate(&rim, &tol()), None);
}

/// The triangulation of a loop with no existing chords, sorted.
fn free_triangulation(points: &[DVec3], across: &[DVec3]) -> Option<Vec<[usize; 3]>> {
    let rim = Rim {
        points,
        across,
        joined: &|_, _| false,
    };
    let mut tris = triangulate(&rim, &tol())?;
    tris.sort();
    Some(tris)
}

#[test]
fn a_smaller_largest_angle_beats_a_smaller_area() {
    // The bent square again, with the face across the closing edge 3-0 standing upright.
    // Chord 1-3 keeps the flat half against it at a right angle; chord 0-2 tilts that half
    // towards it, for a smaller worst crease and a larger area.
    let points = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(1.0, 1.0, 1.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let across = [DVec3::Z, DVec3::Z, DVec3::Z, -DVec3::X];
    assert_eq!(
        free_triangulation(&points, &across),
        Some(vec![[0, 1, 2], [0, 2, 3]])
    );
}

#[test]
fn between_equal_largest_angles_the_smaller_area_wins() {
    // The face across edge 0-1 is given the edge's own direction, so every triangle on that
    // edge meets it at exactly a right angle and both chords tie on angle.
    let points = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 1.0),
        DVec3::new(1.0, 1.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let edge = DVec3::new(1.0, 0.0, 1.0).normalize().unwrap();
    let across = [edge, DVec3::Z, DVec3::Z, DVec3::Z];
    assert_eq!(
        free_triangulation(&points, &across),
        Some(vec![[0, 1, 2], [0, 2, 3]])
    );
}

#[test]
fn a_triangle_with_no_area_is_not_chosen() {
    // Vertex 2 sits 2^-40 off chord 1-3, so triangle 1-2-3 is a sliver; the flat loop
    // otherwise ties and would take it.
    let off = 0.5 + libm::ldexp(1.0, -40);
    let points = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(off, off, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    assert_eq!(
        free_triangulation(&points, &[DVec3::Z; 4]),
        Some(vec![[0, 1, 2], [0, 2, 3]])
    );
}

#[test]
fn a_crease_inside_a_sub_loop_counts_towards_the_largest_angle() {
    // A pentagon with vertex 1 dropped. Enumerating its five triangulations by hand, the
    // fan around vertex 1 has the smallest worst crease (0.169); the fan 1-3, 0-3 creases
    // 0.219 between triangles that only meet inside the sub-loop 0..=3.
    let points = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(2.0, 0.0, -1.0),
        DVec3::new(3.0, 2.0, 0.0),
        DVec3::new(1.0, 3.0, 0.0),
        DVec3::new(-1.0, 2.0, 0.0),
    ];
    assert_eq!(
        free_triangulation(&points, &[DVec3::Z; 5]),
        Some(vec![[0, 1, 4], [1, 2, 3], [1, 3, 4]])
    );
}

#[test]
fn a_tie_on_angle_goes_to_the_least_total_area() {
    // Edge 0-1's across normal is the edge's direction, so every triangulation's worst
    // angle is exactly 1 and area decides. The fan from vertex 0 has the least total area
    // (5.15, against 5.56 next), though its last triangle is not the smallest apex.
    let points = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 1.0),
        DVec3::new(1.5, 0.0, 0.5),
        DVec3::new(1.5, 3.0, 0.0),
        DVec3::new(-1.0, 1.0, 0.0),
    ];
    let edge = DVec3::new(1.0, 0.0, 1.0).normalize().unwrap();
    let across = [edge, DVec3::Z, DVec3::Z, DVec3::Z, DVec3::Z];
    assert_eq!(
        free_triangulation(&points, &across),
        Some(vec![[0, 1, 2], [0, 2, 3], [0, 3, 4]])
    );
}
