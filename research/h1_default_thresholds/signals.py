"""The three hint signals of research R1 as reference code, so their default thresholds can be swept on the corpus.

Sharp edges and concave creases: the signed dihedral fold on each interior edge, chained once at the most permissive
threshold (data-model section 3: chain once, filter the chains) with continuations by smallest turning angle, then
filtered by the chain's length-weighted mean fold and its physical length. Dart candidates: the angle defect at each
vertex, integrated over a geodesic disk of physical radius (Dijkstra over edges), thresholded, clustered once at the
most permissive threshold, one candidate per cluster at its maximum. Nothing here ships; the Rust detectors (H028,
H029, H031) implement the same definitions.
"""

import heapq
import math
from dataclasses import dataclass
from pathlib import Path

Vec = tuple[float, float, float]
Face = tuple[int, int, int]
Edge = tuple[int, int]


def sub(a: Vec, b: Vec) -> Vec:
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def dot(a: Vec, b: Vec) -> float:
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(u: Vec, w: Vec) -> Vec:
    return (u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0])


def unit(v: Vec) -> Vec:
    n = math.sqrt(dot(v, v))
    return (v[0] / n, v[1] / n, v[2] / n)


def angle_deg(a: Vec, b: Vec) -> float:
    return math.degrees(math.acos(max(-1.0, min(1.0, dot(unit(a), unit(b))))))


def edges_of(f: Face) -> list[Edge]:
    return [(f[0], f[1]), (f[1], f[2]), (f[2], f[0])]


def key(a: int, b: int) -> Edge:
    return (a, b) if a < b else (b, a)


@dataclass(frozen=True)
class Mesh:
    pos: list[Vec]
    faces: list[Face]

    def normal(self, f: Face) -> Vec:
        a, b, c = (self.pos[v] for v in f)
        return unit(cross(sub(b, a), sub(c, a)))

    def centroid(self, f: Face) -> Vec:
        a, b, c = (self.pos[v] for v in f)
        return ((a[0] + b[0] + c[0]) / 3, (a[1] + b[1] + c[1]) / 3, (a[2] + b[2] + c[2]) / 3)

    def length(self, e: Edge) -> float:
        return math.dist(self.pos[e[0]], self.pos[e[1]])


def signed_folds(m: Mesh) -> dict[Edge, float]:
    """Fold angle in degrees per interior edge: positive where the surface folds outward (a ridge), negative where it
    folds inward (a valley). A boundary edge has no fold; an edge with more than two faces raises."""
    by_edge: dict[Edge, list[Face]] = {}
    for f in m.faces:
        for a, b in edges_of(f):
            by_edge.setdefault(key(a, b), []).append(f)
    out: dict[Edge, float] = {}
    for e, fs in by_edge.items():
        if len(fs) > 2:
            raise ValueError(f"edge {e} has {len(fs)} faces")
        if len(fs) < 2:
            continue
        n, o = m.normal(fs[0]), m.normal(fs[1])
        angle = angle_deg(n, o)
        concave = dot(n, sub(m.centroid(fs[1]), m.centroid(fs[0]))) > 0
        out[e] = -angle if concave else angle
    return out


@dataclass(frozen=True)
class Chain:
    edges: list[Edge]
    length_mm: float
    mean_fold: float  # length-weighted mean |fold|, degrees
    max_fold: float


def chains(m: Mesh, folds: dict[Edge, float], qualifying: list[Edge]) -> list[Chain]:
    """Maximal polylines through the qualifying edges. At a vertex with more than two of them, edges pair off by
    the smallest turning angle (ties by lowest edge key), so a junction never swallows a straight continuation."""
    at: dict[int, list[Edge]] = {}
    for e in sorted(qualifying):
        at.setdefault(e[0], []).append(e)
        at.setdefault(e[1], []).append(e)
    nxt: dict[tuple[Edge, int], Edge | None] = {}  # (edge, vertex it leaves through) -> the edge it continues into
    for v, es in at.items():
        free = list(es)
        while len(free) >= 2:
            best = min(
                ((turning(m, a, b, v), a, b) for i, a in enumerate(free) for b in free[i + 1 :]),
                key=lambda t: (t[0], t[1], t[2]),
            )
            _, a, b = best
            nxt[(a, v)], nxt[(b, v)] = b, a
            free.remove(a)
            free.remove(b)
        for e in free:
            nxt[(e, v)] = None
    seen: set[Edge] = set()
    out: list[Chain] = []
    for start in sorted(qualifying):
        if start in seen:
            continue
        path = walk(start, start[0], nxt, seen)
        tail = walk(start, start[1], nxt, seen)
        edges = list(reversed(tail[1:])) + path
        lengths = [m.length(e) for e in edges]
        total = sum(lengths)
        mean = sum(abs(folds[e]) * l for e, l in zip(edges, lengths)) / total
        out.append(Chain(edges, total, mean, max(abs(folds[e]) for e in edges)))
    return out


def turning(m: Mesh, a: Edge, b: Edge, v: int) -> float:
    """Degrees by which the path turns at v going from edge a to edge b; 0 is straight on."""
    pa = m.pos[a[0] if a[1] == v else a[1]]
    pb = m.pos[b[0] if b[1] == v else b[1]]
    return 180.0 - angle_deg(sub(pa, m.pos[v]), sub(pb, m.pos[v]))


def walk(start: Edge, through: int, nxt: dict[tuple[Edge, int], Edge | None], seen: set[Edge]) -> list[Edge]:
    """Follows continuations from `start` out through vertex `through` until the chain ends or closes."""
    path = [start]
    seen.add(start)
    e, v = start, through
    while True:
        n = nxt.get((e, v))
        if n is None or n in seen:
            return path
        seen.add(n)
        path.append(n)
        v = n[0] if n[1] == v else n[1]
        e = n


def angle_defects(m: Mesh) -> list[float]:
    """Discrete Gaussian curvature per vertex, in degrees: 360 minus the sum of the corner angles."""
    total = [0.0] * len(m.pos)
    for f in m.faces:
        for k in range(3):
            v, u, w = f[k], f[(k + 1) % 3], f[(k + 2) % 3]
            total[v] += angle_deg(sub(m.pos[u], m.pos[v]), sub(m.pos[w], m.pos[v]))
    return [360.0 - t for t in total]


def integrated_curvature(m: Mesh, radius_mm: float, absolute: bool = False) -> list[float]:
    """|Sum of the angle defects| over every vertex within geodesic distance `radius_mm` (Dijkstra over mesh edges),
    or the sum of |angle defect| when `absolute`. The signed sum cancels tessellation noise (alternating saddle and
    peak vertices on a grid mesh); the absolute sum is research R1's wording and measures that noise instead."""
    adjacency: dict[int, list[tuple[int, float]]] = {}
    for f in m.faces:
        for a, b in edges_of(f):
            d = m.length((a, b))
            adjacency.setdefault(a, []).append((b, d))
            adjacency.setdefault(b, []).append((a, d))
    defects = angle_defects(m)
    out = []
    for source in range(len(m.pos)):
        dist = {source: 0.0}
        heap = [(0.0, source)]
        total = 0.0
        while heap:
            d, v = heapq.heappop(heap)
            if d > dist[v]:
                continue
            total += abs(defects[v]) if absolute else defects[v]
            for w, l in adjacency[v]:
                nd = d + l
                if nd <= radius_mm and nd < dist.get(w, math.inf):
                    dist[w] = nd
                    heapq.heappush(heap, (nd, w))
        out.append(abs(total))
    return out


def clusters(m: Mesh, signal: list[float], threshold: float) -> list[tuple[int, float]]:
    """Connected groups of vertices whose signal reaches the threshold; each as (vertex of maximum, that maximum)."""
    over = {v for v, s in enumerate(signal) if s >= threshold}
    adjacency: dict[int, set[int]] = {}
    for f in m.faces:
        for a, b in edges_of(f):
            if a in over and b in over:
                adjacency.setdefault(a, set()).add(b)
                adjacency.setdefault(b, set()).add(a)
    seen: set[int] = set()
    out: list[tuple[int, float]] = []
    for v in sorted(over):
        if v in seen:
            continue
        stack, group = [v], []
        seen.add(v)
        while stack:
            u = stack.pop()
            group.append(u)
            for w in adjacency.get(u, ()):
                if w not in seen:
                    seen.add(w)
                    stack.append(w)
        peak = max(group, key=lambda u: (signal[u], -u))
        out.append((peak, signal[peak]))
    return out


def read_obj(path: Path) -> Mesh:
    pos: list[Vec] = []
    faces: list[Face] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        word, *rest = line.split() or [""]
        if word == "v":
            x, y, z = (float(s) for s in rest)
            pos.append((x, y, z))
        elif word == "f":
            a, b, c = (int(s.split("/")[0]) - 1 for s in rest)
            faces.append((a, b, c))
    return Mesh(pos, faces)
