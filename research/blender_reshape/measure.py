# SPDX-License-Identifier: GPL-3.0-or-later
"""Re-places the original export's seams and faces on each edited re-export by position (003 research R5).

Run: python3 measure.py <export-dir>   (the directory export_models.py wrote; needs no bpy)
Prints one JSON report. Every count in results.md comes from it; export_models.py's printout gives the model's size.
"""

import json
import math
import struct
import sys
from pathlib import Path

Vec = tuple[float, float, float]
Polygon = list[Vec]
Cell = tuple[int, int, int]
WELD_RELATIVE = 1e-6  # 001's weld tolerance, a fraction of the bbox diagonal (openplush_geom::epsilon::WELD_RELATIVE)


class Mesh:
    """Polygons welded by position: one vertex per point within `eps`, edges and faces as vertex-index sets."""

    def __init__(self, eps: float) -> None:
        self.eps = eps
        self.vertices: list[Vec] = []
        self.cells: dict[Cell, list[int]] = {}
        self.edges: set[frozenset[int]] = set()
        self.faces: set[frozenset[int]] = set()

    def cell(self, p: Vec) -> Cell:
        return (math.floor(p[0] / self.eps), math.floor(p[1] / self.eps), math.floor(p[2] / self.eps))

    def find(self, p: Vec) -> int | None:
        """The vertex within `eps` of `p`, or None; raises if several are, since welding should have merged them."""
        cx, cy, cz = self.cell(p)
        near = sorted(
            i
            for dx in (-1, 0, 1)
            for dy in (-1, 0, 1)
            for dz in (-1, 0, 1)
            for i in self.cells.get((cx + dx, cy + dy, cz + dz), [])
            if math.dist(self.vertices[i], p) <= self.eps
        )
        if len(near) > 1:
            raise ValueError(f"{p} is within the weld tolerance of {len(near)} vertices")
        return near[0] if near else None

    def add(self, p: Vec) -> int:
        found = self.find(p)
        if found is not None:
            return found
        self.vertices.append(p)
        self.cells.setdefault(self.cell(p), []).append(len(self.vertices) - 1)
        return len(self.vertices) - 1


def weld(polygons: list[Polygon], eps: float) -> Mesh:
    mesh = Mesh(eps)
    for polygon in polygons:
        ids = [mesh.add(p) for p in polygon]
        mesh.faces.add(frozenset(ids))
        mesh.edges.update(frozenset((a, b)) for a, b in zip(ids, ids[1:] + ids[:1]))
    return mesh


def tolerance(polygons: list[Polygon]) -> float:
    points = [p for polygon in polygons for p in polygon]
    low = [min(p[k] for p in points) for k in range(3)]
    high = [max(p[k] for p in points) for k in range(3)]
    diagonal = math.dist(low, high)
    if diagonal == 0.0:
        raise ValueError("every point coincides, so the model has no size to scale the tolerance by")
    return WELD_RELATIVE * diagonal


def kept_positions(old: list[Polygon], new: list[Polygon]) -> int:
    """How many of the old export's distinct positions appear bit-for-bit in the new one."""
    return len({p for polygon in old for p in polygon} & {p for polygon in new for p in polygon})


def placed_edges(edges: list[tuple[Vec, Vec]], new: Mesh) -> list[bool]:
    """Whether each old edge's endpoints are both vertices of `new`, joined by an edge there."""
    out = []
    for p, q in edges:
        a, b = new.find(p), new.find(q)
        out.append(a is not None and b is not None and frozenset((a, b)) in new.edges)
    return out


def placed_faces(faces: list[Polygon], new: Mesh) -> list[bool]:
    """Whether each old face's corners are all vertices of `new` and form one of its faces."""
    out = []
    for face in faces:
        ids = [new.find(p) for p in face]
        out.append(None not in ids and frozenset(i for i in ids if i is not None) in new.faces)
    return out


def shares(old: list[Polygon], seams: list[tuple[Vec, Vec]], edited: list[Vec], new: list[Polygon]) -> dict[str, int]:
    """Placed counts for seams and faces, split by whether they touch a vertex the edit was applied to."""
    eps = tolerance(old)
    region = weld([[p] for p in edited], eps)
    touches = [any(region.find(p) is not None for p in points) for points in [list(s) for s in seams]]
    face_touches = [any(region.find(p) is not None for p in face) for face in old]
    target = weld(new, tolerance(new))
    seam_ok = placed_edges(seams, target)
    face_ok = placed_faces(old, target)
    out: dict[str, int] = {}
    for name, hits, ok in (("seams", touches, seam_ok), ("faces", face_touches, face_ok)):
        out[f"{name}_away"] = hits.count(False)
        out[f"{name}_away_placed"] = sum(1 for h, o in zip(hits, ok) if not h and o)
        out[f"{name}_edited"] = hits.count(True)
        out[f"{name}_edited_placed"] = sum(1 for h, o in zip(hits, ok) if h and o)
    return out


def y_up(p: object) -> Vec:
    """Blender's Z-up world position as both exporters write it by default: (x, z, -y)."""
    if not (isinstance(p, list) and len(p) == 3 and all(isinstance(c, float | int) for c in p)):
        raise TypeError(f"expected a position, got {p!r}")
    x, y, z = (float(c) for c in p)
    return (x, z, -y)


def as_dict(x: object, what: str) -> dict[str, object]:
    if not isinstance(x, dict):
        raise TypeError(f"{what}: expected a JSON object, got {x!r}")
    return x


def as_list(x: object, what: str) -> list[object]:
    if not isinstance(x, list):
        raise TypeError(f"{what}: expected a JSON array, got {x!r}")
    return x


def as_int(x: object, what: str) -> int:
    if type(x) is not int:
        raise TypeError(f"{what}: expected an integer, got {x!r}")
    return x


def accessor(doc: dict[str, object], blob: bytes, index: object) -> list[tuple[float, ...]]:
    """One tightly packed accessor as rows: f32 VEC3 positions or u16/u32 SCALAR indices."""
    acc = as_dict(as_list(doc["accessors"], "accessors")[as_int(index, "accessor index")], "accessor")
    view = as_dict(as_list(doc["bufferViews"], "bufferViews")[as_int(acc["bufferView"], "bufferView")], "bufferView")
    width = {"SCALAR": 1, "VEC3": 3}[str(acc["type"])]
    fmt, size = {5126: ("f", 4), 5125: ("I", 4), 5123: ("H", 2)}[as_int(acc["componentType"], "componentType")]
    start = as_int(view.get("byteOffset", 0), "byteOffset") + as_int(acc.get("byteOffset", 0), "byteOffset")
    stride = size * width
    if as_int(view.get("byteStride", stride), "byteStride") != stride:
        raise ValueError("interleaved accessor data is not supported")
    count = as_int(acc["count"], "count")
    return [struct.unpack_from(f"<{width}{fmt}", blob, start + i * stride) for i in range(count)]


def read_glb_polygons(data: bytes) -> list[Polygon]:
    """Every primitive's triangles as corner positions; raises on a malformed container."""
    magic, version, length = struct.unpack_from("<4sII", data, 0)
    js_len, js_type = struct.unpack_from("<I4s", data, 12)
    bin_at = 20 + js_len
    bin_len, bin_type = struct.unpack_from("<I4s", data, bin_at)
    if (magic, version, length, js_type, bin_type) != (b"glTF", 2, len(data), b"JSON", b"BIN\0"):
        raise ValueError("not a two-chunk GLB")
    doc = as_dict(json.loads(data[20:bin_at]), "glTF")
    blob = data[bin_at + 8 : bin_at + 8 + bin_len]
    polygons: list[Polygon] = []
    for mesh in as_list(doc["meshes"], "meshes"):
        for p in as_list(as_dict(mesh, "mesh")["primitives"], "primitives"):
            prim = as_dict(p, "primitive")
            pos = accessor(doc, blob, as_dict(prim["attributes"], "attributes")["POSITION"])
            idx = [int(i[0]) for i in accessor(doc, blob, prim["indices"])]
            polygons += [[(pos[g][0], pos[g][1], pos[g][2]) for g in idx[n : n + 3]] for n in range(0, len(idx), 3)]
    return polygons


def read_obj_polygons(text: str) -> list[Polygon]:
    """Polygons (any corner count) as corner positions; `vt`, `vn`, `s` and `o` are read past."""
    pos: list[Vec] = []
    polygons: list[Polygon] = []
    for line in text.splitlines():
        word, *rest = line.split() or [""]
        if word == "v":
            pos.append((float(rest[0]), float(rest[1]), float(rest[2])))
        elif word == "f":
            polygons.append([pos[int(c.split("/")[0]) - 1] for c in rest])
        elif word not in ("", "#", "mtllib", "o", "vt", "vn", "s"):
            raise ValueError(f"unexpected OBJ line {line!r}")
    return polygons


def read(directory: Path, name: str, fmt: str) -> list[Polygon]:
    path = directory / f"{name}.{fmt}"
    return read_glb_polygons(path.read_bytes()) if fmt == "glb" else read_obj_polygons(path.read_text(encoding="utf-8"))


def measure(directory: Path) -> dict[str, object]:
    record = as_dict(json.loads((directory / "model.json").read_text()), "model.json")
    seams = [(y_up(e[0]), y_up(e[1])) for e in (as_list(x, "seam") for x in as_list(record["seams"], "seams"))]
    edited = [y_up(p) for p in as_list(record["edited"], "edited")]
    report: dict[str, object] = {"blender": record["blender"]}
    for fmt in ("glb", "obj"):
        old = read(directory, "original", fmt)
        for edit in as_list(record["edits"], "edits"):
            new = read(directory, str(edit), fmt)
            report[f"{edit}.{fmt}"] = {**shares(old, seams, edited, new), "positions_kept": kept_positions(old, new),
                                       "positions": len({p for polygon in old for p in polygon})}
    return report


if __name__ == "__main__":
    print(json.dumps(measure(Path(sys.argv[1])), indent=1, sort_keys=True))
