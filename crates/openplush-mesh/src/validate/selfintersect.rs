use openplush_geom::epsilon::{ModelTolerances, WELD_RELATIVE};
use openplush_geom::{DVec3, Sign};

use crate::FaceId;

/// Pairs of kept faces sharing no edge that cross along a segment longer than the
/// weld distance; faces sharing one vertex count, so a fold is caught. Faces that
/// touch, share an edge, lie within the weld distance of one plane, or have no area
/// are not reported. The test is in floating point against the weld distance, which
/// is enough for a report that only warns; exact predicates would take their slow
/// path on every pair of coplanar faces. Candidates come from a bounding-box tree;
/// faces around a vertex of high valence all overlap there, so that vertex costs its
/// valence squared.
pub(super) fn crossings(
    tris: &[[u32; 3]],
    kept: &[u32],
    positions: &[DVec3],
    tol: &ModelTolerances,
) -> Vec<[FaceId; 2]> {
    let Some(&first) = kept.first() else {
        return Vec::new();
    };
    // INVARIANT: kept faces index `tris`, and their vertices index `positions`.
    let origin = positions[tris[first as usize][0] as usize];
    // Every used vertex is within the diagonal of `origin`, so these stay near 1.
    let local = |v: u32| (positions[v as usize] - origin) * (1.0 / tol.diagonal());
    let planes: Vec<Option<Plane>> = kept
        .iter()
        .map(|&f| Plane::through(tris[f as usize].map(local)))
        .collect();
    let boxes: Vec<Aabb> = kept
        .iter()
        .map(|&f| Aabb::of(&tris[f as usize].map(local)))
        .collect();
    let tree = Tree::build(&boxes);
    let mut out = Vec::new();
    let mut candidates = Vec::new();
    for (i, &f) in kept.iter().enumerate() {
        tree.overlapping(&boxes, i, &mut candidates);
        for &j in &candidates {
            let g = kept[j];
            let shared = tris[f as usize]
                .iter()
                .filter(|v| tris[g as usize].contains(v))
                .count();
            if let (true, Some(a), Some(b)) = (shared < 2, &planes[i], &planes[j])
                && cross(a, b)
            {
                out.push([FaceId::new(f), FaceId::new(g)]);
            }
        }
    }
    out.sort();
    out
}

/// A face with area: its corners and its unit normal.
#[derive(Debug, Clone, Copy)]
struct Plane {
    corners: [DVec3; 3],
    unit: DVec3,
}

impl Plane {
    fn through(corners: [DVec3; 3]) -> Option<Self> {
        let [a, b, c] = corners;
        let unit = (b - a).cross(c - a).normalize()?;
        Some(Self { corners, unit })
    }

    /// Signed distance from the plane, in diagonals.
    fn distance(&self, x: DVec3) -> f64 {
        self.unit.dot(x - self.corners[0])
    }

    /// Which side each corner of `t` lies on, within the weld distance counting as on.
    fn sides(&self, t: &Self) -> [Sign; 3] {
        t.corners.map(|x| {
            let d = self.distance(x);
            if d > WELD_RELATIVE {
                Sign::Positive
            } else if d < -WELD_RELATIVE {
                Sign::Negative
            } else {
                Sign::Zero
            }
        })
    }
}

/// The faces' planes meet in a line, and the two faces overlap along it for more
/// than the weld distance.
fn cross(a: &Plane, b: &Plane) -> bool {
    let (sa, sb) = (b.sides(a), a.sides(b));
    if apart(sa) || apart(sb) {
        return false;
    }
    let line = a.unit.cross(b.unit);
    let Some(line) = line.normalize() else {
        return false;
    };
    let (ia, ib) = (span(a, sa, b, line), span(b, sb, a, line));
    ia.1.min(ib.1) - ia.0.max(ib.0) > WELD_RELATIVE
}

/// All corners strictly on one side, or all in the plane: the faces cannot cross.
fn apart(s: [Sign; 3]) -> bool {
    s.iter().all(|&x| x == Sign::Positive)
        || s.iter().all(|&x| x == Sign::Negative)
        || s.iter().all(|&x| x == Sign::Zero)
}

/// Where face `t` (corner sides `s`) meets plane `other`, as an interval along the
/// unit direction `line`: corners on the plane, and edges whose ends lie on opposite
/// sides, cut where the plane crosses them. `s` is not all one sign (`apart`), so the
/// interval has at least one point.
fn span(t: &Plane, s: [Sign; 3], other: &Plane, line: DVec3) -> (f64, f64) {
    let distance = t.corners.map(|p| other.distance(p));
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut take = |p: DVec3| {
        let x = line.dot(p);
        lo = lo.min(x);
        hi = hi.max(x);
    };
    for i in 0..3 {
        let j = (i + 1) % 3;
        if s[i] == Sign::Zero {
            take(t.corners[i]);
        }
        if s[i] != Sign::Zero && s[j] != Sign::Zero && s[i] != s[j] {
            let (di, dj) = (distance[i], distance[j]);
            take(t.corners[i] + (t.corners[j] - t.corners[i]) * (di / (di - dj)));
        }
    }
    (lo, hi)
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

    /// Fills `out` with the indices `j > i` whose boxes overlap box `i`, in no
    /// particular order.
    fn overlapping(&self, boxes: &[Aabb], i: usize, out: &mut Vec<usize>) {
        out.clear();
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
    }
}
