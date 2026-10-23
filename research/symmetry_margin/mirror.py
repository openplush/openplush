"""Mirror-plane candidates and their match scores, as research R1 defines them for the symmetry hint.

A candidate plane passes through the area centroid with a unit normal. Its score is the RMS distance from each
vertex's reflection to the surface, over the bbox diagonal: zero for an exact mirror plane, the same definition
as the 002 T002 fixture oracle (research/fixtures_gen/test_symmetry.py), so the numbers here and there compare.
Candidates come from area-weighted PCA (the design's starting set) and a Fibonacci sweep of normals, each
refined by a bounded pattern search, then merged when they converge to the same plane.
"""

import json
import math
import struct
from pathlib import Path

Vec = tuple[float, float, float]
Face = tuple[int, int, int]

CELL = 8.0  # mm; spatial hash cell for closest-triangle queries
SWEEP = 60  # normals tried before refining
REFINE = 3  # best sweep normals refined, on top of the three PCA planes
MOVES = 3  # pattern-search moves per step size; more only shaves the last digits
STEPS = (0.1, 0.05, 0.025, 0.0125, 0.00625)  # normal nudges; the offset nudge is OFFSET_MM times the step
OFFSET_MM = 20.0
SAME_PLANE_DEG = 10.0  # refined normals closer than this (as lines, n and -n alike) are one candidate

Plane = tuple[float, Vec, float]  # score, unit normal, offset along the normal from the volume centroid


def sub(a: Vec, b: Vec) -> Vec:
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def dot(a: Vec, b: Vec) -> float:
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def along(a: Vec, d: Vec, t: float) -> Vec:
    return (a[0] + t * d[0], a[1] + t * d[1], a[2] + t * d[2])


def cross(u: Vec, w: Vec) -> Vec:
    return (u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0])


def normalized(v: Vec) -> Vec:
    n = math.sqrt(dot(v, v))
    if n == 0.0:
        raise ValueError("zero vector has no direction")
    return (v[0] / n, v[1] / n, v[2] / n)


def field(obj: object, key: str) -> object:
    if not isinstance(obj, dict) or key not in obj:
        raise ValueError(f"missing {key!r}")
    value: object = obj[key]
    return value


def only(obj: object, what: str) -> object:
    if not isinstance(obj, list) or len(obj) != 1:
        raise ValueError(f"expected exactly one {what}")
    item: object = obj[0]
    return item


def exact_int(obj: object, what: str) -> int:
    if not isinstance(obj, int) or isinstance(obj, bool):
        raise ValueError(f"{what}: expected an integer, got {obj!r}")
    return obj


def read_glb_mesh(path: Path) -> tuple[list[Vec], list[Face]]:
    """Positions and triangles of a GLB with one mesh of one primitive, float32 POSITION and uint32 indices."""
    data = path.read_bytes()
    magic, version, length = struct.unpack_from("<4sII", data, 0)
    if (magic, version, length) != (b"glTF", 2, len(data)):
        raise ValueError(f"{path.name}: bad GLB header {(magic, version, length)}")
    js_len, js_type = struct.unpack_from("<I4s", data, 12)
    bin_at = 20 + js_len
    bin_len, bin_type = struct.unpack_from("<I4s", data, bin_at)
    if (js_type, bin_type) != (b"JSON", b"BIN\0") or bin_at + 8 + bin_len != len(data):
        raise ValueError(f"{path.name}: bad GLB chunks")
    doc: object = json.loads(data[20:bin_at])
    blob = data[bin_at + 8 :]
    prim = only(field(only(field(doc, "meshes"), "mesh"), "primitives"), "primitive")
    attrs = field(prim, "attributes")
    if not isinstance(attrs, dict) or set(attrs) != {"POSITION"}:
        raise ValueError(f"{path.name}: expected POSITION only, got {attrs}")
    pos = accessor(doc, blob, exact_int(field(attrs, "POSITION"), "POSITION"), "VEC3", 5126, "f", 3)
    idx = accessor(doc, blob, exact_int(field(prim, "indices"), "indices"), "SCALAR", 5125, "I", 1)
    faces = [(int(idx[i][0]), int(idx[i + 1][0]), int(idx[i + 2][0])) for i in range(0, len(idx), 3)]
    return [(p[0], p[1], p[2]) for p in pos], faces


def accessor(
    doc: object, blob: bytes, index: int, kind: str, component: int, fmt: str, width: int
) -> list[tuple[float, ...]]:
    """A tightly packed accessor's rows, after checking its type and component type."""
    accessors, views = field(doc, "accessors"), field(doc, "bufferViews")
    if not isinstance(accessors, list) or not isinstance(views, list):
        raise ValueError("accessors and bufferViews must be arrays")
    acc: object = accessors[index]
    if (field(acc, "type"), exact_int(field(acc, "componentType"), "componentType")) != (kind, component):
        raise ValueError(f"accessor {index}: expected {kind} of component type {component}")
    view: object = views[exact_int(field(acc, "bufferView"), "bufferView")]
    count, start = exact_int(field(acc, "count"), "count"), exact_int(field(view, "byteOffset"), "byteOffset")
    flat = struct.unpack_from(f"<{count * width}{fmt}", blob, start)
    return [tuple(float(x) for x in flat[i : i + width]) for i in range(0, len(flat), width)]


def closest_on_triangle(p: Vec, a: Vec, b: Vec, c: Vec) -> Vec:
    """Closest point to `p` on triangle abc, by Voronoi region (Ericson, Real-Time Collision Detection, 5.1.5)."""
    ab, ac, ap = sub(b, a), sub(c, a), sub(p, a)
    d1, d2 = dot(ab, ap), dot(ac, ap)
    if d1 <= 0 and d2 <= 0:
        return a
    bp = sub(p, b)
    d3, d4 = dot(ab, bp), dot(ac, bp)
    if d3 >= 0 and d4 <= d3:
        return b
    vc = d1 * d4 - d3 * d2
    if vc <= 0 and d1 >= 0 and d3 <= 0:
        return along(a, ab, d1 / (d1 - d3))
    cp = sub(p, c)
    d5, d6 = dot(ab, cp), dot(ac, cp)
    if d6 >= 0 and d5 <= d6:
        return c
    vb = d5 * d2 - d1 * d6
    if vb <= 0 and d2 >= 0 and d6 <= 0:
        return along(a, ac, d2 / (d2 - d6))
    va = d3 * d6 - d5 * d4
    if va <= 0 and d4 - d3 >= 0 and d5 - d6 >= 0:
        return along(b, sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)))
    v, w = vb / (va + vb + vc), vc / (va + vb + vc)
    return along(along(a, ab, v), ac, w)


class Surface:
    """Distance from a point to a triangle mesh, through a spatial hash of triangle bounding boxes."""

    def __init__(self, pos: list[Vec], faces: list[Face]) -> None:
        self.tris = [(pos[a], pos[b], pos[c]) for a, b, c in faces]
        self.cells: dict[tuple[int, int, int], list[int]] = {}
        for i, t in enumerate(self.tris):
            lo = [math.floor(min(p[k] for p in t) / CELL) for k in range(3)]
            hi = [math.floor(max(p[k] for p in t) / CELL) for k in range(3)]
            for x in range(lo[0], hi[0] + 1):
                for y in range(lo[1], hi[1] + 1):
                    for z in range(lo[2], hi[2] + 1):
                        self.cells.setdefault((x, y, z), []).append(i)

    def distance(self, p: Vec) -> float:
        """Searches shells of cells outward until no unsearched cell can hold anything nearer."""
        k = [math.floor(c / CELL) for c in p]
        best, seen, r = math.inf, set[int](), 0
        while best > r * CELL:  # every point within r * CELL of p lies in a cell within r shells
            for a in range(-r, r + 1):
                for b in range(-r, r + 1):
                    for c in range(-r, r + 1):
                        if max(abs(a), abs(b), abs(c)) != r:
                            continue
                        for i in self.cells.get((k[0] + a, k[1] + b, k[2] + c), []):
                            if i not in seen:
                                seen.add(i)
                                best = min(best, math.dist(p, closest_on_triangle(p, *self.tris[i])))
            r += 1
        return best


def jacobi_eigenvectors(m: list[list[float]]) -> list[Vec]:
    """Eigenvectors of a symmetric 3 × 3 matrix by cyclic Jacobi rotations, largest eigenvalue first."""
    a = [row[:] for row in m]
    v = [[1.0 if i == j else 0.0 for j in range(3)] for i in range(3)]
    for _ in range(50):
        p, q = max(((i, j) for i in range(3) for j in range(i + 1, 3)), key=lambda ij: abs(a[ij[0]][ij[1]]))
        if abs(a[p][q]) < 1e-15:
            break
        theta = 0.5 * math.atan2(2 * a[p][q], a[q][q] - a[p][p])
        c, s = math.cos(theta), math.sin(theta)
        for k in range(3):
            akp, akq = a[k][p], a[k][q]
            a[k][p], a[k][q] = c * akp - s * akq, s * akp + c * akq
        for k in range(3):
            apk, aqk = a[p][k], a[q][k]
            a[p][k], a[q][k] = c * apk - s * aqk, s * apk + c * aqk
        for k in range(3):
            vkp, vkq = v[k][p], v[k][q]
            v[k][p], v[k][q] = c * vkp - s * vkq, s * vkp + c * vkq
    order = sorted(range(3), key=lambda i: -a[i][i])
    return [(v[0][i], v[1][i], v[2][i]) for i in order]


def fibonacci_normals(count: int) -> list[Vec]:
    """`count` directions spread over the upper hemisphere; a plane's normal and its negation are the same plane."""
    out: list[Vec] = []
    for i in range(count):
        z = (i + 0.5) / count
        t = i * math.pi * (3 - math.sqrt(5))
        out.append((math.sqrt(1 - z * z) * math.cos(t), math.sqrt(1 - z * z) * math.sin(t), z))
    return out


def same_plane(m: Vec, n: Vec) -> bool:
    return abs(dot(m, n)) >= math.cos(math.radians(SAME_PLANE_DEG))


class Mirror:
    """Mirror scores of a closed mesh, for planes given by a normal and an offset from its volume centroid.

    The offset is refined with the normal because no centroid is exact: the area centroid moves with which diagonal
    each quad takes, and the volume centroid moves with it too wherever a quad is not planar, so a plane through
    either misses an exact mirror plane of the shape by a fraction of an edge (measured: 0.01 mm off y = 0 on the
    upright two-plane head, which costs an exact plane a score of 0.0001).
    """

    def __init__(self, pos: list[Vec], faces: list[Face]) -> None:
        self.pos, self.surface = pos, Surface(pos, faces)
        lo = [min(p[k] for p in pos) for k in range(3)]
        hi = [max(p[k] for p in pos) for k in range(3)]
        self.diag = math.dist(lo, hi)
        volume, acc = 0.0, (0.0, 0.0, 0.0)
        samples: list[tuple[float, Vec]] = []
        for f in faces:
            a, b, c = (pos[v] for v in f)
            tetra = dot(a, cross(b, c)) / 6  # signed volume of the tetrahedron (origin, a, b, c)
            volume += tetra
            corners = (a[0] + b[0] + c[0], a[1] + b[1] + c[1], a[2] + b[2] + c[2])
            acc = along(acc, corners, tetra / 4)
            n = cross(sub(b, a), sub(c, a))
            samples.append((math.sqrt(dot(n, n)) / 2, (corners[0] / 3, corners[1] / 3, corners[2] / 3)))
        if volume <= 0:
            raise ValueError("mesh is not closed and wound outward")
        self.centroid = (acc[0] / volume, acc[1] / volume, acc[2] / volume)
        self.pca = self._pca(samples)

    def _pca(self, samples: list[tuple[float, Vec]]) -> list[Vec]:
        """Normals of the three principal planes of the surface, weighted by face area."""
        cov = [[0.0] * 3 for _ in range(3)]
        for area, c in samples:
            d = sub(c, self.centroid)
            for i in range(3):
                for j in range(3):
                    cov[i][j] += area * d[i] * d[j]
        return jacobi_eigenvectors(cov)

    def score(self, n: Vec, offset: float = 0.0) -> float:
        """RMS distance from each vertex's reflection in the plane to the surface, over the bbox diagonal."""
        sq = 0.0
        for p in self.pos:
            q = along(p, n, -2 * (dot(sub(p, self.centroid), n) - offset))
            sq += self.surface.distance(q) ** 2
        return math.sqrt(sq / len(self.pos)) / self.diag

    def refine(self, plane: Plane) -> Plane:
        """Nudges the normal along the axes and the offset along the normal while that lowers the score, at most
        MOVES times per step size, halving the step."""
        axes: list[Vec] = [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)]
        s, n, d = plane
        for step in STEPS:
            for _ in range(MOVES):
                tries = [(normalized(along(n, e, sign * step)), d) for e in axes for sign in (1, -1)]
                tries += [(n, d + sign * step * OFFSET_MM) for sign in (1, -1)]
                found = min((self.score(m, o), m, o) for m, o in tries)
                if found[0] >= s:
                    break
                s, n, d = found
        return s, n, d

    def candidates(self) -> list[Plane]:
        """Distinct refined planes, best first: the PCA planes plus the best REFINE of a SWEEP-normal sweep."""
        sweep = sorted((self.score(n), n, 0.0) for n in fibonacci_normals(SWEEP))
        starts = [(self.score(n), n, 0.0) for n in self.pca] + sweep[:REFINE]
        refined = sorted(self.refine(p) for p in starts)
        distinct: list[Plane] = []
        for plane in refined:
            if not any(same_plane(plane[1], m) for _, m, _ in distinct):
                distinct.append(plane)
        return distinct


def decide(cands: list[Plane], accept: float, margin: float) -> tuple[str, Plane | None]:
    """002 D-006's rule with an absolute margin: the best plane if it scores within `accept` and no other distinct
    plane both fits within `accept` and scores within `margin` of it; otherwise none, and why.

    A plane that does not fit cannot make the fitting one ambiguous, whatever the margin; a margin of `accept`
    or more reduces the rule to "ambiguous whenever two distinct planes fit".
    """
    if not cands or cands[0][0] > accept:
        return "none: no plane fits", None
    if len(cands) > 1 and cands[1][0] <= accept and cands[1][0] - cands[0][0] <= margin:
        return "none: ambiguous", None
    return "plane", cands[0]
