//! Which faces on an over-shared edge share a copy of it.

use std::collections::BTreeSet;

use openplush_geom::DVec3;

use crate::validate::EdgeUse;
use crate::{FaceId, MeshError, VertexId};

/// One turn in [`pseudo_angle`] units.
const PERIOD: f64 = 4.0;

/// A strictly increasing function of the counter-clockwise angle of `(x, y)` from
/// the +x axis, in `[0, 4)`, using only `+ - /` so it is identical on every target
/// (`atan2` is not).
fn pseudo_angle(x: f64, y: f64) -> f64 {
    let p = if y >= 0.0 {
        if x >= 0.0 {
            y / (x + y)
        } else {
            1.0 - x / (y - x)
        }
    } else if x < 0.0 {
        2.0 - y / (-x - y)
    } else {
        3.0 + x / (x - y)
    };
    p.rem_euclid(PERIOD)
}

/// A face on the edge, placed by its angle round the edge's axis, measured from the lowest face.
#[derive(Debug, Clone, Copy)]
struct Member {
    face: u32,
    forward: bool,
    angle: f64,
}

/// How the faces on one edge are paired; each pair has the lower face first, sorted.
pub(super) struct Pairing {
    pub(super) pairs: Vec<[u32; 2]>,
    pub(super) unpaired: Option<u32>,
}

impl Pairing {
    fn none() -> Self {
        Self {
            pairs: Vec::new(),
            unpaired: None,
        }
    }
}

/// Fewer sheet layers paired together, then fewer pairs walking the edge the same
/// way, then less angle between the pairs, is better.
#[derive(Debug, Clone, Copy)]
struct Score {
    layers: usize,
    same_way: usize,
    gap: f64,
}

impl Score {
    fn beats(&self, other: &Score) -> bool {
        self.layers
            .cmp(&other.layers)
            .then(self.same_way.cmp(&other.same_way))
            .then(self.gap.total_cmp(&other.gap))
            .is_lt()
    }
}

/// Pairs the faces on `edge` that are neighbours round its axis. With an even
/// count the ring splits into pairs in one of two ways; with an odd count one face
/// is left out first. Layers of a sheet, `sheets`, are never paired when another
/// choice exists. Requires faces of nonzero area; one with its third vertex on
/// the edge's axis is refused.
pub(super) fn pair(
    positions: &[DVec3],
    tris: &[[u32; 3]],
    edge: [u32; 2],
    uses: &[EdgeUse],
    sheets: &BTreeSet<[u32; 2]>,
) -> Result<Pairing, MeshError> {
    let ring = ring(positions, tris, edge, uses)?;
    let n = ring.len();
    let starts = if n % 2 == 0 { 0..2 } else { 0..n };
    let mut best: Option<(Score, Pairing)> = None;
    for start in starts {
        let (left_out, first) = if n % 2 == 0 {
            (None, start)
        } else {
            (Some(ring[start].face), start + 1)
        };
        let pairs: Vec<[Member; 2]> = (0..n / 2)
            .map(|k| [ring[(first + 2 * k) % n], ring[(first + 2 * k + 1) % n]])
            .collect();
        let score = score(&pairs, sheets);
        if best.as_ref().is_none_or(|(b, _)| score.beats(b)) {
            let mut faces: Vec<[u32; 2]> = pairs
                .iter()
                .map(|p| ordered(p[0].face, p[1].face))
                .collect();
            faces.sort();
            best = Some((
                score,
                Pairing {
                    pairs: faces,
                    unpaired: left_out,
                },
            ));
        }
    }
    Ok(best.map_or_else(Pairing::none, |(_, pairing)| pairing))
}

fn score(pairs: &[[Member; 2]], sheets: &BTreeSet<[u32; 2]>) -> Score {
    let mut score = Score {
        layers: 0,
        same_way: 0,
        gap: 0.0,
    };
    for [x, y] in pairs {
        score.layers += usize::from(sheets.contains(&ordered(x.face, y.face)));
        score.same_way += usize::from(x.forward == y.forward);
        score.gap += (y.angle - x.angle).rem_euclid(PERIOD);
    }
    score
}

fn ordered(a: u32, b: u32) -> [u32; 2] {
    [a.min(b), a.max(b)]
}

/// The faces on `edge` in angle order round its axis from the lowest face. At one
/// angle the faces walking the edge forward come first, so coincident layers of a
/// sheet alternate direction round the ring.
fn ring(
    positions: &[DVec3],
    tris: &[[u32; 3]],
    [a, b]: [u32; 2],
    uses: &[EdgeUse],
) -> Result<Vec<Member>, MeshError> {
    // INVARIANT: soup indices are in range (`TriangleSoup::new`).
    let origin = positions[a as usize];
    let axis = (positions[b as usize] - origin)
        .normalize()
        .ok_or(MeshError::NonManifoldEdge {
            a: VertexId::new(a),
            b: VertexId::new(b),
        })?;
    let mut offsets = Vec::with_capacity(uses.len());
    for u in uses {
        let tri = tris[u.face as usize];
        let c =
            tri.into_iter()
                .find(|&v| v != a && v != b)
                .ok_or(MeshError::DegenerateTriangle {
                    triangle: FaceId::new(u.face),
                })?;
        let w = positions[c as usize] - origin;
        let w = w - axis * w.dot(axis);
        if !(w.length() > 0.0 && w.length().is_finite()) {
            return Err(MeshError::ZeroAreaTriangle {
                triangle: FaceId::new(u.face),
            });
        }
        offsets.push(w);
    }
    let Some(lowest) = (0..uses.len()).min_by_key(|&i| uses[i].face) else {
        return Ok(Vec::new());
    };
    let e1 = offsets[lowest]
        .normalize()
        .ok_or(MeshError::ZeroAreaTriangle {
            triangle: FaceId::new(uses[lowest].face),
        })?;
    let e2 = axis.cross(e1);
    let mut ring: Vec<Member> = uses
        .iter()
        .zip(&offsets)
        .map(|(u, &w)| Member {
            face: u.face,
            forward: u.forward,
            angle: pseudo_angle(w.dot(e1), w.dot(e2)),
        })
        .collect();
    ring.sort_by(|x, y| {
        x.angle
            .total_cmp(&y.angle)
            .then(y.forward.cmp(&x.forward))
            .then(x.face.cmp(&y.face))
    });
    Ok(ring)
}
