//! The symmetric Dirichlet energy of a flat layout against each face's 3D rest shape, and the
//! longest step along a direction that flips no face (Smith & Schaefer 2015).

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};

/// A face's 3D shape in its own plane: the inverse of the 2×2 matrix whose columns are its two
/// edges from corner 0 (the first along +x), and its area.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Rest {
    inv: [[f64; 2]; 2],
    area: f64,
}

impl Rest {
    pub(crate) fn area(&self) -> f64 {
        self.area
    }

    /// The Jacobian of the map taking this face's rest shape to the layout.
    pub(crate) fn jacobian(&self, uv: &[DVec2], t: &[VertexId; 3]) -> [[f64; 2]; 2] {
        let (d1, d2) = edges(uv, t);
        [
            [
                d1.x * self.inv[0][0] + d2.x * self.inv[1][0],
                d1.x * self.inv[0][1] + d2.x * self.inv[1][1],
            ],
            [
                d1.y * self.inv[0][0] + d2.y * self.inv[1][0],
                d1.y * self.inv[0][1] + d2.y * self.inv[1][1],
            ],
        ]
    }

    /// Row `v` holds the coefficients of corner `v`'s position in the Jacobian's two columns,
    /// so `J[k][m] = Σ_v operator[v][m] · uv_v[k]`.
    pub(super) fn operator(&self) -> [[f64; 2]; 3] {
        let [a, b] = self.inv;
        [[-(a[0] + b[0]), -(a[1] + b[1])], a, b]
    }
}

/// Each face's rest shape, or the first face naming a missing vertex or with zero or
/// non-finite area.
pub(crate) fn rest_shapes(
    xyz: &[DVec3],
    faces: &[(FaceId, [VertexId; 3])],
) -> Result<Vec<Rest>, FaceId> {
    faces
        .iter()
        .map(|&(face, t)| {
            let [Some(a), Some(b), Some(c)] = t.map(|v| xyz.get(v.index() as usize).copied())
            else {
                return Err(face);
            };
            let (e1, e2) = (b - a, c - a);
            let length = e1.length();
            let twice_area = e1.cross(e2).length();
            let (x, y) = (e1.dot(e2) / length, twice_area / length);
            let inv = [[1.0 / length, -x / (length * y)], [0.0, 1.0 / y]];
            // Zero area makes the inverse infinite; an overflowing area leaves it finite.
            let measurable = twice_area.is_finite() && inv.iter().flatten().all(|v| v.is_finite());
            measurable
                .then_some(Rest {
                    inv,
                    area: 0.5 * twice_area,
                })
                .ok_or(face)
        })
        .collect()
}

/// The area-weighted mean over faces of `|J|² + |J⁻¹|²`, which is 4 exactly when every face
/// keeps its shape; `None` when any face is flipped or collapsed (det J ≤ 0).
pub(super) fn energy(
    rest: &[Rest],
    faces: &[(FaceId, [VertexId; 3])],
    uv: &[DVec2],
) -> Option<f64> {
    let mut total = 0.0;
    let mut area = 0.0;
    for (r, (_, t)) in rest.iter().zip(faces) {
        let j = r.jacobian(uv, t);
        let det = j[0][0] * j[1][1] - j[0][1] * j[1][0];
        if det <= 0.0 {
            return None;
        }
        let frobenius: f64 = j.iter().flatten().map(|v| v * v).sum();
        // |J⁻¹|² = |J|² / det² for a 2×2 matrix.
        total += r.area * (frobenius + frobenius / (det * det));
        area += r.area;
    }
    let mean = total / area;
    mean.is_finite().then_some(mean)
}

/// The largest `t` such that `uv + s · dir` flips no face for any `s` in `[0, t)`; infinite
/// when no face ever collapses, zero when a coordinate is not finite.
pub(super) fn max_step(faces: &[(FaceId, [VertexId; 3])], uv: &[DVec2], dir: &[DVec2]) -> f64 {
    faces
        .iter()
        .map(|(_, t)| {
            let (d1, d2) = edges(uv, t);
            let (e1, e2) = edges(dir, t);
            // det(t) = c + b t + a t², the signed doubled area along the step.
            let c = d1.perp_dot(d2);
            let b = d1.perp_dot(e2) + e1.perp_dot(d2);
            let a = e1.perp_dot(e2);
            first_positive_root(a, b, c)
        })
        .fold(f64::INFINITY, f64::min)
}

/// The edges from corner 0 to corners 1 and 2.
fn edges(points: &[DVec2], t: &[VertexId; 3]) -> (DVec2, DVec2) {
    // INVARIANT: callers pass the faces `rest_shapes` accepted, whose vertices all have
    // positions, and layouts with one point per position.
    let at = |k: usize| points[t[k].index() as usize];
    (at(1) - at(0), at(2) - at(0))
}

/// The smallest root above zero of `a t² + b t + c`; infinite when there is none.
fn first_positive_root(a: f64, b: f64, c: f64) -> f64 {
    if !(a.is_finite() && b.is_finite() && c.is_finite()) {
        return 0.0;
    }
    // A negative discriminant (no real root) gives NaN roots, which the filter drops.
    let disc = b * b - 4.0 * a * c;
    // Roots q / a and c / q with q = -(b + sign(b) √disc) / 2: no cancellation when one root
    // is tiny, and c / q is the linear root as a → 0.
    let q = -0.5 * (b + disc.sqrt().copysign(b));
    [q / a, c / q]
        .into_iter()
        .filter(|r| *r > 0.0)
        .fold(f64::INFINITY, f64::min)
}

#[cfg(test)]
mod tests;
