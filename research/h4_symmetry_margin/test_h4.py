"""Oracles for the H4 spike: the scorer, the candidate search, the tie models, and the decision rule.

Run: python3 -m unittest -v   (from this directory)
Expected values are derived by hand or from the committed fixtures, not from the sweep's own output.
"""

import math
import unittest
from pathlib import Path

import variants
from mirror import Mirror, Plane, Vec, closest_on_triangle, decide, jacobi_eigenvectors, read_glb_mesh

FIXTURES = Path(__file__).resolve().parents[2] / "fixtures" / "meshes"
X: Vec = (1.0, 0.0, 0.0)
Y: Vec = (0.0, 1.0, 0.0)
Z: Vec = (0.0, 0.0, 1.0)
FLOOR = 0.0003  # an exact plane's score on this tessellation, from the centroid's triangulation drift


def mirrors_vertices(pos: list[Vec], axis: int) -> bool:
    """Whether every vertex's reflection in the coordinate plane is a vertex, to the generator's 6 decimals."""
    vertices = {tuple(round(c, 6) for c in p) for p in pos}
    for p in pos:
        q = list(p)
        q[axis] = -q[axis]
        if tuple(round(c, 6) for c in q) not in vertices:
            return False
    return True


class ClosestPoint(unittest.TestCase):
    """Prevents a distance that is right in the triangle's interior but wrong in a vertex or edge region."""

    def test_each_voronoi_region(self) -> None:
        a, b, c = (0.0, 0.0, 0.0), (4.0, 0.0, 0.0), (0.0, 4.0, 0.0)
        self.assertEqual(closest_on_triangle((-1.0, -1.0, 5.0), a, b, c), a)
        self.assertEqual(closest_on_triangle((6.0, -1.0, 0.0), a, b, c), b)
        self.assertEqual(closest_on_triangle((-1.0, 6.0, 0.0), a, b, c), c)
        self.assertEqual(closest_on_triangle((2.0, -3.0, 1.0), a, b, c), (2.0, 0.0, 0.0))
        self.assertEqual(closest_on_triangle((-3.0, 2.0, 1.0), a, b, c), (0.0, 2.0, 0.0))
        self.assertAlmostEqual(math.dist(closest_on_triangle((4.0, 4.0, 0.0), a, b, c), (2.0, 2.0, 0.0)), 0.0)
        self.assertEqual(closest_on_triangle((1.0, 1.0, 7.0), a, b, c), (1.0, 1.0, 0.0))


class Pca(unittest.TestCase):
    """Prevents a principal-axis start that is off-axis on a shape whose axes are the coordinate axes."""

    def test_diagonal_matrix_gives_the_axes_largest_first(self) -> None:
        vectors = jacobi_eigenvectors([[2.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 0.0, 3.0]])
        self.assertEqual([tuple(abs(c) for c in v) for v in vectors], [Y, Z, X])

    def test_rotated_matrix_is_diagonalised(self) -> None:
        s = math.sqrt(0.5)
        m = [[3.5, 1.5, 0.0], [1.5, 3.5, 0.0], [0.0, 0.0, 1.0]]  # eigenvalues 5, 2, 1
        first = jacobi_eigenvectors(m)[0]
        self.assertAlmostEqual(abs(first[0] * s + first[1] * s), 1.0, places=9)


class ExactPlanes(unittest.TestCase):
    """Prevents a scorer that reads near zero everywhere, which would make every tie and every margin vacuous."""

    def test_spheroid_axis_planes_are_exact_and_an_oblique_plane_is_not(self) -> None:
        m = Mirror(*variants.spheroid((40.0, 55.0, 45.0)))
        for n in (X, Y, Z):
            self.assertLess(m.score(n), FLOOR)
        self.assertGreater(m.score((math.sqrt(0.5), math.sqrt(0.5), 0.0)), 0.02)

    def test_offset_is_part_of_the_plane(self) -> None:
        m = Mirror(*variants.spheroid((40.0, 55.0, 45.0)))
        self.assertGreater(m.score(X, 5.0), 0.02)


class TieModels(unittest.TestCase):
    """Prevents a tie family whose second plane is broken by something other than the snout."""

    def test_upright_head_mirrors_in_x_and_y_but_not_z(self) -> None:
        pos, _ = variants.two_planes(0.0, 0.0)
        self.assertTrue(mirrors_vertices(pos, 0) and mirrors_vertices(pos, 1))
        self.assertFalse(mirrors_vertices(pos, 2), "the ears are on top")

    def test_lump_breaks_only_x_and_snout_breaks_only_y(self) -> None:
        self.assertTrue(mirrors_vertices(variants.two_planes(2.0, 0.0)[0], 1))
        self.assertFalse(mirrors_vertices(variants.two_planes(2.0, 0.0)[0], 0))
        self.assertTrue(mirrors_vertices(variants.two_planes(0.0, 3.0)[0], 0))
        self.assertFalse(mirrors_vertices(variants.two_planes(0.0, 3.0)[0], 1))

    def test_jitter_is_seeded_and_bounded(self) -> None:
        base = variants.two_planes(0.0, 0.0)
        once, twice = variants.jittered(base, 0.5, 7), variants.jittered(base, 0.5, 7)
        self.assertEqual(once, twice)
        self.assertLess(max(math.dist(p, q) for p, q in zip(base[0], once[0])), 5.0)
        self.assertNotEqual(variants.jittered(base, 0.5, 8), once)


class Fixtures(unittest.TestCase):
    """Prevents a candidate search that disagrees with the H002 oracle on the committed heads."""

    def test_near_symmetric_best_is_x_0_and_the_second_does_not_fit(self) -> None:
        cands = Mirror(*read_glb_mesh(FIXTURES / "near-symmetric.glb")).candidates()
        self.assertGreater(abs(cands[0][1][0]), math.cos(math.radians(2.0)))
        self.assertTrue(0.002 < cands[0][0] < 0.005)
        self.assertGreater(cands[1][0], 0.01)

    def test_asymmetric_has_no_fitting_plane(self) -> None:
        cands = Mirror(*read_glb_mesh(FIXTURES / "asymmetric.glb")).candidates()
        self.assertGreater(cands[0][0], 0.01)


class Decision(unittest.TestCase):
    """Prevents the rule drifting from H-006: none when nothing fits, none when two fit too closely, else the best."""

    def plane(self, score: float, n: Vec) -> Plane:
        return (score, n, 0.0)

    def test_nothing_fits(self) -> None:
        self.assertEqual(decide([], 0.01, 0.003)[0], "none: no plane fits")
        self.assertEqual(decide([self.plane(0.02, X)], 0.01, 0.003)[0], "none: no plane fits")

    def test_two_fitting_planes_within_the_margin_are_ambiguous(self) -> None:
        self.assertEqual(decide([self.plane(0.002, X), self.plane(0.004, Y)], 0.01, 0.003)[0], "none: ambiguous")
        self.assertEqual(decide([self.plane(0.0, X), self.plane(0.0, Y)], 0.01, 0.0)[0], "none: ambiguous")

    def test_a_clear_gap_or_a_second_that_does_not_fit_gives_the_best(self) -> None:
        self.assertEqual(decide([self.plane(0.002, X), self.plane(0.006, Y)], 0.01, 0.003)[0], "plane")
        self.assertEqual(decide([self.plane(0.002, X), self.plane(0.011, Y)], 0.01, 0.01)[0], "plane")

    def test_margin_equal_to_accept_means_ambiguous_whenever_two_fit(self) -> None:
        self.assertEqual(decide([self.plane(0.001, X), self.plane(0.0099, Y)], 0.01, 0.01)[0], "none: ambiguous")


if __name__ == "__main__":
    unittest.main()
