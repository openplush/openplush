"""Oracle tests for the committed fixture corpus. Run: python3 -m unittest -v (from this directory).

The oracles read the committed files from disk and share no code with the generator,
so a generator bug cannot hide itself. Detectors run in the repair order (weld, then
drop duplicates, then topology) so each defect fixture trips exactly one of them.
"""

import json
import math
import tempfile
import unittest
from collections import Counter
from pathlib import Path

FIXTURES = Path(__file__).resolve().parents[2] / "fixtures"
MESHES = ("capsule", "cube", "sphere")
# Every class the repair pipeline detects. Non-finite input fails at import instead, so it is not here.
DEFECTS = ("bowtie-vertex", "duplicate-face", "flipped-normals", "hole", "multi-component")
DEFECTS += ("non-manifold-edge", "self-intersection", "unwelded-split-vertices", "zero-area-face")
SEAM_KINDS = {"cut", "dart", "opening", "fabricBoundary", "mandatory"}

Vec = tuple[float, float, float]
Face = tuple[int, int, int]


def read_obj(path: Path) -> tuple[list[Vec], list[Face]]:
    pos: list[Vec] = []
    faces: list[Face] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        word, *rest = line.split() or [""]
        if word == "v":
            x, y, z = (float(s) for s in rest)
            pos.append((x, y, z))
        elif word == "f":
            a, b, c = (int(s) - 1 for s in rest)
            faces.append((a, b, c))
        elif word not in ("", "#"):
            raise ValueError(f"{path.name}: unexpected OBJ line {line!r}")
    return pos, faces


class UnionFind:
    def __init__(self) -> None:
        self.parent: dict[object, object] = {}

    def find(self, x: object) -> object:
        self.parent.setdefault(x, x)
        while self.parent[x] != x:
            x = self.parent[x]
        return x

    def union(self, a: object, b: object) -> None:
        self.parent[self.find(a)] = self.find(b)


def sub(a: Vec, b: Vec) -> Vec:
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def cross(a: Vec, b: Vec) -> Vec:
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def dot(a: Vec, b: Vec) -> float:
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def segment_hits_triangle(p: Vec, q: Vec, tri: tuple[Vec, Vec, Vec], eps: float = 1e-9) -> bool:
    """Moller-Trumbore, strictly inside both: touching and coplanar contact are not crossings."""
    d, e1, e2 = sub(q, p), sub(tri[1], tri[0]), sub(tri[2], tri[0])
    h = cross(d, e2)
    det = dot(e1, h)
    if abs(det) < eps:
        return False
    s = sub(p, tri[0])
    u = dot(s, h) / det
    qv = cross(s, e1)
    v = dot(d, qv) / det
    t = dot(e2, qv) / det
    return eps < u and eps < v and u + v < 1 - eps and eps < t < 1 - eps


def triangles_cross(s: tuple[Vec, Vec, Vec], t: tuple[Vec, Vec, Vec]) -> bool:
    """A transverse crossing always puts an edge of one triangle through the other's interior."""
    return any(segment_hits_triangle(a[k], a[(k + 1) % 3], b) for a, b in ((s, t), (t, s)) for k in range(3))


def edges_of(f: Face) -> list[tuple[int, int]]:
    return [(f[0], f[1]), (f[1], f[2]), (f[2], f[0])]


def detect(pos: list[Vec], faces: list[Face]) -> dict[str, int]:
    """Count each defect class; zero everywhere means closed, manifold, oriented, and clean."""
    canon: dict[Vec, int] = {}
    welded = [tuple(canon.setdefault(pos[i], i) for i in f) for f in faces]
    unique = list({tuple(sorted(f)): f for f in reversed(welded)}.values())
    lo = [min(p[k] for p in pos) for k in range(3)]
    hi = [max(p[k] for p in pos) for k in range(3)]
    diag2 = sum((h - l) ** 2 for h, l in zip(hi, lo))
    edge_faces: dict[tuple[int, int], set[int]] = {}
    fans, comps = UnionFind(), UnionFind()
    for i, f in enumerate(unique):
        comps.union(f[0], f[1])
        comps.union(f[1], f[2])
        for a, b in edges_of(f):
            edge_faces.setdefault((min(a, b), max(a, b)), set()).add(i)
    for (a, b), fs in edge_faces.items():
        first = min(fs)
        for g in fs:
            fans.union((a, first), (a, g))
            fans.union((b, first), (b, g))
    fan_roots = {(v, fans.find((v, i))) for i, f in enumerate(unique) for v in f}
    tris = [(pos[f[0]], pos[f[1]], pos[f[2]]) for f in unique]
    crossings = sum(
        triangles_cross(tris[i], tris[j])
        for i in range(len(unique))
        for j in range(i + 1, len(unique))
        if not set(unique[i]) & set(unique[j])
    )
    return {
        "bowtie-vertex": len(fan_roots) - len({v for v, _ in fan_roots}),
        "duplicate-face": len(welded) - len(unique),
        # Winding is only defined across a manifold edge; repair orients after splitting the others.
        "flipped-normals": sum(
            n - 1
            for (a, b), n in Counter(e for f in unique for e in edges_of(f)).items()
            if len(edge_faces[(min(a, b), max(a, b))]) <= 2
        ),
        "hole": sum(len(fs) == 1 for fs in edge_faces.values()),
        "multi-component": len({comps.find(v) for f in unique for v in f}) - 1,
        "non-finite": sum(not math.isfinite(c) for p in pos for c in p),
        "non-manifold-edge": sum(len(fs) > 2 for fs in edge_faces.values()),
        "self-intersection": crossings,
        "unwelded-split-vertices": len(pos) - len(canon),
        "zero-area-face": sum(
            dot(n, n) / 4 <= 1e-12 * diag2 * diag2
            for n in (cross(sub(t[1], t[0]), sub(t[2], t[0])) for t in tris)
        ),
    }


def signed_volume(pos: list[Vec], faces: list[Face]) -> float:
    return sum(dot(pos[a], cross(pos[b], pos[c])) for a, b, c in faces) / 6


class CleanMeshes(unittest.TestCase):
    """Prevents a base mesh that is itself defective, which would make every pattern test meaningless."""

    def test_every_mesh_is_closed_manifold_oriented_and_clean(self) -> None:
        for name in MESHES:
            with self.subTest(name):
                pos, faces = read_obj(FIXTURES / "meshes" / f"{name}.obj")
                self.assertEqual({k: v for k, v in detect(pos, faces).items() if v}, {})
                self.assertLessEqual(len(faces), 400, "keep fixtures small")
                edges = {tuple(sorted(e)) for f in faces for e in edges_of(f)}
                self.assertEqual(len(pos) - len(edges) + len(faces), 2, "sphere topology")
                self.assertTrue(set(range(len(pos))) <= {v for f in faces for v in f}, "no unused vertex")
                self.assertGreater(signed_volume(pos, faces), 0, "wound outward")


class DefectCorpus(unittest.TestCase):
    """Prevents a defect class the detector is never measured on, or a fixture blamed for the wrong one."""

    def test_one_fixture_per_defect_class_and_nothing_else(self) -> None:
        present = sorted(p.stem for p in (FIXTURES / "defects").iterdir())
        self.assertEqual(present, sorted(DEFECTS))

    def test_each_fixture_has_exactly_its_own_defect(self) -> None:
        for name in DEFECTS:
            with self.subTest(name):
                found = {k: v for k, v in detect(*read_obj(FIXTURES / "defects" / f"{name}.obj")).items() if v}
                self.assertEqual(list(found), [name])


def chart_topology(faces: list[Face], seams: set[tuple[int, int]]) -> list[tuple[int, int]]:
    """Per chart, (Euler characteristic, boundary loops) after cutting along `seams`."""
    edge_faces: dict[tuple[int, int], list[int]] = {}
    for i, f in enumerate(faces):
        for a, b in edges_of(f):
            edge_faces.setdefault((min(a, b), max(a, b)), []).append(i)
    charts, corners = UnionFind(), UnionFind()
    for (a, b), (f, g) in edge_faces.items():
        if (a, b) not in seams:
            charts.union(f, g)
            corners.union((f, a), (g, a))
            corners.union((f, b), (g, b))
    out: list[tuple[int, int]] = []
    for root in sorted({charts.find(i) for i in range(len(faces))}):
        mine = [i for i in range(len(faces)) if charts.find(i) == root]
        wedges = {corners.find((i, v)) for i in mine for v in faces[i]}
        cut = [(i, a, b) for i in mine for a, b in edges_of(faces[i]) if (min(a, b), max(a, b)) in seams]
        inner = {(min(a, b), max(a, b)) for i in mine for a, b in edges_of(faces[i])} - seams
        succ = {corners.find((i, a)): corners.find((i, b)) for i, a, b in cut}
        if len(succ) != len(cut):
            raise AssertionError("a wedge has two outgoing boundary edges: the cut is not a manifold")
        loops, seen = 0, set()
        for start in succ:
            loops += start not in seen
            while start not in seen:
                seen.add(start)
                start = succ[start]
        out.append((len(wedges) - len(inner) - len(cut) + len(mine), loops))
    return out


class SeamSets(unittest.TestCase):
    """Prevents a reference seam set that names a non-edge or leaves a chart that cannot flatten."""

    def load(self, path: Path) -> tuple[list[Face], set[tuple[int, int]], dict[str, object]]:
        doc = json.loads(path.read_text(encoding="utf-8"))
        self.assertIsInstance(doc, dict)
        self.assertEqual((doc["format"], doc["version"], doc["vertexIndexBase"]), ("openplush-seam-set", 1, 0))
        pos, faces = read_obj(FIXTURES / str(doc["mesh"]))
        pairs = [(s["edge"][0], s["edge"][1]) for s in doc["seams"]]
        self.assertEqual(pairs, sorted(set(pairs)), "sorted, no duplicates")
        self.assertTrue(all(a < b for a, b in pairs), "each pair sorted")
        self.assertTrue({s["kind"] for s in doc["seams"]} <= SEAM_KINDS)
        mesh_edges = {tuple(sorted(e)) for f in faces for e in edges_of(f)}
        self.assertEqual(set(pairs) - mesh_edges, set(), "every seam is a mesh edge")
        return faces, set(pairs), doc

    def test_every_mesh_has_a_seam_set(self) -> None:
        meshes = {json.loads(p.read_text(encoding="utf-8"))["mesh"] for p in (FIXTURES / "seams").glob("*.json")}
        self.assertTrue({f"meshes/{m}.obj" for m in MESHES} <= meshes)

    def test_every_chart_is_a_topological_disk(self) -> None:
        paths = sorted((FIXTURES / "seams").glob("*.json"))
        self.assertGreaterEqual(len(paths), len(MESHES))
        for path in paths:
            with self.subTest(path.name):
                faces, seams, _ = self.load(path)
                topo = chart_topology(faces, seams)
                self.assertGreaterEqual(len(topo), 1)
                self.assertEqual(topo, [(1, 1)] * len(topo), "each chart: chi 1, one boundary loop")

    def test_disk_oracle_rejects_an_uncut_sphere_and_an_annulus(self) -> None:
        faces, seams, _ = self.load(FIXTURES / "seams" / "capsule.json")
        self.assertEqual(chart_topology(faces, set()), [(2, 0)])
        # Dropping a middle edge of the capsule's lengthwise cut leaves the tube an annulus.
        weakened = [chart_topology(faces, seams - {e}) for e in sorted(seams)]
        self.assertIn((0, 2), [c for topo in weakened for c in topo])


class Reproducible(unittest.TestCase):
    """Prevents hand edits to generated files and generator output that drifts between runs."""

    def test_regenerating_reproduces_the_committed_files_byte_for_byte(self) -> None:
        import generate

        with tempfile.TemporaryDirectory() as tmp:
            generate.write_all(Path(tmp))
            made = sorted(p.relative_to(tmp) for p in Path(tmp).rglob("*") if p.is_file())
            self.assertGreater(len(made), 0)
            committed = sorted(
                p.relative_to(FIXTURES)
                for d in ("meshes", "defects", "seams")
                for p in (FIXTURES / d).rglob("*")
                if p.is_file() and p.suffix in (".obj", ".json")
            )
            self.assertEqual(made, committed)
            for rel in made:
                with self.subTest(str(rel)):
                    self.assertEqual((Path(tmp) / rel).read_bytes(), (FIXTURES / rel).read_bytes())


if __name__ == "__main__":
    unittest.main()
