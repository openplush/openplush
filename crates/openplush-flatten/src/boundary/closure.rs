//! Closing the boundary polygon by changing only its turning angles, never its lengths.

use std::f64::consts::TAU;

use openplush_geom::DVec2;

use super::BoundaryError;

/// Each step solves the closure conditions linearised at the last angles, so convergence is
/// quadratic near a closed polygon; the limit only stops a system with no nearby solution.
const MAX_STEPS: usize = 100;
/// A step that does not shrink the closure error is halved up to this many times, down to
/// about one part in a billion of the full step, before the solve gives up.
const MAX_HALVINGS: i32 = 30;
/// Allowed closure error per edge, in ulps of a unit length: each sum adds one rounding per edge.
const ULPS_PER_EDGE: f64 = 64.0;

/// Turning angles at which edges of `lengths`, turning counter-clockwise by them, close up:
/// a stationary point of the squared change from `target`, the nearest one when `target` nearly
/// closes. Angle `i` is the turn at the start of edge `i`.
pub(super) fn close(lengths: &[f64], target: &[f64]) -> Result<Vec<f64>, BoundaryError> {
    let n = lengths.len();
    let valid = n >= 3
        && target.len() == n
        && lengths.iter().all(|l| l.is_finite() && *l > 0.0)
        && target.iter().all(|k| k.is_finite());
    if !valid {
        return Err(BoundaryError::BadInput);
    }
    let perimeter: f64 = lengths.iter().sum();
    let unit: Vec<f64> = lengths.iter().map(|l| l / perimeter).collect();
    let tolerance = n as f64 * ULPS_PER_EDGE * f64::EPSILON;
    let mut kappa = target.to_vec();
    for _ in 0..MAX_STEPS {
        let (residual, jacobian) = conditions(&unit, &kappa);
        if residual.iter().all(|r| r.abs() <= tolerance) {
            return Ok(kappa);
        }
        let full = step(target, &kappa, residual, &jacobian).ok_or(BoundaryError::NotClosed)?;
        kappa = damped(&unit, &kappa, &full, norm(residual)).ok_or(BoundaryError::NotClosed)?;
    }
    Err(BoundaryError::NotClosed)
}

/// The polygon's corners, from the origin with edge 0 along +x; edge `i` runs from corner `i`.
pub(super) fn walk(lengths: &[f64], kappa: &[f64]) -> Vec<DVec2> {
    let mut corner = DVec2::ZERO;
    headings(kappa)
        .zip(lengths)
        .map(|(phi, &l)| {
            let start = corner;
            corner += direction(phi) * l;
            start
        })
        .collect()
}

/// Edge `i`'s heading: the sum of the turns at corners 1 to `i`. Corner 0's turn only closes the loop.
fn headings(kappa: &[f64]) -> impl Iterator<Item = f64> + '_ {
    kappa.iter().enumerate().scan(0.0, |phi, (i, &k)| {
        if i > 0 {
            *phi += k;
        }
        Some(*phi)
    })
}

fn direction(phi: f64) -> DVec2 {
    // libm, not std: std's sin and cos are the platform's and differ between native and wasm32.
    DVec2::new(libm::cos(phi), libm::sin(phi))
}

/// The closure conditions (total turn minus 2π, then the end point's x and y on unit
/// perimeter) and their Jacobian rows with respect to each turning angle.
fn conditions(unit: &[f64], kappa: &[f64]) -> ([f64; 3], [Vec<f64>; 3]) {
    let steps: Vec<DVec2> = headings(kappa)
        .zip(unit)
        .map(|(phi, &l)| direction(phi) * l)
        .collect();
    let end = steps.iter().fold(DVec2::ZERO, |sum, &s| sum + s);
    // A turn at corner j > 0 swings every edge from j on: d(x, y)/d(kappa_j) = sum over i >= j of (-y_i, x_i).
    let mut tail = DVec2::ZERO;
    let mut dx = vec![0.0; unit.len()];
    let mut dy = vec![0.0; unit.len()];
    for j in (1..unit.len()).rev() {
        tail += steps[j];
        dx[j] = -tail.y;
        dy[j] = tail.x;
    }
    let total = kappa.iter().sum::<f64>() - TAU;
    ([total, end.x, end.y], [vec![1.0; unit.len()], dx, dy])
}

/// The first of the full step, half of it, a quarter, … that lowers the closure error.
fn damped(unit: &[f64], kappa: &[f64], full: &[f64], error: f64) -> Option<Vec<f64>> {
    (0..MAX_HALVINGS).find_map(|h| {
        let a = libm::ldexp(1.0, -h);
        let tried: Vec<f64> = kappa
            .iter()
            .zip(full)
            .map(|(k, f)| k + a * (f - k))
            .collect();
        (norm(conditions(unit, &tried).0) < error).then_some(tried)
    })
}

fn norm(r: [f64; 3]) -> f64 {
    r.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// The angles nearest `target` that satisfy the conditions linearised at `kappa`:
/// `target + Jᵀ μ` with `(J Jᵀ) μ = J (kappa - target) - residual`. `None` when `J Jᵀ` is singular.
fn step(
    target: &[f64],
    kappa: &[f64],
    residual: [f64; 3],
    jacobian: &[Vec<f64>; 3],
) -> Option<Vec<f64>> {
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let offset: Vec<f64> = kappa.iter().zip(target).map(|(k, t)| k - t).collect();
    let gram: [[f64; 3]; 3] =
        std::array::from_fn(|r| std::array::from_fn(|c| dot(&jacobian[r], &jacobian[c])));
    let rhs: [f64; 3] = std::array::from_fn(|r| dot(&jacobian[r], &offset) - residual[r]);
    let mu = solve3(gram, rhs)?;
    Some(
        target
            .iter()
            .enumerate()
            .map(|(i, t)| t + (0..3).map(|r| jacobian[r][i] * mu[r]).sum::<f64>())
            .collect(),
    )
}

/// Cramer's rule on a 3 by 3 system; `None` when it is singular or overflows.
fn solve3(m: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(m);
    let x: [f64; 3] = std::array::from_fn(|c| {
        let mut mc = m;
        (0..3).for_each(|r| mc[r][c] = b[r]);
        det(mc) / d
    });
    x.iter().all(|v| v.is_finite()).then_some(x)
}

#[cfg(test)]
mod tests;
