# SPDX-License-Identifier: GPL-3.0-or-later
"""Oracles for measure.py on hand-built polygons, so the Blender numbers rest on a checked measuring stick.

Run: python3 -m unittest -v   (from this directory; needs no bpy)
"""

import unittest

from measure import Polygon, Vertices, edge_gaps, flip_matches, report, tip_uv_counts

A, B = (0.0, 0.0), (0.5, 0.0)


class EdgeGaps(unittest.TestCase):
    def test_a_cut_that_differs_at_one_endpoint_is_still_a_gap(self) -> None:
        """Prevents a measuring rule that only compares whole edges and would miss a slit that runs into a junction."""
        left = [(0, A), (1, A), (2, A)]
        right = [(1, B), (0, A), (3, A)]
        gaps, boundary = edge_gaps([left, right])
        self.assertEqual(gaps[frozenset((0, 1))], 0.5)
        self.assertEqual(boundary, 4)

    def test_an_uncut_edge_has_no_gap(self) -> None:
        """Prevents a rule that reads every shared edge as a cut."""
        gaps, _ = edge_gaps([[(0, A), (1, A), (2, A)], [(1, A), (0, A), (3, A)]])
        self.assertEqual(gaps[frozenset((0, 1))], 0.0)

    def test_quads_and_triangles_share_edges(self) -> None:
        """Prevents an OBJ reader path that only understands triangles; Blender writes quads."""
        quad = [(0, A), (1, A), (2, A), (3, A)]
        tri = [(1, A), (0, A), (4, A)]
        gaps, boundary = edge_gaps([quad, tri])
        self.assertIn(frozenset((0, 1)), gaps)
        self.assertEqual(boundary, 5)

    def test_an_edge_with_three_faces_is_refused(self) -> None:
        """Prevents a non-manifold export from being measured as if it were clean."""
        faces = [[(0, A), (1, A), (n, A)] for n in (2, 3, 4)]
        with self.assertRaises(ValueError):
            edge_gaps(faces)


class Tips(unittest.TestCase):
    def test_a_tip_carries_one_uv_and_a_split_vertex_more(self) -> None:
        """Prevents a slit test that cannot tell a closed tip from an opened one."""
        polys = [[(0, A), (1, A), (2, A)], [(1, B), (0, A), (3, A)]]
        self.assertEqual(tip_uv_counts(polys, [0, 1]), [1, 2])


class Matching(unittest.TestCase):
    def test_a_position_matches_exactly_one_vertex_or_raises(self) -> None:
        """Prevents a corner being named by the wrong vertex, or silently by none."""
        verts = Vertices([(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)])
        self.assertEqual(verts.index((10.0000004, 0.0, 0.0)), 1)
        with self.assertRaises(ValueError):
            verts.index((5.0, 0.0, 0.0))
        with self.assertRaises(ValueError):
            Vertices([(0.0, 0.0, 0.0), (0.0001, 0.0, 0.0)]).index((0.0, 0.0, 0.0))



def tetrahedron(gap: float) -> list[Polygon]:
    """A closed tetrahedron whose face (0, 1, 2) alone carries vertex 1 at a UV `gap` away from the others'."""
    moved = (A[0] + gap, A[1])
    faces = [(0, 1, 2), (0, 3, 1), (1, 3, 2), (0, 2, 3)]
    return [[(v, moved if (f == (0, 1, 2) and v == 1) else A) for v in f] for f in faces]


class Report(unittest.TestCase):
    def test_gaps_over_the_threshold_are_cuts_and_their_free_ends_are_tips(self) -> None:
        """Prevents a cut count or tip count that a wrong threshold or a wrong degree test would leave unchanged."""
        r = report(tetrahedron(0.5), None, [])
        self.assertEqual((r["cuts"], r["dead_end_tips"], r["boundary_edges"]), (2, 2, 0))

    def test_a_gap_under_the_threshold_is_no_cut(self) -> None:
        """Prevents a threshold an order of magnitude off: 2e-5 must cut and 5e-6 must not."""
        self.assertEqual(report(tetrahedron(2e-5), None, [])["cuts"], 2)
        self.assertEqual(report(tetrahedron(5e-6), None, [])["cuts"], 0)

    def test_marked_seams_are_compared_edge_for_edge(self) -> None:
        """Prevents a seam comparison that counts a missing or an extra cut as a match."""
        r = report(tetrahedron(0.5), [frozenset((0, 1)), frozenset((2, 3))], [])
        self.assertEqual((r["marked_not_cut"], r["cut_not_marked"]), (1, 1))


class Flip(unittest.TestCase):
    def test_a_flipped_back_uv_must_match_within_a_millionth(self) -> None:
        """Prevents a V-flip check so loose that any UV passes."""
        obj = [[(0, (0.25, 0.75))]]
        self.assertTrue(flip_matches([[(0, (0.25, 0.75))]], obj))
        self.assertFalse(flip_matches([[(0, (0.25, 0.751))]], obj))
        self.assertFalse(flip_matches([[(1, (0.25, 0.75))]], obj))


if __name__ == "__main__":
    unittest.main()
