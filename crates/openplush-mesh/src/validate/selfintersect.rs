use openplush_geom::predicates::orient3d;
use openplush_geom::{DVec3, Sign};

use crate::FaceId;

/// Pairs of kept faces that share no vertex and cross each other: an edge of one
/// passes strictly through the other. Faces that only touch, at a point or along a
/// line, or overlap in one plane, are not reported; this report only warns, and an
/// exact predicate decides every crossing it does report. Candidates come from a
/// bounding-box tree, so a model is not compared pair by pair.
pub(super) fn crossings(tris: &[[u32; 3]], kept: &[u32], positions: &[DVec3]) -> Vec<[FaceId; 2]> {
    // INVARIANT: kept faces index `tris`, and their vertices index `positions`.
    let corners = |f: u32| tris[f as usize].map(|v| positions[v as usize]);
    let boxes: Vec<Aabb> = kept.iter().map(|&f| Aabb::of(&corners(f))).collect();
    let tree = Tree::build(&boxes);
    let mut out = Vec::new();
    for (i, &f) in kept.iter().enumerate() {
        for j in tree.overlapping(&boxes, i) {
            let g = kept[j];
            let shares = tris[f as usize]
                .iter()
                .any(|v| tris[g as usize].contains(v));
            if !shares && cross(corners(f), corners(g)) {
                out.push([FaceId::new(f), FaceId::new(g)]);
            }
        }
    }
    out.sort();
    out
}

fn cross(a: [DVec3; 3], b: [DVec3; 3]) -> bool {
    edges(a).iter().any(|&(p, q)| pierces(p, q, b))
        || edges(b).iter().any(|&(p, q)| pierces(p, q, a))
}

fn edges([a, b, c]: [DVec3; 3]) -> [(DVec3, DVec3); 3] {
    [(a, b), (b, c), (c, a)]
}

/// Segment pq passes strictly through the interior of triangle abc: its ends lie
/// strictly on opposite sides of the plane, and it passes strictly inside all three
/// edges. A predicate that cannot decide counts as no crossing.
fn pierces(p: DVec3, q: DVec3, [a, b, c]: [DVec3; 3]) -> bool {
    let side = |d| orient3d(a, b, c, d).ok();
    let opposite = matches!(
        (side(p), side(q)),
        (Some(Sign::Positive), Some(Sign::Negative)) | (Some(Sign::Negative), Some(Sign::Positive))
    );
    if !opposite {
        return false;
    }
    let turn = |u, v| orient3d(p, q, u, v).ok();
    let signs = [turn(a, b), turn(b, c), turn(c, a)];
    signs.iter().all(|s| *s == Some(Sign::Positive))
        || signs.iter().all(|s| *s == Some(Sign::Negative))
}

#[derive(Debug, Clone, Copy)]
struct Aabb {
    lo: DVec3,
    hi: DVec3,
}

impl Aabb {
    fn of(points: &[DVec3]) -> Self {
        let mut out = Self {
            lo: DVec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            hi: DVec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        };
        for p in points {
            out.lo = DVec3::new(out.lo.x.min(p.x), out.lo.y.min(p.y), out.lo.z.min(p.z));
            out.hi = DVec3::new(out.hi.x.max(p.x), out.hi.y.max(p.y), out.hi.z.max(p.z));
        }
        out
    }

    fn union(self, o: Self) -> Self {
        Self::of(&[self.lo, self.hi, o.lo, o.hi])
    }

    fn overlaps(&self, o: &Self) -> bool {
        self.lo.x <= o.hi.x
            && o.lo.x <= self.hi.x
            && self.lo.y <= o.hi.y
            && o.lo.y <= self.hi.y
            && self.lo.z <= o.hi.z
            && o.lo.z <= self.hi.z
    }

    fn centre(&self, axis: usize) -> f64 {
        let (lo, hi) = match axis {
            0 => (self.lo.x, self.hi.x),
            1 => (self.lo.y, self.hi.y),
            _ => (self.lo.z, self.hi.z),
        };
        lo / 2.0 + hi / 2.0
    }

    fn longest_axis(&self) -> usize {
        let d = self.hi - self.lo;
        if d.x >= d.y && d.x >= d.z {
            0
        } else if d.y >= d.z {
            1
        } else {
            2
        }
    }
}

const LEAF: usize = 4;

/// A bounding-box tree over boxes, split at the median along the longest axis. Built
/// with stable sorts that break ties by index, so its shape is deterministic.
struct Tree {
    nodes: Vec<Node>,
    order: Vec<usize>,
}

struct Node {
    bounds: Aabb,
    kind: Kind,
}

/// A leaf holds the boxes `order[start..end]`; an inner node, two child nodes.
enum Kind {
    Leaf { start: usize, end: usize },
    Inner { left: usize, right: usize },
}

impl Tree {
    fn build(boxes: &[Aabb]) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            order: (0..boxes.len()).collect(),
        };
        if !boxes.is_empty() {
            tree.split(boxes, 0, boxes.len());
        }
        tree
    }

    /// Builds the node for `order[start..end]` and returns its index.
    fn split(&mut self, boxes: &[Aabb], start: usize, end: usize) -> usize {
        let members = &mut self.order[start..end];
        let bounds = members
            .iter()
            .map(|&i| boxes[i])
            .reduce(Aabb::union)
            .unwrap_or_else(|| Aabb::of(&[]));
        let at = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            kind: Kind::Leaf { start, end },
        });
        if end - start <= LEAF {
            return at;
        }
        let axis = bounds.longest_axis();
        members.sort_by(|&a, &b| {
            boxes[a]
                .centre(axis)
                .total_cmp(&boxes[b].centre(axis))
                .then(a.cmp(&b))
        });
        let mid = start + (end - start) / 2;
        let left = self.split(boxes, start, mid);
        let right = self.split(boxes, mid, end);
        self.nodes[at].kind = Kind::Inner { left, right };
        at
    }

    /// Indices `j > i` whose boxes overlap box `i`, in no particular order.
    fn overlapping(&self, boxes: &[Aabb], i: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = if self.nodes.is_empty() {
            vec![]
        } else {
            vec![0]
        };
        while let Some(n) = stack.pop() {
            let node = &self.nodes[n];
            if !node.bounds.overlaps(&boxes[i]) {
                continue;
            }
            match node.kind {
                Kind::Leaf { start, end } => out.extend(
                    self.order[start..end]
                        .iter()
                        .copied()
                        .filter(|&j| j > i && boxes[j].overlaps(&boxes[i])),
                ),
                Kind::Inner { left, right } => stack.extend([left, right]),
            }
        }
        out
    }
}
