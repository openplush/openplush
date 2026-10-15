"""Load the fixture corpus and cut each mesh into charts along its reference seam set, for spike S7.

Non-shipping research code. The cut follows data-model.md section 4 (ChartMesh, virtual
cut): faces are flood-filled across non-seam edges, and each vertex is split into one copy
per fan of corners joined across non-seam edges. A free seam endpoint (a dart tip) has a
single such fan, so it stays one vertex and the seam becomes a slit. Every chart must be a
topological disk (Constitution III pre-flatten invariant) or the cut raises NotADisk.
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from pathlib import Path

import numpy as np

from meshes import Chart

type Edge = tuple[int, int]

FIXTURES = Path(__file__).resolve().parents[2] / "fixtures"
NAMES = ("sphere", "capsule", "cube")
SEAM_KINDS = ("cut", "dart", "opening", "fabricBoundary", "mandatory")  # contracts/project.schema.json


class NotADisk(ValueError):
    """A chart left by the cut is not connected with one boundary loop and chi = 1."""


@dataclass(frozen=True)
class SeamSet:
    mesh: str
    edges: list[Edge]


@dataclass(frozen=True)
class FixtureChart:
    chart: Chart
    to_global: np.ndarray  # chart vertex -> mesh vertex (provenance)
    face_ids: np.ndarray  # chart face -> mesh face


def read_obj(path: Path) -> tuple[np.ndarray, np.ndarray]:
    """Triangle OBJ with `v x y z` and `f a b c` lines only; returns 0-based faces."""
    pos: list[tuple[float, float, float]] = []
    faces: list[tuple[int, int, int]] = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        words = line.split()
        if not words or words[0].startswith("#"):
            continue
        where = f"{path.name}:{number}"
        if words[0] == "v" and len(words) == 4:
            x, y, z = (float(w) for w in words[1:])
            if not all(math.isfinite(c) for c in (x, y, z)):
                raise ValueError(f"{where}: non-finite coordinate")
            pos.append((x, y, z))
        elif words[0] == "f" and len(words) == 4:
            if not all(w.isdigit() for w in words[1:]):
                raise ValueError(f"{where}: face indices must be plain positive integers: {line!r}")
            a, b, c = (int(w) - 1 for w in words[1:])
            faces.append((a, b, c))
        else:
            raise ValueError(f"{where}: unsupported OBJ line {line!r}")
    if not faces:
        raise ValueError(f"{path.name}: no faces")
    f = np.array(faces, dtype=int)
    if f.min() < 0 or f.max() >= len(pos):
        raise ValueError(f"{path.name}: face index out of range")
    return np.array(pos, dtype=float), f


def _field(doc: object, key: str) -> object:
    if not isinstance(doc, dict):
        raise TypeError(f"expected a JSON object holding {key!r}, got {doc!r}")
    fields: dict[object, object] = doc
    if key not in fields:
        raise KeyError(key)
    return fields[key]


def _int(x: object, what: str) -> int:
    if type(x) is not int:  # JSON 1.0 and true would compare equal to 1
        raise TypeError(f"{what}: expected an integer, got {x!r}")
    return x


def _str(x: object, what: str) -> str:
    if not isinstance(x, str):
        raise TypeError(f"{what}: expected a string, got {x!r}")
    return x


def _list(x: object, what: str) -> list[object]:
    if not isinstance(x, list):
        raise TypeError(f"{what}: expected an array, got {x!r}")
    out: list[object] = x
    return out


def parse_seam_set(doc: object) -> SeamSet:
    """fixtures/README.md seam-set format, version 1; raises on any type or value mismatch.

    Edges are (smaller, larger) pairs, listed in strictly increasing order: sorted, no repeats.
    """
    header = (_str(_field(doc, "format"), "format"), _int(_field(doc, "version"), "version"),
              _int(_field(doc, "vertexIndexBase"), "vertexIndexBase"))
    if header != ("openplush-seam-set", 1, 0):
        raise ValueError(f"unsupported seam-set header {header!r}")
    edges: list[Edge] = []
    for seam in _list(_field(doc, "seams"), "seams"):
        pair = _list(_field(seam, "edge"), "edge")
        if len(pair) != 2:
            raise ValueError(f"an edge is two vertex indices, got {pair!r}")
        a, b = _int(pair[0], "edge index"), _int(pair[1], "edge index")
        if not 0 <= a < b:
            raise ValueError(f"edge {pair!r}: indices must be non-negative, smaller first")
        kind = _str(_field(seam, "kind"), "kind")
        if kind not in SEAM_KINDS:
            raise ValueError(f"edge {pair!r}: unknown seam kind {kind!r}")
        edges.append((a, b))
    if any(e >= f for e, f in zip(edges, edges[1:])):
        raise ValueError("seam edges must be sorted with no repeats")
    return SeamSet(_str(_field(doc, "mesh"), "mesh"), edges)


class _UnionFind:
    def __init__(self, n: int) -> None:
        self.parent = list(range(n))

    def find(self, x: int) -> int:
        while self.parent[x] != x:
            self.parent[x] = self.parent[self.parent[x]]
            x = self.parent[x]
        return x

    def union(self, a: int, b: int) -> None:
        ra, rb = self.find(a), self.find(b)
        if ra != rb:
            self.parent[max(ra, rb)] = min(ra, rb)


def _edge_faces(faces: np.ndarray) -> dict[Edge, list[int]]:
    out: dict[Edge, list[int]] = {}
    for i, t in enumerate(faces.tolist()):
        for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            out.setdefault((min(a, b), max(a, b)), []).append(i)
    return out


def _check_disk(name: str, faces: np.ndarray) -> None:
    edges = {(min(a, b), max(a, b)) for t in faces.tolist() for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0]))}
    chi = len(np.unique(faces)) - len(edges) + len(faces)
    directed = {(a, b) for t in faces.tolist() for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0]))}
    succ: dict[int, int] = {}
    for a, b in sorted(directed):
        if (b, a) not in directed:
            if a in succ:
                raise NotADisk(f"{name}: vertex {a} has two boundary fans")
            succ[a] = b
    loops, seen = 0, set[int]()
    for start in succ:
        loops += start not in seen
        while start not in seen:
            seen.add(start)
            start = succ[start]
    if (chi, loops) != (1, 1):
        raise NotADisk(f"{name}: chi = {chi}, {loops} boundary loop(s); a disk has chi = 1 and one loop")


def cut(pos: np.ndarray, faces: np.ndarray, seams: set[Edge], name: str) -> list[FixtureChart]:
    """Charts in order of their smallest face index, named `name#k`. Raises NotADisk or ValueError."""
    edge_faces = _edge_faces(faces)
    missing = sorted(seams - edge_faces.keys())
    if missing:
        raise ValueError(f"{name}: seam edges {missing[:3]} are not mesh edges")
    if any(len(fs) > 2 for fs in edge_faces.values()):
        raise ValueError(f"{name}: non-manifold edge; the cut needs a repaired mesh")
    m = len(faces)
    charts = _UnionFind(m)
    corners = _UnionFind(3 * m)  # corner k of face i is 3 * i + k
    slot = {(i, int(v)): 3 * i + k for i, t in enumerate(faces) for k, v in enumerate(t)}
    for (a, b), fs in sorted(edge_faces.items()):
        if len(fs) == 2 and (a, b) not in seams:
            p, q = fs
            charts.union(p, q)
            corners.union(slot[(p, a)], slot[(q, a)])
            corners.union(slot[(p, b)], slot[(q, b)])
    members: dict[int, list[int]] = {}
    for i in range(m):
        members.setdefault(charts.find(i), []).append(i)
    out: list[FixtureChart] = []
    for k, ids in enumerate(sorted(members.values(), key=min)):
        local: dict[int, int] = {}
        to_global: list[int] = []
        tris: list[list[int]] = []
        for i in ids:
            tri = []
            for c in range(3):
                root = corners.find(3 * i + c)
                if root not in local:
                    local[root] = len(to_global)
                    to_global.append(int(faces[i, c]))
                tri.append(local[root])
            tris.append(tri)
        label = f"{name}#{k}"
        chart_faces = np.array(tris, dtype=int)
        _check_disk(label, chart_faces)
        g = np.array(to_global, dtype=int)
        out.append(FixtureChart(Chart(label, pos[g], chart_faces, f"fixture {name}"), g, np.array(ids, dtype=int)))
    return out


def load(name: str) -> list[FixtureChart]:
    """fixtures/meshes/<name>.obj cut along fixtures/seams/<name>.json."""
    seam_set = parse_seam_set(json.loads((FIXTURES / "seams" / f"{name}.json").read_text(encoding="utf-8")))
    if seam_set.mesh != f"meshes/{name}.obj":
        raise ValueError(f"seams/{name}.json names {seam_set.mesh!r}, not meshes/{name}.obj")
    pos, faces = read_obj(FIXTURES / seam_set.mesh)
    return cut(pos, faces, set(seam_set.edges), name)
