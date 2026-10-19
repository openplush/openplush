"""Oracles for the faceted fixture: a low-poly icosphere on which every edge reads as a hard fold.

Run: python3 -m unittest -v   (from this directory)
Reads the committed files only and shares no code with generate.py or source_faceted.py.
"""

import math
import unittest

from test_fixtures import FIXTURES, Face, Vec, detect, edges_of, signed_volume
from test_symmetry import read_glb_mesh

FACETED = FIXTURES / "meshes" / "faceted.glb"
CAP = 50  # hints per category above which the panel reports a flood instead of a list
RADIUS = 60.0  # mm
POS_TOL = 1e-4  # mm; f32 spacing near 60 mm is about 4e-6


def normal(pos: list[Vec], f: Face) -> Vec:
    a, b, c = (pos[v] for v in f)
    u = (b[0] - a[0], b[1] - a[1], b[2] - a[2])
    w = (c[0] - a[0], c[1] - a[1], c[2] - a[2])
    n = (u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0])
    length = math.sqrt(n[0] ** 2 + n[1] ** 2 + n[2] ** 2)
    return (n[0] / length, n[1] / length, n[2] / length)


def bends(pos: list[Vec], faces: list[Face]) -> dict[tuple[int, int], float]:
    """Angle in degrees between the normals of the two faces on each interior edge; 0 is flat. Boundary edges have none."""
    by_edge: dict[tuple[int, int], list[Vec]] = {}
    for f in faces:
        for a, b in edges_of(f):
            by_edge.setdefault((min(a, b), max(a, b)), []).append(normal(pos, f))
    out: dict[tuple[int, int], float] = {}
    for e, ns in by_edge.items():
        if len(ns) > 2:
            raise ValueError(f"edge {e} has {len(ns)} faces")
        if len(ns) < 2:
            continue
        n, m = ns
        out[e] = math.degrees(math.acos(max(-1.0, min(1.0, n[0] * m[0] + n[1] * m[1] + n[2] * m[2]))))
    return out


class Faceted(unittest.TestCase):
    """Prevents a 'faceted' fixture too smooth or too small to flood a category, so the flood path goes untested."""

    def test_is_a_clean_frequency_2_icosphere(self) -> None:
        pos, faces = read_glb_mesh(FACETED)
        self.assertEqual({k: v for k, v in detect(pos, faces).items() if v}, {})
        self.assertEqual((len(pos), len(faces)), (42, 80))
        self.assertGreater(signed_volume(pos, faces), 0, "wound outward")
        for p in pos:
            self.assertAlmostEqual(math.dist(p, (0.0, 0.0, 0.0)), RADIUS, delta=POS_TOL)

    def test_every_edge_is_a_hard_fold_long_enough_to_survive_a_length_filter(self) -> None:
        """More than CAP edges, each folded at least 15 degrees and at least 30 mm long, so none is near-coplanar noise."""
        pos, faces = read_glb_mesh(FACETED)
        folds = bends(pos, faces)
        self.assertEqual(len(folds), 120)
        self.assertGreater(len(folds), CAP)
        self.assertGreater(min(folds.values()), 15.0)
        self.assertGreater(min(math.dist(pos[a], pos[b]) for a, b in folds), 30.0)

    def test_the_bend_measure_reads_zero_on_a_flat_pair(self) -> None:
        """Prevents the fold check passing because the bend measure is wrong rather than because the edges fold."""
        flat = [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (1.0, 1.0, 0.0), (0.0, 1.0, 0.0)]
        self.assertAlmostEqual(bends(flat, [(0, 1, 2), (0, 2, 3)])[(0, 2)], 0.0)


if __name__ == "__main__":
    unittest.main()
