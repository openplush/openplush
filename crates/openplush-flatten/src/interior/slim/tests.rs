use openplush_geom::{DVec2, DVec3};
use openplush_mesh::VertexId;
use openplush_seams::ChartId;

use super::{System, descend};
use crate::boundary::fixtures::{Faces, grid};
use crate::injectivity;
use crate::interior::energy::{energy, rest_shapes};
use crate::interior::free::Free;

const SIDE: u32 = 5;

/// A flat grid on the lattice, its rim fixed, its interior vertices shoved off it.
fn shoved() -> (Vec<DVec3>, Faces, Free, Vec<DVec2>) {
    let id = |c: u32, r: u32| r * (SIDE + 1) + c;
    let on_rim = |c: u32, r: u32| c == 0 || r == 0 || c == SIDE || r == SIDE;
    let lattice = || (0..=SIDE).flat_map(|r| (0..=SIDE).map(move |c| (c, r)));
    let xyz: Vec<DVec3> = lattice()
        .map(|(c, r)| DVec3::new(f64::from(c), f64::from(r), 0.0))
        .collect();
    let uv: Vec<DVec2> = lattice()
        .map(|(c, r)| {
            let shove = if on_rim(c, r) {
                DVec2::ZERO
            } else {
                DVec2::new(
                    0.3 * f64::from((c * 3 + r) % 4) - 0.45,
                    0.3 * f64::from((c + r * 5) % 3) - 0.3,
                )
            };
            DVec2::new(f64::from(c), f64::from(r)) + shove
        })
        .collect();
    let rim: Vec<VertexId> = lattice()
        .filter(|&(c, r)| on_rim(c, r))
        .map(|(c, r)| VertexId::new(id(c, r)))
        .collect();
    let faces = grid(SIDE, SIDE);
    let free = Free::new(xyz.len(), &faces, &rim);
    (xyz, faces, free, uv)
}

fn tris(faces: &Faces, uv: &[DVec2]) -> Vec<(openplush_mesh::FaceId, [DVec2; 3])> {
    faces
        .iter()
        .map(|&(f, t)| (f, t.map(|v| uv[v.index() as usize])))
        .collect()
}

#[test]
fn iterating_a_shoved_flat_chart_returns_to_the_isometry() -> Result<(), String> {
    let (xyz, faces, free, mut uv) = shoved();
    let rest = rest_shapes(&xyz, &faces).map_err(|f| format!("{f:?}"))?;
    let system = System::new(&faces, &free).map_err(|e| format!("{e:?}"))?;
    let mut e = energy(&rest, &faces, &uv).ok_or("start flipped")?;
    assert!(e > 4.1, "the shove must cost something: {e}");
    for _ in 0..200 {
        let target = system.target(&rest, &faces, &uv).map_err(|e| format!("{e:?}"))?;
        let dir: Vec<DVec2> = target.iter().zip(&uv).map(|(t, u)| *t - *u).collect();
        let Some((next, lower)) = descend(&rest, &faces, &uv, &dir, e) else {
            break;
        };
        assert!(lower < e, "an accepted step lowers the energy");
        assert_eq!(injectivity::verify(ChartId::new(0), &tris(&faces, &next)), Ok(()));
        (uv, e) = (next, lower);
    }
    assert!((e - 4.0).abs() <= 1e-6, "energy {e}");
    Ok(())
}

#[test]
fn the_rim_never_moves() -> Result<(), String> {
    let (xyz, faces, free, uv) = shoved();
    let rest = rest_shapes(&xyz, &faces).map_err(|f| format!("{f:?}"))?;
    let system = System::new(&faces, &free).map_err(|e| format!("{e:?}"))?;
    let target = system.target(&rest, &faces, &uv).map_err(|e| format!("{e:?}"))?;
    for (v, slot) in free.slot.iter().enumerate() {
        if slot.is_none() {
            assert_eq!(target[v], uv[v], "vertex {v}");
        }
    }
    Ok(())
}

#[test]
fn an_overlong_direction_is_cut_back_before_any_face_flips() -> Result<(), String> {
    let (xyz, faces, free, uv) = shoved();
    let rest = rest_shapes(&xyz, &faces).map_err(|f| format!("{f:?}"))?;
    let system = System::new(&faces, &free).map_err(|e| format!("{e:?}"))?;
    let e = energy(&rest, &faces, &uv).ok_or("start flipped")?;
    let target = system.target(&rest, &faces, &uv).map_err(|e| format!("{e:?}"))?;
    let dir: Vec<DVec2> = target
        .iter()
        .zip(&uv)
        .map(|(t, u)| (*t - *u) * 50.0)
        .collect();
    let (next, lower) = descend(&rest, &faces, &uv, &dir, e).ok_or("no step found")?;
    assert!(lower < e);
    assert_eq!(injectivity::verify(ChartId::new(0), &tris(&faces, &next)), Ok(()));
    Ok(())
}

#[test]
fn no_step_is_found_at_the_minimum() -> Result<(), String> {
    let (xyz, faces, free, _) = shoved();
    let lattice: Vec<DVec2> = xyz.iter().map(|p| DVec2::new(p.x, p.y)).collect();
    let rest = rest_shapes(&xyz, &faces).map_err(|f| format!("{f:?}"))?;
    let system = System::new(&faces, &free).map_err(|e| format!("{e:?}"))?;
    let e = energy(&rest, &faces, &lattice).ok_or("flipped")?;
    let target = system.target(&rest, &faces, &lattice).map_err(|e| format!("{e:?}"))?;
    let dir: Vec<DVec2> = target.iter().zip(&lattice).map(|(t, u)| *t - *u).collect();
    assert!(descend(&rest, &faces, &lattice, &dir, e).is_none());
    Ok(())
}
