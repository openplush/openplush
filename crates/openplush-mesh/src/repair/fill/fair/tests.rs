//! Fairing, against the equations it solves.

use std::collections::{BTreeMap, BTreeSet};

use openplush_geom::DVec3;

use super::{fair, solve};
use crate::repair::fill::refine::Patch;

/// The uniform Laplacian at each patch vertex, its neighbours read off the triangles.
fn laplacians(patch: &Patch, outer: &[Vec<DVec3>]) -> Vec<DVec3> {
    let mut near: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); patch.pos.len()];
    for t in &patch.tris {
        for k in 0..3 {
            near[t[k]].insert(t[(k + 1) % 3]);
            near[t[(k + 1) % 3]].insert(t[k]);
        }
    }
    (0..patch.pos.len())
        .map(|v| {
            let ring: Vec<DVec3> = near[v]
                .iter()
                .map(|&w| patch.pos[w])
                .chain(outer.get(v).into_iter().flatten().copied())
                .collect();
            let sum = ring.iter().fold(DVec3::ZERO, |s, &p| s + p);
            patch.pos[v] * ring.len() as f64 - sum
        })
        .collect()
}

#[test]
fn the_added_vertices_solve_the_bilaplacian_with_the_rim_and_its_ring_fixed() {
    // A bent square rim, two added vertices, and an outer ring of uneven sizes, so each rim
    // vertex's row differs and every one of its neighbours shows in the answer.
    let p = DVec3::new;
    let mut patch = Patch {
        pos: vec![
            p(0.0, 0.0, 0.0),
            p(2.0, 0.0, 0.5),
            p(2.0, 2.0, 0.0),
            p(0.0, 2.0, 1.0),
            p(0.7, 1.0, 0.0),
            p(1.3, 1.0, 0.0),
        ],
        sigma: vec![1.0; 6],
        rim: 4,
        tris: vec![
            [0, 1, 4],
            [1, 5, 4],
            [1, 2, 5],
            [2, 3, 5],
            [3, 4, 5],
            [3, 0, 4],
        ],
    };
    let outer = vec![
        vec![p(-1.0, -1.0, 0.5)],
        vec![p(3.0, -1.0, 0.0), p(3.0, 0.5, 1.0)],
        vec![],
        vec![p(-1.0, 3.0, -1.0), p(0.5, 3.0, 2.0), p(-1.0, 1.0, 0.0)],
    ];
    assert_eq!(fair(&mut patch, &outer), Some(()));
    let lap = laplacians(&patch, &outer);
    let near: BTreeMap<usize, Vec<usize>> =
        BTreeMap::from([(4, vec![0, 1, 3, 5]), (5, vec![1, 2, 3, 4])]);
    for (&i, ring) in &near {
        let bilaplacian = ring
            .iter()
            .fold(lap[i] * ring.len() as f64, |s, &w| s - lap[w]);
        assert!(bilaplacian.length() < 1e-12, "{i}: {bilaplacian:?}");
    }
}

#[test]
fn a_solve_that_overflows_is_refused() {
    let lower = BTreeMap::from([((0, 0), 1e-300)]);
    assert_eq!(
        solve(1, &lower, &[DVec3::new(f64::MAX, f64::MAX, f64::MAX)]),
        None
    );
}
