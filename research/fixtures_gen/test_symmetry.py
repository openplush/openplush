"""Oracles for the symmetry fixtures: a head with one tilted ear, and a head with no mirror plane.

Run: python3 -m unittest -v   (from this directory)
Reads the committed files only and shares no code with generate.py or source_symmetry.py.
"""

import math
import unittest
from pathlib import Path

from test_fixtures import FIXTURES, Face, Vec, array, detect, edges_of, exact_int, field, signed_volume
from test_source_seams import accessor_floats, accessor_ints, read_glb

NEAR = FIXTURES / "meshes" / "near-symmetric.glb"
ASYM = FIXTURES / "meshes" / "asymmetric.glb"
CELL = 8.0  # mm; spatial hash cell for closest-triangle queries
NEAR_MAX = 0.005  # near-symmetric's mirror score about x = 0 stays below this; the asymmetric's best stays above twice it
SWEEP = 60  # plane normals tried before refining; with the refinement, finds near-symmetric's plane from scratch
AXES: list[Vec] = [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)]
MOVES = 3  # bounds the refinement's cost; a long crawl at one step size only shaves the last digits


def read_glb_mesh(path: Path) -> tuple[list[Vec], list[Face]]:
    """Positions and triangles of a GLB with one mesh of one primitive and no attribute but POSITION."""
    doc, blob = read_glb(path)
    (mesh,) = array(field(doc, "meshes"), "meshes")
    (prim,) = array(field(mesh, "primitives"), "primitives")
    attrs = field(prim, "attributes")
    if not isinstance(attrs, dict) or set(attrs) != {"POSITION"}:
        raise ValueError(f"{path.name}: expected POSITION only, got {attrs}")
    pos = accessor_floats(doc, blob, exact_int(field(attrs, "POSITION"), "POSITION"), 3)
    idx = accessor_ints(doc, blob, exact_int(field(prim, "indices"), "indices"))
    return [(p[0], p[1], p[2]) for p in pos], [(idx[i], idx[i + 1], idx[i + 2]) for i in range(0, len(idx), 3)]


def sub(a: Vec, b: Vec) -> Vec:
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def dot(a: Vec, b: Vec) -> float:
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def along(a: Vec, d: Vec, t: float) -> Vec:
    return (a[0] + t * d[0], a[1] + t * d[1], a[2] + t * d[2])


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


class Mirror:
    """How far a mesh is from its own reflection, per plane through its area centroid, relative to its bbox diagonal."""

    def __init__(self, pos: list[Vec], faces: list[Face]) -> None:
        self.pos, self.surface = pos, Surface(pos, faces)
        lo = [min(p[k] for p in pos) for k in range(3)]
        hi = [max(p[k] for p in pos) for k in range(3)]
        self.diag = math.dist(lo, hi)
        total, acc = 0.0, (0.0, 0.0, 0.0)
        for f in faces:
            a, b, c = (pos[v] for v in f)
            u, w = sub(b, a), sub(c, a)
            n = (u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0])
            area = math.sqrt(dot(n, n)) / 2
            total += area
            acc = along(acc, (a[0] + b[0] + c[0], a[1] + b[1] + c[1], a[2] + b[2] + c[2]), area / 3)
        self.centroid = (acc[0] / total, acc[1] / total, acc[2] / total)

    def score(self, n: Vec) -> float:
        """RMS distance from each vertex's reflection to the surface; zero for an exact mirror plane with unit normal n."""
        sq = 0.0
        for p in self.pos:
            q = along(p, n, -2 * dot(sub(p, self.centroid), n))
            sq += self.surface.distance(q) ** 2
        return math.sqrt(sq / len(self.pos)) / self.diag

    def best(self) -> tuple[float, Vec]:
        """A Fibonacci sweep of normals over a hemisphere, then a pattern search from the three best."""
        sweep: list[tuple[float, Vec]] = []
        for i in range(SWEEP):
            z = (i + 0.5) / SWEEP
            t = i * math.pi * (3 - math.sqrt(5))
            n = (math.sqrt(1 - z * z) * math.cos(t), math.sqrt(1 - z * z) * math.sin(t), z)
            sweep.append((self.score(n), n))
        sweep.sort()
        return min(self.refine(s, n) for s, n in sweep[:3])

    def refine(self, s: float, n: Vec) -> tuple[float, Vec]:
        """Moves `n` along the axes while that lowers the score, at most MOVES times per step size, halving the step."""
        for step in (0.1, 0.05, 0.025, 0.0125, 0.00625):
            for _ in range(MOVES):
                tries = [along(n, e, sign * step) for e in AXES for sign in (1, -1)]
                found = min((self.score(m), m) for m in (along((0.0, 0.0, 0.0), t, 1 / math.sqrt(dot(t, t))) for t in tries))
                if found[0] >= s:
                    break
                s, n = found
        return s, n


class Clean(unittest.TestCase):
    """Prevents a symmetry fixture that is itself defective, so that a symmetry test fails for the wrong reason."""

    def test_both_are_closed_manifold_oriented_and_clean(self) -> None:
        for path in (NEAR, ASYM):
            with self.subTest(path.name):
                pos, faces = read_glb_mesh(path)
                self.assertEqual({k: v for k, v in detect(pos, faces).items() if v}, {})
                edges = {(min(e), max(e)) for f in faces for e in edges_of(f)}
                self.assertEqual(len(pos) - len(edges) + len(faces), 2, "sphere topology")
                self.assertEqual({v for f in faces for v in f}, set(range(len(pos))), "no unused vertex")
                self.assertGreater(signed_volume(pos, faces), 0, "wound outward")


class NearSymmetric(unittest.TestCase):
    """Prevents a near-symmetric fixture that is exactly symmetric, or asymmetric everywhere rather than at one ear."""

    def test_only_the_ears_break_the_mirror_x_to_minus_x(self) -> None:
        pos, _ = read_glb_mesh(NEAR)
        vertices = set(pos)
        unmatched = [p for p in pos if (-p[0], p[1], p[2]) not in vertices]
        self.assertGreater(len(unmatched), 0, "not exactly symmetric")
        self.assertLess(len(unmatched), len(pos) / 10, "everything but a small region mirrors bit for bit")
        # The tilted +x ear has no exact partner, and neither does the -x ear, whose partner would be the untilted ear.
        self.assertTrue(any(p[0] > 0 for p in unmatched) and any(p[0] < 0 for p in unmatched))
        self.assertTrue(all(p[2] > 0 for p in unmatched), "both ears are on top")

    def test_mirror_score_about_x_0_is_small_but_not_zero(self) -> None:
        score = Mirror(*read_glb_mesh(NEAR)).score((1.0, 0.0, 0.0))
        self.assertGreater(score, NEAR_MAX / 10)
        self.assertLess(score, NEAR_MAX)

    def test_the_plane_search_finds_x_0_from_scratch(self) -> None:
        """Prevents the asymmetric oracle passing vacuously because the search cannot find a plane that exists."""
        score, n = Mirror(*read_glb_mesh(NEAR)).best()
        self.assertLess(score, NEAR_MAX)
        self.assertGreater(abs(n[0]), 0.99)


class Asymmetric(unittest.TestCase):
    """Prevents an "asymmetric" fixture with a mirror plane, which would let a detector that always says yes pass."""

    def test_no_plane_scores_within_twice_the_near_symmetric_bound(self) -> None:
        score, n = Mirror(*read_glb_mesh(ASYM)).best()
        self.assertGreater(score, 2 * NEAR_MAX, f"best plane normal {n}")


if __name__ == "__main__":
    unittest.main()
