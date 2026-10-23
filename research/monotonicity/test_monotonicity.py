"""Oracles for 002 T006: shapes whose hint sets at two thresholds are known by hand, and the corpus property.

Run: python3 -m unittest -v   (from this directory; the corpus case takes about 20 s)
"""

import math
import random
import sys
import unittest
from pathlib import Path

from monotone import CreaseHints, DartHints, Mesh, clusters
from property import FOLD, LENGTH, PAIRS, RADIUS_MM, SEED, ordered

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "default_thresholds"))
from sweep import load  # noqa: E402


def ridge(folds_deg: list[float], segment_mm: float = 10.0) -> Mesh:
    """A straight crease of one edge per fold: each edge is shared by a left and a right triangle with their own
    apex, placed so the faces meet at that fold, so the crease edges are the mesh's only interior edges."""
    n = len(folds_deg)
    pos = [(segment_mm * i, 0.0, 0.0) for i in range(n + 1)]
    faces = []
    for i, fold in enumerate(folds_deg):
        half = math.radians(fold) / 2
        x = segment_mm * (i + 0.5)
        pos.append((x, segment_mm * math.cos(half), segment_mm * math.sin(half)))
        pos.append((x, -segment_mm * math.cos(half), segment_mm * math.sin(half)))
        left, right = len(pos) - 2, len(pos) - 1
        faces.append((i, i + 1, left))
        faces.append((i + 1, i, right))
    return Mesh(pos, faces)


def strip(n: int) -> Mesh:
    """A path of n vertices, each edge carrying one triangle with a private apex, so adjacency is the path."""
    pos = [(float(i), 0.0, 0.0) for i in range(n)] + [(i + 0.5, 1.0, 0.0) for i in range(n - 1)]
    return Mesh(pos, [(i, i + 1, n + i) for i in range(n - 1)])


class RidgeOracle(unittest.TestCase):
    """A 40°, 22°, 40° crease: one chain built once; two fragments when rebuilt above 22°."""

    def setUp(self) -> None:
        self.hints = CreaseHints(ridge([40.0, 22.0, 40.0]))

    def test_the_ridge_is_one_chain_at_the_permissive_fold(self) -> None:
        self.assertEqual(len(self.hints.once), 1, "the crease edges did not chain: the oracle mesh is wrong")
        self.assertAlmostEqual(self.hints.once[0].mean_fold, 34.0, places=6)

    def test_built_once_the_strict_set_is_inside_the_loose_set(self) -> None:
        self.assertLessEqual(self.hints.filtered(30.0, 0.0), self.hints.filtered(20.0, 0.0))
        self.assertEqual(len(self.hints.filtered(30.0, 0.0)), 1, "a 34° mean chain must pass a 30° filter")

    def test_rebuilt_per_threshold_the_strict_set_escapes_the_loose_set(self) -> None:
        strict, loose = self.hints.rebuilt(30.0, 0.0), self.hints.rebuilt(20.0, 0.0)
        self.assertEqual(len(strict), 2, "above 22° the middle edge drops and the ridge splits in two")
        self.assertEqual(len(loose), 1)
        self.assertFalse(strict <= loose, "two fragments are not the one chain: raising sensitivity would replace hints")

    def test_rebuilt_a_length_filter_drops_what_the_loose_setting_showed(self) -> None:
        self.assertEqual(self.hints.rebuilt(30.0, 15.0), set(), "10 mm fragments fail a 15 mm minimum")
        self.assertEqual(len(self.hints.rebuilt(20.0, 15.0)), 1)


class StripOracle(unittest.TestCase):
    """A signal 50, 70, 40, 80, 50 along a path: one cluster peaking at 80; two when rebuilt above 40."""

    def setUp(self) -> None:
        self.hints = DartHints(strip(5), radius_mm=0.0)
        self.hints.signal = [50.0, 70.0, 40.0, 80.0, 50.0] + [0.0] * 4
        self.hints.once = clusters(self.hints.m, self.hints.signal, 30.0)

    def test_built_once_a_stricter_threshold_keeps_the_same_peak(self) -> None:
        self.assertEqual(self.hints.filtered(30.0), {3})
        self.assertEqual(self.hints.filtered(45.0), {3})
        self.assertEqual(self.hints.filtered(90.0), set())

    def test_rebuilt_per_threshold_a_second_peak_appears_when_tightening(self) -> None:
        self.assertEqual(self.hints.rebuilt(30.0), {3})
        self.assertEqual(self.hints.rebuilt(45.0), {1, 3}, "the saddle at 40 splits the cluster: a new hint at a stricter setting")
        self.assertFalse(self.hints.rebuilt(45.0) <= self.hints.rebuilt(30.0))


class CorpusProperty(unittest.TestCase):
    """Built once, no ordered pair of settings on any fixture breaks the subset relation (FR-006, SC-009)."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.meshes = load()

    def test_crease_chains_built_once_are_monotone_on_every_fixture(self) -> None:
        rng = random.Random(SEED)
        for name, m in sorted(self.meshes.items()):
            hints = CreaseHints(m)
            for _ in range(PAIRS):
                (sf, lf), (sl, ll) = ordered(rng, *FOLD), ordered(rng, *LENGTH)
                self.assertLessEqual(hints.filtered(sf, sl), hints.filtered(lf, ll), f"{name}: ({sf:.1f}°, {sl:.1f} mm) ⊄ ({lf:.1f}°, {ll:.1f} mm)")

    def test_dart_clusters_built_once_are_monotone_on_every_fixture(self) -> None:
        rng = random.Random(SEED)
        for name, m in sorted(self.meshes.items()):
            hints = DartHints(m, RADIUS_MM)
            for _ in range(PAIRS):
                strict, loose = ordered(rng, 30.0, 240.0)
                self.assertLessEqual(hints.filtered(strict), hints.filtered(loose), f"{name}: {strict:.1f}° ⊄ {loose:.1f}°")

    def test_rebuilding_breaks_monotonicity_somewhere_on_the_corpus(self) -> None:
        rng = random.Random(SEED)
        broken = 0
        for _, m in sorted(self.meshes.items()):
            hints = CreaseHints(m)
            for _ in range(PAIRS):
                (sf, lf), (sl, ll) = ordered(rng, *FOLD), ordered(rng, *LENGTH)
                broken += not hints.rebuilt(sf, sl) <= hints.rebuilt(lf, ll)
        self.assertGreater(broken, 0, "if rebuilding never breaks the subset relation, the chain-once rule costs nothing to drop")


if __name__ == "__main__":
    unittest.main()
