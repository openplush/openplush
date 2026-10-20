"""Oracles for the H1 signals, on shapes whose answers are known by hand.

Run: python3 -m unittest -v   (from this directory)
"""

import math
import unittest

from signals import Chain, Mesh, angle_defects, chains, clusters, integrated_curvature, signed_folds


def box(w: float, d: float, h: float) -> Mesh:
    """A closed box with outward winding, 12 triangles."""
    pos = [(x, y, z) for z in (0.0, h) for y in (0.0, d) for x in (0.0, w)]
    quads = [(0, 2, 3, 1), (4, 5, 7, 6), (0, 1, 5, 4), (2, 6, 7, 3), (0, 4, 6, 2), (1, 3, 7, 5)]
    faces = [t for a, b, c, d_ in quads for t in ((a, b, c), (a, c, d_))]
    return Mesh(pos, faces)


def tube(n: int, r: float, length: float, rings: int) -> Mesh:
    """An open cylinder of `rings` bands, each `n` quads, wound outward."""
    pos = [(r * math.cos(2 * math.pi * j / n), r * math.sin(2 * math.pi * j / n), length * i / rings) for i in range(rings + 1) for j in range(n)]
    faces = []
    for i in range(rings):
        for j in range(n):
            a, b = i * n + j, i * n + (j + 1) % n
            c, d = a + n, b + n
            faces += [(a, b, d), (a, d, c)]
    return Mesh(pos, faces)


class Folds(unittest.TestCase):
    """Prevents a sign convention that calls a box's edges valleys, or a magnitude that is not the dihedral."""

    def test_box_edges_fold_plus_90_and_face_diagonals_zero(self) -> None:
        m = box(10.0, 20.0, 30.0)
        folds = signed_folds(m)
        self.assertEqual(sorted(round(a) for a in folds.values()), [0] * 6 + [90] * 12)

    def test_an_inward_fold_is_negative(self) -> None:
        pos = [(0.0, 0.0, 5.0), (10.0, 0.0, 0.0), (0.0, 10.0, 5.0), (-10.0, 0.0, 0.0)]
        # A ridge along (0, 2) wound to face up is a roof; the same faces wound the other way see it as a valley.
        roof, valley = Mesh(pos, [(0, 1, 2), (0, 2, 3)]), Mesh(pos, [(0, 2, 1), (0, 3, 2)])
        self.assertGreater(signed_folds(roof)[(0, 2)], 50.0)
        self.assertAlmostEqual(signed_folds(valley)[(0, 2)], -signed_folds(roof)[(0, 2)])


class Chains(unittest.TestCase):
    """Prevents chains that stop at every vertex, or swallow a side branch instead of going straight on."""

    def test_a_tube_chains_each_facet_edge_end_to_end(self) -> None:
        m = tube(12, 10.0, 40.0, 2)
        folds = signed_folds(m)
        got = chains(m, folds, [e for e, a in folds.items() if abs(a) >= 10.0])
        # The 12 lengthwise facet edges fold 30° and run through two bands; the ring edges between coplanar
        # quads are flat and the rims are boundary, so each facet edge is one 40 mm chain of two edges.
        self.assertEqual(sorted(len(c.edges) for c in got), [2] * 12)
        self.assertTrue(all(abs(c.length_mm - 40.0) < 1e-9 and abs(c.mean_fold - 30.0) < 1e-9 for c in got))

    def test_a_junction_pairs_the_straightest_continuation(self) -> None:
        m = box(10.0, 10.0, 10.0)
        folds = signed_folds(m)
        got = chains(m, folds, [e for e, a in folds.items() if a >= 10.0])
        # Each box corner meets three 90° edges at right angles: no straight continuation, so one pair and a stub.
        self.assertEqual(sum(len(c.edges) for c in got), 12)
        self.assertTrue(all(isinstance(c, Chain) and c.mean_fold == 90.0 for c in got))


class Curvature(unittest.TestCase):
    """Prevents an angle defect that does not sum to 720° on a closed surface, or a disk that ignores distance."""

    def test_defects_sum_to_720_on_a_box_and_concentrate_at_its_corners(self) -> None:
        d = angle_defects(box(10.0, 20.0, 30.0))
        self.assertAlmostEqual(sum(d), 720.0, places=6)
        self.assertEqual([round(x) for x in d], [90] * 8)

    def test_integration_radius_counts_only_vertices_within_reach(self) -> None:
        m = box(10.0, 10.0, 10.0)
        self.assertEqual([round(x) for x in integrated_curvature(m, 5.0)], [90] * 8)
        self.assertEqual([round(x) for x in integrated_curvature(m, 10.0)], [90 * 4] * 8)

    def test_signed_integration_cancels_a_saddle_against_its_peaks_but_absolute_does_not(self) -> None:
        m = box(10.0, 10.0, 10.0)
        signed = integrated_curvature(m, 10.0)
        self.assertEqual([round(x) for x in integrated_curvature(m, 10.0, absolute=True)], [round(x) for x in signed])
        # Pull one corner inward through the box: its defect turns negative and the signed disk sum drops.
        dented = Mesh([p if i != 7 else (7.0, 7.0, 7.0) for i, p in enumerate(m.pos)], m.faces)
        self.assertLess(round(integrated_curvature(dented, 10.0)[7]), round(integrated_curvature(dented, 10.0, absolute=True)[7]))

    def test_clusters_merge_adjacent_vertices_and_report_the_peak(self) -> None:
        m = box(10.0, 10.0, 10.0)
        signal = [1.0] * 8
        signal[6] = 5.0
        self.assertEqual(clusters(m, signal, 0.5), [(6, 5.0)])
        self.assertEqual(clusters(m, signal, 2.0), [(6, 5.0)])
        self.assertEqual(clusters(m, signal, 9.0), [])


if __name__ == "__main__":
    unittest.main()
