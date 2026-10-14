"""Oracle tests for the S7 prototype. Run: python3 -m unittest -v (from this directory).

Each test checks the prototype against something independent of it: topology
(Euler characteristic), Gauss-Bonnet, finite differences, or a surface whose
correct flattening is known (a flat disk, a bent plane).
"""

import unittest

import numpy as np

import flatten as fl
import meshes as ms


class Topology(unittest.TestCase):
    def test_every_chart_is_a_disk(self):
        for chart in ms.corpus():
            with self.subTest(chart.name):
                v, e, f = fl.euler_counts(chart.faces)
                self.assertEqual(v - e + f, 1, "chi must be 1 for a disk")
                self.assertEqual(len(fl.boundary_loops(chart.faces)), 1, "exactly one boundary loop")

    def test_faces_are_consistently_oriented(self):
        for chart in ms.corpus():
            with self.subTest(chart.name):
                # Consistent orientation: an interior edge is traversed once in each
                # direction, so no directed edge can appear twice.
                directed = {(a, b) for t in chart.faces for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0]))}
                self.assertEqual(len(directed), 3 * len(chart.faces), "no directed edge used twice")


class Curvature(unittest.TestCase):
    def test_gauss_bonnet(self):
        # Total geodesic curvature of the boundary plus total interior Gaussian
        # curvature is 2*pi for any disk, discretely and exactly.
        for chart in ms.corpus():
            with self.subTest(chart.name):
                geo = fl.Geometry(chart.vertices, chart.faces)
                total = geo.kappa_boundary.sum() + geo.gauss_interior.sum()
                self.assertAlmostEqual(total, 2 * np.pi, places=9)

    def test_bff_boundary_curvature_is_flat_gauss_bonnet(self):
        # After moving interior curvature to the boundary, the boundary alone turns 2*pi.
        for chart in ms.corpus():
            with self.subTest(chart.name):
                geo = fl.Geometry(chart.vertices, chart.faces)
                self.assertAlmostEqual(fl.bff_boundary_curvature(geo).sum(), 2 * np.pi, places=8)

    def test_flat_disk_keeps_its_boundary_curvature(self):
        chart = ms.flat_disk()
        geo = fl.Geometry(chart.vertices, chart.faces)
        np.testing.assert_allclose(fl.bff_boundary_curvature(geo), geo.kappa_boundary, atol=1e-10)


class Closure(unittest.TestCase):
    def test_closing_keeps_lengths_exact_and_closes(self):
        rng = np.random.default_rng(7)
        n = 40
        lengths = rng.uniform(0.5, 1.5, n)
        target = np.full(n, 2 * np.pi / n) + rng.normal(0, 0.05, n)
        target *= 2 * np.pi / target.sum()
        result = fl.close_polygon(lengths, target)
        self.assertTrue(result.success)
        pts = fl.polygon_points(lengths, result.kappa)
        self.assertLess(np.linalg.norm(pts[-1] - pts[0]) / lengths.sum(), 1e-9)
        np.testing.assert_allclose(np.linalg.norm(np.diff(pts, axis=0), axis=1), lengths, rtol=1e-12)

    def test_already_closed_polygon_is_unchanged(self):
        n = 30
        lengths = np.ones(n)
        target = np.full(n, 2 * np.pi / n)
        result = fl.close_polygon(lengths, target)
        np.testing.assert_allclose(result.kappa, target, atol=1e-9)


class Energy(unittest.TestCase):
    def test_gradient_matches_finite_differences(self):
        chart = ms.sphere_cap(60, rings=4)
        geo = fl.Geometry(chart.vertices, chart.faces)
        x = fl.tutte_free_init(geo)
        rng = np.random.default_rng(3)
        x = x + rng.normal(0, 1e-3, x.shape)
        _, g = fl.sd_energy(geo, x, with_grad=True)
        d = rng.normal(0, 1, x.shape)
        h = 1e-6
        e_plus, _ = fl.sd_energy(geo, x + h * d)
        e_minus, _ = fl.sd_energy(geo, x - h * d)
        self.assertAlmostEqual((e_plus - e_minus) / (2 * h), float((g * d).sum()), places=5)

    def test_isometry_has_minimum_energy(self):
        chart = ms.flat_disk(rings=6)
        geo = fl.Geometry(chart.vertices, chart.faces)
        e, _ = fl.sd_energy(geo, chart.vertices[:, :2])
        self.assertAlmostEqual(e, 4.0, places=12)


class Oracles(unittest.TestCase):
    def test_flat_disk_flattens_to_itself(self):
        r = fl.run_chart(ms.flat_disk(rings=8))
        self.assertTrue(r.pinned.injective)
        self.assertAlmostEqual(r.pinned.sd_mean, 4.0, places=9)
        self.assertLess(r.free.boundary_len_err_max, 1e-6)

    def test_bent_plane_flattens_almost_isometrically(self):
        # A cylinder patch is developable. The discrete mesh is only nearly so
        # (chords), hence the loose tolerance.
        r = fl.run_chart(ms.cylinder_patch(rings=8))
        self.assertTrue(r.pinned.injective)
        self.assertLess(r.pinned.sd_mean, 4.01)
        self.assertLess(r.pinned.stretch_max, 1.03)

    def test_pinned_boundary_lengths_are_exact(self):
        r = fl.run_chart(ms.sphere_cap(90, rings=8))
        self.assertLess(r.pinned.boundary_len_err_max, 1e-9)

    def test_optimizer_never_introduces_a_flip(self):
        for chart in (ms.sphere_cap(90, rings=8), ms.saddle(rings=8)):
            with self.subTest(chart.name):
                r = fl.run_chart(chart)
                if r.pinned.init_flips == 0:
                    self.assertEqual(r.pinned.flips, 0)
                self.assertEqual(r.free.flips, 0)


if __name__ == "__main__":
    unittest.main()
