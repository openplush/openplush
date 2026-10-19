//! Seeded generator of simple CCW outlines with per-edge allowances (mm).

use super::geom::{P, is_simple_ccw};

/// SplitMix64: tiny, seedable, identical on every target.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1) with 53 random bits.
    pub(crate) fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub(crate) fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// The shape family a case is drawn from, chosen by its index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Family {
    Star,
    Monotone,
    ShortEdges,
    NearCollinear,
    Spike,
    Sliver,
}

pub(crate) struct Case {
    pub(crate) outline: Vec<P>,
    /// `allowance[i]` belongs to the edge from `outline[i]` to `outline[i + 1]`.
    pub(crate) allowance: Vec<f64>,
}

const FAMILIES: [Family; 6] = [
    Family::Star,
    Family::Monotone,
    Family::ShortEdges,
    Family::NearCollinear,
    Family::Spike,
    Family::Sliver,
];

/// Case `index` of the corpus for `seed`; independent of every other index.
pub(crate) fn case(seed: u64, index: u64) -> Case {
    let mut rng = Rng::new(seed ^ index.wrapping_mul(0xD1B5_4A32_D192_ED03));
    let family = FAMILIES[(index % FAMILIES.len() as u64) as usize];
    loop {
        let unit = match family {
            Family::Star => star(&mut rng, 3, 61),
            Family::Monotone => monotone(&mut rng),
            Family::ShortEdges => short_edges(&mut rng),
            Family::NearCollinear => near_collinear(&mut rng),
            Family::Spike => spike(&mut rng),
            Family::Sliver => sliver(&mut rng),
        };
        // One in ten tiny, one in ten huge, the rest plush-sized.
        let size = match rng.below(10) {
            0 => rng.range(0.01, 1.0),
            1 => rng.range(1e4, 1e6),
            _ => rng.range(10.0, 1000.0),
        };
        let shift = if rng.below(4) == 0 {
            [rng.range(-1e5, 1e5), rng.range(-1e5, 1e5)]
        } else {
            [0.0, 0.0]
        };
        let outline: Vec<P> = unit
            .iter()
            .map(|p| [p[0] * size + shift[0], p[1] * size + shift[1]])
            .collect();
        if !is_simple_ccw(&outline) {
            continue;
        }
        let allowance = (0..outline.len()).map(|_| allowance(&mut rng)).collect();
        return Case { outline, allowance };
    }
}

/// Mostly the allowances a maker uses (0 = fold or edge-stitched), sometimes any value.
fn allowance(rng: &mut Rng) -> f64 {
    const COMMON: [f64; 5] = [0.0, 3.0, 6.0, 6.0, 10.0];
    if rng.below(5) == 0 {
        rng.range(0.0, 12.0)
    } else {
        COMMON[rng.below(COMMON.len())]
    }
}

/// Star-shaped about the origin: sorted angles, positive radii, so simple by construction.
fn star(rng: &mut Rng, min: usize, spread: usize) -> Vec<P> {
    let n = min + rng.below(spread);
    let mut angles: Vec<f64> = (0..n)
        .map(|_| rng.range(0.0, std::f64::consts::TAU))
        .collect();
    angles.sort_by(f64::total_cmp);
    let r_min = rng.range(0.05, 0.5);
    angles
        .iter()
        .map(|&a| polar(rng.range(r_min, 0.5), a))
        .collect()
}

/// x-monotone: random x, split into a lower and an upper chain.
fn monotone(rng: &mut Rng) -> Vec<P> {
    let n = 4 + rng.below(60);
    let mut xs: Vec<f64> = (0..n).map(|_| rng.unit()).collect();
    xs.sort_by(f64::total_cmp);
    let (mut lower, mut upper) = (vec![[xs[0], 0.5]], Vec::new());
    for &x in &xs[1..n - 1] {
        if rng.below(2) == 0 {
            lower.push([x, rng.range(0.0, 0.5)])
        } else {
            upper.push([x, rng.range(0.5, 1.0)])
        }
    }
    lower.push([xs[n - 1], 0.5]);
    lower.extend(upper.into_iter().rev());
    lower
}

/// A star with some vertices split into two at 1e-9 .. 1e-3 of the size apart.
fn short_edges(rng: &mut Rng) -> Vec<P> {
    let base = star(rng, 5, 30);
    let mut out = Vec::with_capacity(base.len() * 2);
    for p in base {
        out.push(p);
        if rng.below(3) == 0 {
            let d = libm::pow(10.0, rng.range(-9.0, -3.0));
            let a = rng.range(0.0, std::f64::consts::TAU);
            out.push([p[0] + d * libm::cos(a), p[1] + d * libm::sin(a)]);
        }
    }
    out
}

/// A star with edges subdivided and the new points pushed off the line by 1e-12 .. 1e-4.
fn near_collinear(rng: &mut Rng) -> Vec<P> {
    let base = star(rng, 4, 20);
    let mut out = Vec::with_capacity(base.len() * 3);
    for (i, &p) in base.iter().enumerate() {
        out.push(p);
        let q = base[(i + 1) % base.len()];
        let mut ts: Vec<f64> = (0..rng.below(3)).map(|_| rng.range(0.1, 0.9)).collect();
        ts.sort_by(f64::total_cmp);
        for t in ts {
            let e = libm::pow(10.0, rng.range(-12.0, -4.0))
                * if rng.below(2) == 0 { 1.0 } else { -1.0 };
            let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
            out.push([p[0] + t * dx - e * dy, p[1] + t * dy + e * dx]);
        }
    }
    out
}

/// A star with outward and inward needles whose tip angle is 0.05° .. 5°.
fn spike(rng: &mut Rng) -> Vec<P> {
    let n = 6 + rng.below(20);
    let mut angles: Vec<f64> = (0..n)
        .map(|k| std::f64::consts::TAU * k as f64 / n as f64)
        .collect();
    angles.iter_mut().for_each(|a| *a += rng.range(0.0, 0.2));
    let mut out = Vec::with_capacity(n * 3);
    for &a in &angles {
        out.push(polar(0.3, a));
        if rng.below(2) == 0 {
            let tip = rng.range(0.05, 5.0).to_radians();
            let inward = rng.below(3) == 0;
            let base_r = 0.3 * libm::cos(0.5 * std::f64::consts::TAU / n as f64);
            let len = if inward {
                rng.range(0.05, 0.2)
            } else {
                rng.range(0.1, 0.5)
            };
            let half = len * libm::tan(tip / 2.0);
            let r_tip = if inward { base_r - len } else { base_r + len };
            let r_base = base_r;
            let mid = a + 0.5 * std::f64::consts::TAU / n as f64;
            let w = half / r_base;
            out.extend([
                polar(r_base, mid - w),
                polar(r_tip, mid),
                polar(r_base, mid + w),
            ]);
        }
    }
    out
}

/// Large aspect ratio: a star squashed by 1e-1 .. 1e-4 in y.
fn sliver(rng: &mut Rng) -> Vec<P> {
    let k = libm::pow(10.0, -rng.range(1.0, 4.0));
    star(rng, 3, 30)
        .into_iter()
        .map(|p| [p[0], p[1] * k])
        .collect()
}

fn polar(r: f64, a: f64) -> P {
    [r * libm::cos(a), r * libm::sin(a)]
}
