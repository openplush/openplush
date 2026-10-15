"""Oracle tests for the fixture loader and virtual cut. Run: python3 -m unittest -v (from this directory).

The expected chart sizes are derived by hand from the generator's construction
(research/fixtures_gen/README.md), not from the loader. Topology is recounted here
from the face lists alone, so a wrong cut cannot vouch for itself.
"""

import json
import tempfile
import unittest
from collections import Counter
from pathlib import Path

import numpy as np

import corpus
import meshes as ms

type Edge = tuple[int, int]

# (faces, vertices, boundary edges) per chart, in the loader's order (by smallest face index).
# Sphere: 12 segments x 8 bands, cut on every other meridian. A gore holds 3 meridians x 7 rings
# plus both poles = 23 vertices, 28 faces, and 2 meridians x 8 edges of boundary.
# Capsule: a cap is a pole + 3 rings of 12 = 37 vertices, 60 faces, a 12-edge rim. The tube is
# 4 rings of 12 plus 4 duplicated slit vertices = 52, 72 faces, 12 + 12 + 3 + 3 boundary edges.
# Cube: 56 grid vertices; the 7-edge cut tree duplicates its 14 edge-interior vertices once and
# its corners (tree degree - 1) times = 6 more, so 76 vertices; 108 faces; 2 x 21 boundary edges.
EXPECTED = {
    "sphere": [(28, 23, 16)] * 6,
    "capsule": [(60, 37, 12), (72, 52, 30), (60, 37, 12)],
    "cube": [(108, 76, 42)],
}


def undirected(faces: np.ndarray) -> Counter[Edge]:
    return Counter((min(int(a), int(b)), max(int(a), int(b))) for t in faces for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])))


def boundary_loop_count(faces: np.ndarray) -> int:
    """Walk boundary edges by successor; raises if a vertex has two outgoing boundary edges."""
    uses = undirected(faces)
    succ: dict[int, int] = {}
    for t in faces:
        for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            if uses[(min(int(a), int(b)), max(int(a), int(b)))] == 1:
                if int(a) in succ:
                    raise AssertionError("boundary vertex with two outgoing edges")
                succ[int(a)] = int(b)
    loops, seen = 0, set[int]()
    for start in sorted(succ):
        loops += start not in seen
        while start not in seen:
            seen.add(start)
            start = succ[start]
    return loops


class LoadsTheCorpus(unittest.TestCase):
    """Prevents measuring the flattener on charts that are not the fixture's charts."""

    def test_chart_sizes_match_the_hand_count(self) -> None:
        for name, want in EXPECTED.items():
            with self.subTest(name):
                got = [(len(c.chart.faces), len(c.chart.vertices), sum(n == 1 for n in undirected(c.chart.faces).values()))
                       for c in corpus.load(name)]
                self.assertEqual(got, want)

    def test_every_chart_is_a_disk_counted_independently(self) -> None:
        for name in EXPECTED:
            for c in corpus.load(name):
                with self.subTest(c.chart.name):
                    f = c.chart.faces
                    v, e = len(np.unique(f)), len(undirected(f))
                    self.assertEqual(v, len(c.chart.vertices), "no unused vertex")
                    self.assertEqual(v - e + len(f), 1, "chi = 1")
                    self.assertEqual(boundary_loop_count(f), 1, "one boundary loop")
                    directed = {(a, b) for t in f for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0]))}
                    self.assertEqual(len(directed), 3 * len(f), "orientation kept consistent")

    def test_cut_keeps_geometry_and_partitions_faces(self) -> None:
        for name in EXPECTED:
            with self.subTest(name):
                pos, faces = corpus.read_obj(corpus.FIXTURES / "meshes" / f"{name}.obj")
                charts = corpus.load(name)
                ids = np.concatenate([c.face_ids for c in charts])
                self.assertEqual(sorted(ids.tolist()), list(range(len(faces))), "each face in exactly one chart")
                for c in charts:
                    np.testing.assert_array_equal(c.chart.vertices, pos[c.to_global])
                    np.testing.assert_array_equal(c.to_global[c.chart.faces], faces[c.face_ids])

    def test_each_seam_edge_has_two_sides_and_nothing_else_is_cut(self) -> None:
        for name in EXPECTED:
            with self.subTest(name):
                doc: object = json.loads((corpus.FIXTURES / "seams" / f"{name}.json").read_text(encoding="utf-8"))
                seams = set(corpus.parse_seam_set(doc).edges)
                sides: Counter[Edge] = Counter()
                for c in corpus.load(name):
                    for (a, b), n in undirected(c.chart.faces).items():
                        if n == 1:
                            g = c.to_global
                            sides[(min(int(g[a]), int(g[b])), max(int(g[a]), int(g[b])))] += 1
                self.assertEqual(set(sides), seams)
                self.assertEqual(set(sides.values()), {2})


class DartTips(unittest.TestCase):
    """Prevents a free seam endpoint being split, which would turn a slit into a separated piece."""

    def test_slit_tip_stays_single(self) -> None:
        disk = ms.flat_disk(rings=3)
        # Ray from the centre (vertex 0) along angle 0: vertex 1 + sum of ring sizes before ring k.
        ray = [0, 1, 7, 19]
        cut = corpus.cut(disk.vertices, disk.faces, {(min(a, b), max(a, b)) for a, b in zip(ray, ray[1:])}, "slit")
        self.assertEqual(len(cut), 1)
        c = cut[0]
        # The two inner ray vertices and the rim vertex split; the centre (the tip) does not.
        self.assertEqual(len(c.chart.vertices), len(disk.vertices) + 3)
        self.assertEqual(Counter(c.to_global.tolist())[0], 1)
        self.assertEqual(boundary_loop_count(c.chart.faces), 1)


class FailsLoudly(unittest.TestCase):
    """Prevents bad input quietly becoming a default or a wrong chart."""

    def test_annulus_is_not_a_disk(self) -> None:
        pos, faces = corpus.read_obj(corpus.FIXTURES / "meshes" / "capsule.obj")
        doc: object = json.loads((corpus.FIXTURES / "seams" / "capsule.json").read_text(encoding="utf-8"))
        seams = set(corpus.parse_seam_set(doc).edges)
        # Dropping one edge of the lengthwise slit leaves the tube an annulus.
        with self.assertRaises(corpus.NotADisk):
            corpus.cut(pos, faces, seams - {(37, 49)}, "capsule")

    def test_uncut_closed_mesh_is_not_a_disk(self) -> None:
        pos, faces = corpus.read_obj(corpus.FIXTURES / "meshes" / "sphere.obj")
        with self.assertRaises(corpus.NotADisk):
            corpus.cut(pos, faces, set(), "sphere")

    def test_seam_that_is_not_a_mesh_edge_is_rejected(self) -> None:
        pos, faces = corpus.read_obj(corpus.FIXTURES / "meshes" / "cube.obj")
        with self.assertRaises(ValueError):
            corpus.cut(pos, faces, {(0, 55)}, "cube")

    def test_seam_json_rejects_wrong_types(self) -> None:
        def doc(edge: list[object], version: object) -> dict[str, object]:
            head: dict[str, object] = {"format": "openplush-seam-set", "version": version, "mesh": "m", "vertexIndexBase": 0}
            return head | {"seams": [{"edge": edge, "kind": "cut"}]}

        self.assertEqual(corpus.parse_seam_set(doc([0, 1], 1)).edges, [(0, 1)])
        for bad in (doc([0.0, 1], 1), doc([True, 1], 1), doc([0, 1], 1.0), doc([0, 1, 2], 1), "not an object"):
            with self.subTest(bad), self.assertRaises((TypeError, ValueError)):
                corpus.parse_seam_set(bad)

    def test_obj_rejects_unknown_lines_and_bad_indices(self) -> None:
        for text in ("v 0 0 0\nvt 0 0\n", "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 4\n", "v 0 0 nan\n", "v 0 0 0\nf 1/1 2 3\n"):
            with self.subTest(text), tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / "bad.obj"
                path.write_text(text, encoding="utf-8")
                with self.assertRaises(ValueError):
                    corpus.read_obj(path)


class Refinement(unittest.TestCase):
    """Prevents a refinement sweep that changes topology or breaks seam parity."""

    def test_subdivision_quadruples_faces_and_keeps_disks(self) -> None:
        for name in EXPECTED:
            with self.subTest(name):
                base = corpus.load(name)
                fine = corpus.load(name, levels=1)
                self.assertEqual([4 * len(c.chart.faces) for c in base], [len(c.chart.faces) for c in fine])
                for c in fine:
                    self.assertEqual(boundary_loop_count(c.chart.faces), 1)

    def test_subdivided_sphere_stays_on_the_sphere(self) -> None:
        for c in corpus.load("sphere", levels=2):
            np.testing.assert_allclose(np.linalg.norm(c.chart.vertices, axis=1), 50.0, rtol=1e-12)


class Jitter(unittest.TestCase):
    """Prevents the asymmetric variants from being folded input, irreproducible, or unequal across a seam.

    Every fixture chart is symmetric or developable, so its outline closes for free; the
    jittered copies are what exercise closure. They are only meaningful if still valid.
    """

    def test_jitter_is_seeded_and_bounded(self) -> None:
        for name in EXPECTED:
            with self.subTest(name):
                base, a, b = (corpus.load(name, jitter=j) for j in (0.0, corpus.JITTER, corpus.JITTER))
                for c0, c1, c2 in zip(base, a, b, strict=True):
                    np.testing.assert_array_equal(c1.chart.vertices, c2.chart.vertices)
                    np.testing.assert_array_equal(c0.chart.faces, c1.chart.faces)
                    moved = np.linalg.norm(c1.chart.vertices - c0.chart.vertices, axis=1)
                    self.assertGreater(moved.max(), 0.0)
                    # A vertex moves at most JITTER x its shortest edge, so never more than JITTER x the longest.
                    edges = c0.chart.vertices[c0.chart.faces] - c0.chart.vertices[np.roll(c0.chart.faces, -1, 1)]
                    self.assertLessEqual(moved.max(), corpus.JITTER * np.linalg.norm(edges, axis=2).max())

    def test_copies_of_one_mesh_vertex_move_together(self) -> None:
        for name in EXPECTED:
            with self.subTest(name):
                pos: dict[int, np.ndarray] = {}
                for c in corpus.load(name, jitter=corpus.JITTER):
                    for local, g in enumerate(c.to_global.tolist()):
                        np.testing.assert_array_equal(pos.setdefault(g, c.chart.vertices[local]), c.chart.vertices[local])

    def test_jittered_surfaces_do_not_crease_back(self) -> None:
        for name in EXPECTED:
            for levels in (0, 1):
                for c in corpus.load(name, levels=levels, jitter=corpus.JITTER):
                    with self.subTest(c.chart.name):
                        v, f = c.chart.vertices, c.chart.faces
                        n = np.cross(v[f[:, 1]] - v[f[:, 0]], v[f[:, 2]] - v[f[:, 0]])
                        n /= np.linalg.norm(n, axis=1)[:, None]
                        pairs: dict[Edge, list[int]] = {}
                        for t, tri in enumerate(f.tolist()):
                            for a, b in ((tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])):
                                pairs.setdefault((min(a, b), max(a, b)), []).append(t)
                        two = np.array([p for p in pairs.values() if len(p) == 2])
                        self.assertGreater(np.einsum("ij,ij->i", n[two[:, 0]], n[two[:, 1]]).min(), 0.0)


if __name__ == "__main__":
    unittest.main()
