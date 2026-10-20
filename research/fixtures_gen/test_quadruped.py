"""Oracles for the quadruped fixture: the primary tuning model for the seam-placement hints.

Run: python3 -m unittest -v   (from this directory)
Reads the committed file only and shares no code with generate.py or source_quadruped.py.
"""

import math
import unittest
from collections import Counter

from test_fixtures import FIXTURES, Face, UnionFind, Vec, edges_of, signed_volume, triangles_cross
from test_source_seams import read_glb
from test_symmetry import read_glb_mesh

QUADRUPED = FIXTURES / "meshes" / "quadruped.glb"
CELL = 8.0  # mm; spatial hash cell for the self-intersection search
CREASE_DEG = 40.0  # a concave fold at least this sharp is what the concave-crease detector must find
Box = tuple[tuple[float, float], tuple[float, float], tuple[float, float]]  # x, y, z ranges
NECK: Box = ((-20.0, 20.0), (44.0, 84.0), (78.0, 118.0))
MUZZLE: Box = ((-16.0, 16.0), (96.0, 116.0), (90.0, 120.0))
DOME: Box = ((-22.0, 22.0), (66.0, 104.0), (130.0, 160.0))
LEG_TOPS: Box = ((-30.0, 30.0), (-58.0, 58.0), (34.0, 58.0))
EAR_ROOTS: Box = ((-24.0, 24.0), (60.0, 96.0), (124.0, 150.0))
TAIL_ROOT: Box = ((-12.0, 12.0), (-90.0, -56.0), (70.0, 110.0))
FLANK: Box = ((-40.0, 40.0), (-40.0, 40.0), (60.0, 90.0))  # the body's sides, outside |x| 20, are nearly flat
FEET_Z = 6.0  # mm; everything below is a foot


def sub(a: Vec, b: Vec) -> Vec:
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def cross(u: Vec, w: Vec) -> Vec:
    return (u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0])


def dot(a: Vec, b: Vec) -> float:
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def unit(v: Vec) -> Vec:
    n = math.sqrt(dot(v, v))
    return (v[0] / n, v[1] / n, v[2] / n)


def midpoint(a: Vec, b: Vec) -> Vec:
    return ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (a[2] + b[2]) / 2)


def inside(p: Vec, box: Box) -> bool:
    return all(lo <= p[i] <= hi for i, (lo, hi) in enumerate(box))


def normal(pos: list[Vec], f: Face) -> Vec:
    a, b, c = (pos[v] for v in f)
    return unit(cross(sub(b, a), sub(c, a)))


def centroid(pos: list[Vec], f: Face) -> Vec:
    a, b, c = (pos[v] for v in f)
    return ((a[0] + b[0] + c[0]) / 3, (a[1] + b[1] + c[1]) / 3, (a[2] + b[2] + c[2]) / 3)


def signed_folds(pos: list[Vec], faces: list[Face]) -> dict[tuple[int, int], float]:
    """Fold angle in degrees on each interior edge, negative where the surface folds inward (a concave crease)."""
    by_edge: dict[tuple[int, int], list[int]] = {}
    for i, f in enumerate(faces):
        for a, b in edges_of(f):
            by_edge.setdefault((min(a, b), max(a, b)), []).append(i)
    out: dict[tuple[int, int], float] = {}
    for e, fs in by_edge.items():
        if len(fs) != 2:
            raise ValueError(f"edge {e} has {len(fs)} faces")
        n, m = normal(pos, faces[fs[0]]), normal(pos, faces[fs[1]])
        angle = math.degrees(math.acos(max(-1.0, min(1.0, dot(n, m)))))
        # The second face's centroid lies above the first face's plane exactly when the fold is concave.
        concave = dot(n, sub(centroid(pos, faces[fs[1]]), centroid(pos, faces[fs[0]]))) > 0
        out[e] = -angle if concave else angle
    return out


def angle_defects(pos: list[Vec], faces: list[Face]) -> dict[int, float]:
    """Gaussian curvature per vertex as the angle defect, in degrees: 360 minus the sum of corner angles."""
    total: dict[int, float] = {}
    for f in faces:
        for k in range(3):
            v, u, w = f[k], f[(k + 1) % 3], f[(k + 2) % 3]
            a, b = unit(sub(pos[u], pos[v])), unit(sub(pos[w], pos[v]))
            total[v] = total.get(v, 0.0) + math.degrees(math.acos(max(-1.0, min(1.0, dot(a, b)))))
    return {v: 360.0 - s for v, s in total.items()}


def crossings(pos: list[Vec], faces: list[Face]) -> int:
    """Transverse crossings between faces sharing no vertex, through a spatial hash of face bounding boxes."""
    tris = [(pos[a], pos[b], pos[c]) for a, b, c in faces]
    cells: dict[tuple[int, int, int], list[int]] = {}
    for i, t in enumerate(tris):
        lo = [math.floor(min(p[k] for p in t) / CELL) for k in range(3)]
        hi = [math.floor(max(p[k] for p in t) / CELL) for k in range(3)]
        for x in range(lo[0], hi[0] + 1):
            for y in range(lo[1], hi[1] + 1):
                for z in range(lo[2], hi[2] + 1):
                    cells.setdefault((x, y, z), []).append(i)
    seen: set[tuple[int, int]] = set()
    count = 0
    for bucket in cells.values():
        for i in bucket:
            for j in bucket:
                if i < j and (i, j) not in seen and not set(faces[i]) & set(faces[j]):
                    seen.add((i, j))
                    count += triangles_cross(tris[i], tris[j])
    return count


class Quadruped(unittest.TestCase):
    """Prevents a tuning fixture that is defective, asymmetric, or missing the features the hints are tuned on."""

    pos: list[Vec]
    faces: list[Face]

    @classmethod
    def setUpClass(cls) -> None:
        cls.pos, cls.faces = read_glb_mesh(QUADRUPED)

    def test_closed_manifold_outward_and_clean(self) -> None:
        """A mesh the engine would repair first would tune the detectors on repair's output, not the model."""
        pos, faces = self.pos, self.faces
        uses = Counter((min(a, b), max(a, b)) for f in faces for a, b in edges_of(f))
        self.assertEqual(set(uses.values()), {2}, "every edge on exactly two faces")
        self.assertEqual(len(pos) - len(uses) + len(faces), 2, "sphere topology")
        self.assertEqual(len(set(pos)), len(pos), "no coincident vertices")
        self.assertEqual({v for f in faces for v in f}, set(range(len(pos))), "no unused vertex")
        self.assertEqual(len({tuple(sorted(f)) for f in faces}), len(faces), "no duplicate face")
        directed = Counter(e for f in faces for e in edges_of(f))
        self.assertEqual(max(directed.values()), 1, "consistently wound")
        self.assertGreater(signed_volume(pos, faces), 0, "wound outward")
        fans: UnionFind[tuple[int, int]] = UnionFind()
        by_edge: dict[tuple[int, int], list[int]] = {}
        for i, f in enumerate(faces):
            for a, b in edges_of(f):
                by_edge.setdefault((min(a, b), max(a, b)), []).append(i)
        for (a, b), (i, j) in by_edge.items():
            fans.union((a, i), (a, j))
            fans.union((b, i), (b, j))
        roots = {(v, fans.find((v, i))) for i, f in enumerate(faces) for v in f}
        self.assertEqual(len(roots), len(pos), "one fan per vertex: no bowtie")
        lo = [min(p[k] for p in pos) for k in range(3)]
        hi = [max(p[k] for p in pos) for k in range(3)]
        floor = 1e-12 * sum((h - l) ** 2 for h, l in zip(hi, lo)) ** 2
        for f in faces:
            p, q, r = (pos[v] for v in f)
            n = cross(sub(q, p), sub(r, p))
            self.assertGreater(dot(n, n) / 4, floor, f"face {f} has no area")
        self.assertEqual(crossings(pos, faces), 0)

    def test_the_embedded_note_names_the_mesher_that_made_it(self) -> None:
        """Prevents a fixture whose own provenance string contradicts how it was made: the note travels with the file."""
        doc, _ = read_glb(QUADRUPED)
        note = doc["asset"]["extras"]["note"] if isinstance(doc, dict) else ""
        self.assertIn("dual contouring", note)
        self.assertNotIn("surface nets", note)
        self.assertNotIn("marching", note)

    def test_mirror_symmetric_about_x_0_bit_for_bit(self) -> None:
        """Symmetry detection (SC-005) must find x = 0 on this model with a score of exactly zero."""
        vertices = set(self.pos)
        self.assertTrue(all((-p[0], p[1], p[2]) in vertices for p in self.pos))
        self.assertTrue(any(p[0] > 0 for p in self.pos) and any(p[0] < 0 for p in self.pos))

    def test_is_animal_sized_with_four_feet_and_a_tail(self) -> None:
        """Keeps the model a quadruped: longer than tall, wider than a profile, standing on four separate feet."""
        pos = self.pos
        lo = [min(p[k] for p in pos) for k in range(3)]
        hi = [max(p[k] for p in pos) for k in range(3)]
        length, width, height = hi[1] - lo[1], hi[0] - lo[0], hi[2] - lo[2]
        self.assertTrue(200 < length < 260 and 60 < width < 80 and 140 < height < 170, (length, width, height))
        feet = [p for p in pos if p[2] < FEET_Z]
        quadrants = {(p[0] > 0, p[1] > 0) for p in feet}
        self.assertEqual(len(quadrants), 4, "a foot in every quadrant")
        self.assertTrue(all(abs(p[0]) > 8 and abs(p[1]) > 30 for p in feet), "feet apart from the body axis")
        tail = [p for p in pos if p[1] < -80]
        self.assertTrue(tail and all(p[2] > 70 for p in tail), "a raised tail behind the body")

    def test_neck_and_muzzle_are_concave_creases_and_nothing_else_is(self) -> None:
        """The concave-crease detector is tuned on the neck and the muzzle fold; a crease elsewhere would be a
        tuning target nobody asked for, and a rounded neck would give it nothing to find."""
        pos, faces = self.pos, self.faces
        folds = signed_folds(pos, faces)
        mid = {e: midpoint(pos[e[0]], pos[e[1]]) for e in folds}
        creases = [e for e, a in folds.items() if a <= -CREASE_DEG]
        self.assertGreater(sum(inside(mid[e], NECK) for e in creases), 20, "a ring of creases around the neck")
        self.assertGreater(sum(inside(mid[e], MUZZLE) for e in creases), 12, "a ring of creases around the muzzle")
        elsewhere = [mid[e] for e in creases if not inside(mid[e], NECK) and not inside(mid[e], MUZZLE)]
        stray = [p for p in elsewhere if not (inside(p, LEG_TOPS) or inside(p, EAR_ROOTS) or inside(p, TAIL_ROOT))]
        self.assertEqual(stray, [], "the only other creases are where legs, ears and tail join")

    def test_head_dome_carries_the_curvature(self) -> None:
        """The dart-candidate signal integrates curvature over a disk; the dome must hold enough of it, and the
        flanks of the body must not, or the detector would fire everywhere at any sensitivity."""
        pos, faces = self.pos, self.faces
        defects = angle_defects(pos, faces)
        dome = sum(d for v, d in defects.items() if inside(pos[v], DOME))
        flank = sum(d for v, d in defects.items() if inside(pos[v], FLANK) and abs(pos[v][0]) > 20)
        self.assertGreater(dome, 60.0, "a dome holds a good share of a hemisphere's 360 degrees")
        self.assertLess(flank, dome / 3, "the body's flank is nearly flat by comparison")


if __name__ == "__main__":
    unittest.main()
