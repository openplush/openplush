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
            DVec3::new(a.cos(), a.sin(), 0.0)
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
