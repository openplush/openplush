"""Oracle tests for the S7 prototype. Run: python3 -m unittest -v (from this directory).

Each test checks the prototype against something independent of it: topology
(Euler characteristic), Gauss-Bonnet, finite differences, or a surface whose
correct flattening is known (a flat disk, a bent plane).
"""

import unittest

import numpy as np

import flatten as fl
import meshes as ms

ALL = ms.corpus() + ms.stress()


def _face_normals(chart):
    v, f = chart.vertices, chart.faces
    n = np.cross(v[f[:, 1]] - v[f[:, 0]], v[f[:, 2]] - v[f[:, 0]])
    return n / np.linalg.norm(n, axis=1)[:, None]


class Topology(unittest.TestCase):
    def test_every_chart_is_a_disk(self):
        for chart in ALL:
            with self.subTest(chart.name):
                v, e, f = fl.euler_counts(chart.faces)
                self.assertEqual(v - e + f, 1, "chi must be 1 for a disk")
                self.assertEqual(len(fl.boundary_loops(chart.faces)), 1, "exactly one boundary loop")

    def test_faces_are_consistently_oriented(self):
        for chart in ALL:
            with self.subTest(chart.name):
                # Consistent orientation: an interior edge is traversed once in each
                # direction, so no directed edge can appear twice.
                directed = {(a, b) for t in chart.faces for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0]))}
                self.assertEqual(len(directed), 3 * len(chart.faces), "no directed edge used twice")


class Validity(unittest.TestCase):
    """The 3D input itself must be a sensible surface, or flattening results mean nothing.

    A cap whose polar angle passes 180 degrees wraps past the south pole and folds
    back on itself; flattening that and reporting "folds" measures the input, not the
    method. Independent of the generators: sphere charts are detected from the data.
    """

    def test_sphere_charts_face_outward(self):
        for chart in ALL:
            radii = np.linalg.norm(chart.vertices, axis=1)
            if not np.allclose(radii, 1.0):
                continue
            with self.subTest(chart.name):
                v, f = chart.vertices, chart.faces
                centroid = (v[f[:, 0]] + v[f[:, 1]] + v[f[:, 2]]) / 3
                inverted = int(np.sum(np.einsum("ij,ij->i", _face_normals(chart), centroid) <= 0))
                self.assertEqual(inverted, 0, "faces inverted against the sphere's outward normal")

    def test_no_surface_creases_back_on_itself(self):
        for chart in ALL:
            with self.subTest(chart.name):
                normals = _face_normals(chart)
                edge_faces = {}
                for t, tri in enumerate(chart.faces):
                    for a, b in ((tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])):
                        edge_faces.setdefault(tuple(sorted((int(a), int(b)))), []).append(t)
                pairs = np.array([fs for fs in edge_faces.values() if len(fs) == 2])
                worst = np.einsum("ij,ij->i", normals[pairs[:, 0]], normals[pairs[:, 1]]).min()
                self.assertGreater(worst, -0.5, "adjacent faces fold over each other")


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


    def test_touching_polygon_is_not_simple(self):
        # A vertex lying exactly on another edge is a self-touch: not cuttable.
        touching = np.array([[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [2.0, 0.0], [0.0, 3.0]])
        self.assertFalse(fl.polygon_is_simple(touching))

    def test_clockwise_polygon_is_not_accepted(self):
        square = np.array([[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]])
        self.assertFalse(fl.polygon_is_simple(square))
        self.assertTrue(fl.polygon_is_simple(square[::-1]))


class Roots(unittest.TestCase):
    def test_nearly_linear_quadratic_keeps_its_small_root(self):
        # 1e-16 t^2 - 2 t + 1 = 0 has its small root at ~0.5. The textbook formula
        # loses it to cancellation and returns ~0 or a huge value.
        t = fl.smallest_positive_root(np.array([1e-16]), np.array([-2.0]), np.array([1.0]))
        self.assertAlmostEqual(float(t[0]), 0.5, places=12)

    def test_roots_match_numpy(self):
        rng = np.random.default_rng(11)
        a, b, c = rng.normal(size=(3, 200))
        got = fl.smallest_positive_root(a, b, c)
        for i in range(200):
            r = np.roots([a[i], b[i], c[i]])
            r = r[(np.abs(r.imag) < 1e-12) & (r.real > 0)].real
            want = r.min() if len(r) else np.inf
            self.assertTrue(np.isclose(got[i], want, rtol=1e-9) or (np.isinf(got[i]) and np.isinf(want)), i)


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
        self.assertIn(r.pinned.termination, ("gtol", "stalled"), "optimizer must converge, not run out")
        self.assertAlmostEqual(r.pinned.sd_mean, 4.0, places=9)
        self.assertLess(r.free.boundary_len_err_max, 1e-6)

    def test_solver_reports_how_it_stopped(self):
        r = fl.run_chart(ms.sphere_cap(90, rings=8))
        for stats in (r.pinned, r.free):
            self.assertIn(stats.termination, ("gtol", "stalled"))
            self.assertTrue(np.isfinite(stats.grad_inf))
        self.assertEqual(r.free.status, "ok")

    def test_min_corner_angle_matches_an_independent_computation(self):
        chart = ms.sphere_cap(175, rings=8, peak=True, lobes=((4, 0.16, 0.0),))
        v, f = chart.vertices, chart.faces
        worst = np.pi
        for tri in f:
            for k in range(3):
                a, b, c = v[tri[k]], v[tri[(k + 1) % 3]], v[tri[(k + 2) % 3]]
                u, w = b - a, c - a
                worst = min(worst, np.arccos(np.clip(u @ w / np.linalg.norm(u) / np.linalg.norm(w), -1, 1)))
        self.assertAlmostEqual(fl.run_chart(chart).min_angle_deg, np.degrees(worst), places=6)

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
