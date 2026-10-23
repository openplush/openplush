use openplush_geom::{DVec2, DVec3};
use openplush_mesh::VertexId;
use openplush_seams::ChartId;

use super::{System, descend};
use crate::boundary::fixtures::{Faces, grid};
use crate::injectivity;
use crate::interior::energy::{Rest, max_step};
use crate::interior::energy::{energy, rest_shapes};
use crate::interior::free::Free;
use crate::interior::proxy::proxy;

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
        let target = system
            .target(&rest, &faces, &uv)
            .map_err(|e| format!("{e:?}"))?;
        let dir: Vec<DVec2> = target.iter().zip(&uv).map(|(t, u)| *t - *u).collect();
        let Some((next, lower)) = descend(&rest, &faces, &uv, &dir, e) else {
            break;
        };
        assert!(lower < e, "an accepted step lowers the energy");
        assert_eq!(
            injectivity::verify(ChartId::new(0), &tris(&faces, &next)),
            Ok(())
        );
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
    let target = system
        .target(&rest, &faces, &uv)
        .map_err(|e| format!("{e:?}"))?;
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
    let target = system
        .target(&rest, &faces, &uv)
        .map_err(|e| format!("{e:?}"))?;
    let dir: Vec<DVec2> = target
        .iter()
        .zip(&uv)
        .map(|(t, u)| (*t - *u) * 50.0)
        .collect();
    let (next, lower) = descend(&rest, &faces, &uv, &dir, e).ok_or("no step found")?;
    assert!(lower < e);
    assert_eq!(
        injectivity::verify(ChartId::new(0), &tris(&faces, &next)),
        Ok(())
    );
    Ok(())
}

#[test]
fn no_step_is_found_at_the_minimum() -> Result<(), String> {
    let (xyz, faces, free, _) = shoved();
    let lattice: Vec<DVec2> = xyz.iter().map(|p| DVec2::new(p.x, p.y)).collect();
    let rest = rest_shapes(&xyz, &faces).map_err(|f| format!("{f:?}"))?;
    let system = System::new(&faces, &free).map_err(|e| format!("{e:?}"))?;
    let e = energy(&rest, &faces, &lattice).ok_or("flipped")?;
    let target = system
        .target(&rest, &faces, &lattice)
        .map_err(|e| format!("{e:?}"))?;
    let dir: Vec<DVec2> = target.iter().zip(&lattice).map(|(t, u)| *t - *u).collect();
    assert!(descend(&rest, &faces, &lattice, &dir, e).is_none());
    Ok(())
}

/// `shoved`, with column spacing growing so face areas differ by an order of magnitude.
fn uneven() -> (Vec<DVec3>, Faces, Free, Vec<DVec2>) {
    let (mut xyz, faces, free, mut uv) = shoved();
    let x = [0.0, 0.5, 1.5, 3.5, 7.5, 15.5];
    for (p, q) in xyz.iter_mut().zip(uv.iter_mut()) {
        let c = p.x as usize;
        let shift = q.x - p.x;
        p.x = x[c];
        q.x = x[c] + shift * (x[c.min(4) + 1] - x[c.min(4)]);
    }
    (xyz, faces, free, uv)
}

/// The proxy objective `Σ A ‖W (J − R)‖²` with the weights and rotations fixed at `at`.
fn proxy_objective(rest: &[Rest], faces: &Faces, at: &[DVec2], uv: &[DVec2]) -> f64 {
    rest.iter()
        .zip(faces)
        .map(|(r, (_, t))| {
            let p = proxy(r.jacobian(at, t));
            let j = r.jacobian(uv, t);
            let d = [0, 1].map(|a| [0, 1].map(|b| j[a][b] - p.rotation[a][b]));
            let mut sum = 0.0;
            for m in [0, 1] {
                for a in 0..2 {
                    for b in 0..2 {
                        sum += d[a][m] * p.weight[a][b] * d[b][m];
                    }
                }
            }
            r.area() * sum
        })
        .sum()
}

#[test]
fn the_target_minimises_the_area_weighted_proxy() -> Result<(), String> {
    let (xyz, faces, free, uv) = uneven();
    let rest = rest_shapes(&xyz, &faces).map_err(|f| format!("{f:?}"))?;
    let system = System::new(&faces, &free).map_err(|e| format!("{e:?}"))?;
    let target = system
        .target(&rest, &faces, &uv)
        .map_err(|e| format!("{e:?}"))?;
    let best = proxy_objective(&rest, &faces, &uv, &target);
    for (v, slot) in free.slot.iter().enumerate() {
        if slot.is_none() {
            continue;
        }
        for nudge in [
            DVec2::new(1e-3, 0.0),
            DVec2::new(-1e-3, 0.0),
            DVec2::new(0.0, 1e-3),
            DVec2::new(0.0, -1e-3),
        ] {
            let mut moved = target.clone();
            moved[v] += nudge;
            assert!(
                proxy_objective(&rest, &faces, &uv, &moved) >= best - 1e-12,
                "vertex {v}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_short_direction_is_taken_whole_and_a_long_one_by_four_fifths_of_the_flip_limit()
-> Result<(), String> {
    let (xyz, faces, free, uv) = shoved();
    let rest = rest_shapes(&xyz, &faces).map_err(|f| format!("{f:?}"))?;
    let system = System::new(&faces, &free).map_err(|e| format!("{e:?}"))?;
    let e = energy(&rest, &faces, &uv).ok_or("start flipped")?;
    let target = system
        .target(&rest, &faces, &uv)
        .map_err(|e| format!("{e:?}"))?;
    let toward: Vec<DVec2> = target.iter().zip(&uv).map(|(t, u)| *t - *u).collect();
    let short: Vec<DVec2> = toward.iter().map(|d| *d * 0.01).collect();
    let (next, _) = descend(&rest, &faces, &uv, &short, e).ok_or("no step")?;
    for ((n, u), d) in next.iter().zip(&uv).zip(&short) {
        assert_eq!(*n, *u + *d);
    }
    let long: Vec<DVec2> = toward.iter().map(|d| *d * 50.0).collect();
    let limit = 0.8 * max_step(&faces, &uv, &long);
    let (next, _) = descend(&rest, &faces, &uv, &long, e).ok_or("no step")?;
    let halvings = (0..40).find(|h| {
        let t = libm::ldexp(limit, -*h);
        next.iter()
            .zip(&uv)
            .zip(&long)
            .all(|((n, u), d)| *n == *u + *d * t)
    });
    assert!(
        halvings.is_some(),
        "the accepted step is the limit, halved some number of times"
    );
    Ok(())
}
