//! A Taylor-remainder check of the energy against a closed-form gradient: the remainder of the
//! first-order model shrinks as h² only when the gradient is right.

use openplush_geom::DVec2;

use super::super::energy::{Rest, energy, max_step};
use super::super::{Faces, Solve};
use super::{CHART, hemisphere};
use crate::FlattenError;

type Mat = [[f64; 2]; 2];

/// The hemisphere's rim starts at vertex 1 + 3·16·15; it is pinned, so the direction leaves it.
const RIM: usize = 721;

/// ∂(|J|² + |J⁻¹|²)/∂J = 2J − 2 J⁻ᵀ J⁻¹ J⁻ᵀ.
fn face_gradient(j: Mat) -> Mat {
    let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
    let inv = [
        [j[1][1] / det, -j[0][1] / det],
        [-j[1][0] / det, j[0][0] / det],
    ];
    let inv_t = [[inv[0][0], inv[1][0]], [inv[0][1], inv[1][1]]];
    let m = mul(mul(inv_t, inv), inv_t);
    [0, 1].map(|k| [0, 1].map(|l| 2.0 * j[k][l] - 2.0 * m[k][l]))
}

fn mul(a: Mat, b: Mat) -> Mat {
    [0, 1].map(|k| [0, 1].map(|l| a[k][0] * b[0][l] + a[k][1] * b[1][l]))
}

/// The gradient of the area-weighted mean energy with respect to every vertex position.
fn gradient(rest: &[Rest], faces: Faces<'_>, uv: &[DVec2]) -> Vec<DVec2> {
    let total: f64 = rest.iter().map(Rest::area).sum();
    let mut g = vec![DVec2::ZERO; uv.len()];
    for (r, (_, t)) in rest.iter().zip(faces) {
        let dj = face_gradient(r.jacobian(uv, t));
        let op = r.operator();
        let w = r.area() / total;
        for (corner, &v) in t.iter().enumerate() {
            let c = op[corner];
            let along = |k: usize| dj[k][0] * c[0] + dj[k][1] * c[1];
            g[v.index() as usize] += DVec2::new(along(0), along(1)) * w;
        }
    }
    g
}

/// A fixed-seed direction in [−1, 1)² on every interior vertex, zero on the rim.
fn direction(n: usize) -> Vec<DVec2> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    };
    (0..n)
        .map(|v| {
            let p = DVec2::new(next(), next());
            if v < RIM { p } else { DVec2::ZERO }
        })
        .collect()
}

/// `log₂(r(h) / r(h/2))` for h = 2⁻⁴ … 2⁻¹⁴, kept only where r(h/2) is far above rounding.
fn rates(solve: &Solve, dir: &[DVec2], slope: f64) -> Result<Vec<f64>, FlattenError> {
    let (rest, faces, uv, e0) = (&solve.rest, &solve.faces[..], &solve.uv, solve.energy);
    let floor = 1e4 * f64::EPSILON * e0;
    let remainder = |h: f64| -> Result<f64, FlattenError> {
        let trial: Vec<DVec2> = uv.iter().zip(dir).map(|(u, d)| *u + *d * h).collect();
        let e = energy(rest, faces, &trial).ok_or(FlattenError::SolveFailed { chart: CHART })?;
        Ok((e - e0 - h * slope).abs())
    };
    let r: Vec<f64> = (4..=15)
        .map(|k| remainder(libm::ldexp(1.0, -k)))
        .collect::<Result<_, _>>()?;
    Ok(r.windows(2)
        .filter(|w| w[1] > floor)
        .map(|w| (w[0] / w[1]).log2())
        .collect())
}

#[test]
fn the_energys_remainder_shrinks_quadratically_only_with_its_gradient() -> Result<(), FlattenError>
{
    let (xyz, f) = hemisphere();
    let solve = Solve::start(CHART, &xyz, f)?;
    let faces = &solve.faces[..];
    assert!(solve.energy > 4.5, "the start must not be isometric");
    let raw = direction(xyz.len());
    // Scaled so the longest flip-free step is 2⁸, far beyond the largest h of 2⁻⁴, and so short
    // that the first-order term dominates the zero-gradient control's remainder at every h.
    let reach = max_step(faces, &solve.uv, &raw);
    let dir: Vec<DVec2> = raw.iter().map(|d| *d * libm::ldexp(reach, -8)).collect();
    assert!(max_step(faces, &solve.uv, &dir) > 128.0);
    let g = gradient(&solve.rest, faces, &solve.uv);
    let slope: f64 = g.iter().zip(&dir).map(|(a, b)| a.dot(*b)).sum();
    assert!(
        slope.abs() > 1e-6,
        "the direction must not be level: {slope}"
    );
    for (with, want) in [(slope, 2.0), (0.0, 1.0)] {
        let observed = rates(&solve, &dir, with)?;
        assert!(
            observed.len() >= 4,
            "too few rates above rounding: {observed:?}"
        );
        for rate in &observed {
            assert!(
                (rate - want).abs() <= 0.1,
                "slope {with}: rates {observed:?}"
            );
        }
    }
    Ok(())
}
