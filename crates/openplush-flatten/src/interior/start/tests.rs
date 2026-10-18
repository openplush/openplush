use std::collections::BTreeMap;

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

use super::{choose, harmonic};
use crate::FlattenError;
use crate::boundary::cotangent_weights;
use crate::boundary::fixtures::{Faces, grid};
use crate::injectivity;
use crate::interior::free::Free;

const CHART: ChartId = ChartId::new(2);

fn flip(faces: &[u32]) -> Result<(), FlattenError> {
    Err(FlattenError::Flip {
        chart: CHART,
        faces: faces.iter().map(|&f| FaceId::new(f)).collect(),
    })
}

#[test]
fn the_first_start_wins_when_it_is_injective() {
    assert_eq!(choose([Ok(()), flip(&[1])]), Ok(0));
    assert_eq!(choose([Ok(()), Ok(())]), Ok(0));
}

#[test]
fn the_second_start_is_used_when_only_the_first_flips() {
    assert_eq!(choose([flip(&[1, 2]), Ok(())]), Ok(1));
}

#[test]
fn two_flipped_starts_report_the_one_with_fewer_faces_and_the_first_on_a_tie() {
    assert_eq!(choose([flip(&[1, 2]), flip(&[7])]), flip(&[7]).map(|()| 1));
    assert_eq!(choose([flip(&[1]), flip(&[7, 8])]), flip(&[1]).map(|()| 0));
    assert_eq!(choose([flip(&[1]), flip(&[7])]), flip(&[1]).map(|()| 0));
}

#[test]
fn a_start_no_predicate_can_judge_is_reported_at_once() {
    let unjudgeable = FlattenError::Unjudgeable {
        chart: CHART,
        face: FaceId::new(4),
        source: openplush_geom::GeomError::NonFinite,
    };
    assert_eq!(
        choose([Err(unjudgeable.clone()), Ok(())]),
        Err(unjudgeable.clone())
    );
    assert_eq!(choose([flip(&[1]), Err(unjudgeable.clone())]), Err(unjudgeable));
}

/// A flat 5 by 5 grid on `(c, r)` with its interior vertices moved off the lattice, and the
/// numbering that frees them.
fn bent_grid() -> (Vec<DVec3>, Faces, Free) {
    let (cols, rows) = (5u32, 5u32);
    let on_rim = |c: u32, r: u32| c == 0 || r == 0 || c == cols || r == rows;
    let xyz: Vec<DVec3> = (0..=rows)
        .flat_map(|r| (0..=cols).map(move |c| (c, r)))
        .map(|(c, r)| {
            let wobble = if on_rim(c, r) {
                DVec2::ZERO
            } else {
                DVec2::new(0.2 * f64::from((c * 7 + r * 3) % 5) - 0.4, 0.15 * f64::from((c * 5 + r * 11) % 4) - 0.2)
            };
            DVec3::new(f64::from(c) + wobble.x, f64::from(r) + wobble.y, 0.0)
        })
        .collect();
    let faces = grid(cols, rows);
    let rim: Vec<VertexId> = (0..=rows)
        .flat_map(|r| (0..=cols).map(move |c| (c, r)))
        .filter(|&(c, r)| on_rim(c, r))
        .map(|(c, r)| VertexId::new(r * (cols + 1) + c))
        .collect();
    let free = Free::new(xyz.len(), &faces, &rim);
    (xyz, faces, free)
}

fn flat(xyz: &[DVec3]) -> Vec<DVec2> {
    xyz.iter().map(|p| DVec2::new(p.x, p.y)).collect()
}

#[test]
fn the_cotangent_fill_of_a_flat_chart_is_the_identity() -> Result<(), crate::solver::SolveError> {
    let (xyz, faces, free) = bent_grid();
    let weights = cotangent_weights(&xyz, &faces).map_err(|_| crate::solver::SolveError::Empty)?;
    let mut uv = flat(&xyz);
    for (v, slot) in free.slot.iter().enumerate() {
        if slot.is_some() {
            uv[v] = DVec2::new(-9.0, 9.0);
        }
    }
    let filled = harmonic(&free, &weights, &uv)?;
    for (got, want) in filled.iter().zip(flat(&xyz)) {
        assert!((*got - want).length() <= 1e-9);
    }
    Ok(())
}

#[test]
fn the_uniform_fill_inside_a_convex_boundary_flips_no_face() -> Result<(), crate::solver::SolveError> {
    let (xyz, faces, free) = bent_grid();
    let uniform: BTreeMap<(u32, u32), f64> =
        faces.iter().flat_map(|(_, t)| {
            let t = t.map(VertexId::index);
            [(t[0], t[1]), (t[1], t[2]), (t[0], t[2])].map(|(a, b)| ((a.min(b), a.max(b)), 1.0))
        }).collect();
    let filled = harmonic(&free, &uniform, &flat(&xyz))?;
    let tris: Vec<_> = faces
        .iter()
        .map(|&(f, t)| (f, t.map(|v| filled[v.index() as usize])))
        .collect();
    assert_eq!(injectivity::verify(CHART, &tris), Ok(()));
    Ok(())
}
