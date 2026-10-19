//! SLIM's per-face reweighting: the rotation each face is pulled toward and the weights that
//! make the weighted least-squares gradient equal the symmetric Dirichlet gradient
//! (Rabinovich et al. 2017).

use openplush_geom::DVec2;

type Mat = [[f64; 2]; 2];

/// What one face contributes to the weighted proxy `‖W (J − R)‖²`.
pub(super) struct Proxy {
    /// The rotation closest to J.
    pub(super) rotation: Mat,
    /// `WᵀW = U diag(w₁, w₂) Uᵀ` for J = U Σ Vᵀ.
    pub(super) weight: Mat,
}

/// The proxy for a Jacobian with positive determinant.
pub(super) fn proxy(j: Mat) -> Proxy {
    // J = Rot(φ) diag(s₁, s₂) Rot(θ), with φ + θ the angle of the closest rotation.
    let [[a, b], [c, d]] = j;
    let (e, f, g, h) = ((a + d) / 2.0, (a - d) / 2.0, (c + b) / 2.0, (c - b) / 2.0);
    let (q, r) = (libm::hypot(e, h), libm::hypot(f, g));
    let (a1, a2) = (libm::atan2(g, f), libm::atan2(h, e));
    let phi = (a2 + a1) / 2.0;
    let (sin_phi, cos_phi) = libm::sincos(phi);
    let (sin_r, cos_r) = libm::sincos(a2);
    let (w1, w2) = (reweight(q + r), reweight(q - r));
    let (u1, u2) = (DVec2::new(cos_phi, sin_phi), DVec2::new(-sin_phi, cos_phi));
    let weight = [
        [
            w1 * u1.x * u1.x + w2 * u2.x * u2.x,
            w1 * u1.x * u1.y + w2 * u2.x * u2.y,
        ],
        [
            w1 * u1.x * u1.y + w2 * u2.x * u2.y,
            w1 * u1.y * u1.y + w2 * u2.y * u2.y,
        ],
    ];
    Proxy {
        rotation: [[cos_r, -sin_r], [sin_r, cos_r]],
        weight,
    }
}

/// `(s − s⁻³) / (s − 1)`, the factor by which the proxy's gradient `2 w (s − 1)` matches the
/// energy's `2 (s − s⁻³)`; 4 at `s = 1`.
fn reweight(s: f64) -> f64 {
    (s + 1.0) * (s * s + 1.0) / (s * s * s)
}

#[cfg(test)]
mod tests;
