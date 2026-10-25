//! Whether two pieces' sewing lines are one shape: how one lies on the other, as printed or
//! flipped, after the best turn and shift.

use openplush_geom::DVec2;

use crate::Outline;

/// Pieces further apart than the seam-parity floor and within this are a near match, never
/// merged: the maker's judgement of how far apart two mirror pieces may be and still be meant
/// as one.
pub const NEAR_MM: f64 = 5.0;

/// Which way one piece lies on another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    /// Turned and moved only.
    AsPrinted,
    /// Turned over first: a mirror image.
    Flipped,
}

/// How outline `b` lies on outline `a`: which of `b`'s corners each of `a`'s lands on, and the
/// largest distance between matched corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    facing: Facing,
    offset: usize,
    corners: usize,
    gap_mm: f64,
}

impl Fit {
    /// Which way `b` lies on `a`.
    pub fn facing(&self) -> Facing {
        self.facing
    }

    /// The largest distance, in mm, between a corner of `a` and its match on `b`.
    pub fn gap_mm(&self) -> f64 {
        self.gap_mm
    }

    /// The corner of `b` that corner `i` of `a` lands on.
    pub fn corner(&self, i: usize) -> usize {
        let n = self.corners;
        match self.facing {
            Facing::AsPrinted => (i + self.offset) % n,
            Facing::Flipped => (self.offset + n - i % n) % n,
        }
    }

    /// The edge of `b` that edge `i` of `a`, from corner `i` to the next, lands on.
    pub fn edge(&self, i: usize) -> usize {
        let n = self.corners;
        match self.facing {
            Facing::AsPrinted => self.corner(i),
            // Flipped, corner i + 1 lands one corner back, so the edge starts there.
            Facing::Flipped => self.corner((i + 1) % n),
        }
    }
}

/// The best way `b` lies on `a` facing `facing`: over every pairing of corners in order, the turn
/// and shift that bring them closest in the least-squares sense, and of those the pairing whose
/// largest corner distance is least, the lowest offset on a tie. `None` when the corner counts
/// differ. Uses no trigonometry, so the result is the same on every target.
pub fn fit(a: &Outline, b: &Outline, facing: Facing) -> Option<Fit> {
    every(a, b, facing)
        .into_iter()
        .fold(None, |best: Option<Fit>, f| match best {
            Some(kept) if kept.gap_mm <= f.gap_mm => Some(kept),
            _ => Some(f),
        })
}

/// Every way `b` lies on `a` facing `facing` with its largest corner distance at most
/// `within_mm`, lowest offset first: a symmetric shape lies on another in several, and which one
/// matters once its edges carry different seams. Empty when the corner counts differ.
pub fn fits(a: &Outline, b: &Outline, facing: Facing, within_mm: f64) -> Vec<Fit> {
    every(a, b, facing)
        .into_iter()
        .filter(|f| f.gap_mm <= within_mm)
        .collect()
}

/// `b` on `a` at every offset, each measured as `fit` measures it.
fn every(a: &Outline, b: &Outline, facing: Facing) -> Vec<Fit> {
    let (pa, pb) = (a.points(), b.points());
    let n = pa.len();
    if pb.len() != n {
        return Vec::new();
    }
    let ours: Vec<DVec2> = centred(pa);
    let theirs: Vec<DVec2> = match facing {
        Facing::AsPrinted => centred(pb),
        Facing::Flipped => centred(pb)
            .into_iter()
            .map(|p| DVec2::new(-p.x, p.y))
            .collect(),
    };
    (0..n)
        .map(|offset| {
            let candidate = Fit {
                facing,
                offset,
                corners: n,
                gap_mm: 0.0,
            };
            let matched: Vec<DVec2> = (0..n).map(|i| theirs[candidate.corner(i)]).collect();
            Fit {
                gap_mm: gap(&ours, &matched),
                ..candidate
            }
        })
        .collect()
}

/// `points` moved so their mean is the origin.
fn centred(points: &[DVec2]) -> Vec<DVec2> {
    // Lossless: an outline has far fewer than 2^52 corners.
    #[allow(clippy::cast_precision_loss)]
    let n = points.len() as f64;
    let sum = points.iter().fold(DVec2::new(0.0, 0.0), |s, &p| s + p);
    let mean = sum / n;
    points.iter().map(|&p| p - mean).collect()
}

/// The largest distance between `ours[i]`, turned by the least-squares best turn onto `theirs`,
/// and `theirs[i]`; both are centred.
fn gap(ours: &[DVec2], theirs: &[DVec2]) -> f64 {
    let (dot, cross) = ours.iter().zip(theirs).fold((0.0, 0.0), |(d, c), (p, q)| {
        (d + p.dot(*q), c + p.perp_dot(*q))
    });
    let h = (dot * dot + cross * cross).sqrt();
    // A degenerate pair has no best turn; the identity is as good as any.
    let (cos, sin) = if h > 0.0 {
        (dot / h, cross / h)
    } else {
        (1.0, 0.0)
    };
    ours.iter()
        .zip(theirs)
        .map(|(p, q)| {
            let turned = DVec2::new(cos * p.x - sin * p.y, sin * p.x + cos * p.y);
            (turned - *q).length()
        })
        .fold(0.0, f64::max)
}
