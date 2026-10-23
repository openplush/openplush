# SPDX-License-Identifier: GPL-3.0-or-later
"""Measures the UV cuts in Blender's exports against the seams marked in Blender.

Run: python3 measure.py <export-dir>   (the directory export_models.py wrote; needs no bpy)
Prints one JSON report. Every number in results.md comes from it.
"""

import json
import struct
import sys
from pathlib import Path

Vec = tuple[float, float, float]
UV = tuple[float, float]
Key = int  # the index of the Blender vertex an exported corner sits on
Polygon = list[tuple[Key, UV]]
UV_CUT = 1e-5
MATCH_TOL = 1e-3  # mm; f32 and 6-decimal exports of the same vertex differ by far less, distinct vertices by far more


class Vertices:
    """Blender's vertex positions (Y-up), so each exported corner can be named by the vertex it sits on."""

    def __init__(self, positions: list[Vec]) -> None:
        self.positions = positions

    def index(self, p: Vec) -> Key:
        """The one vertex within MATCH_TOL of `p`; raises on none or several, never guesses."""
        near = [i for i, q in enumerate(self.positions) if max(abs(p[k] - q[k]) for k in range(3)) < MATCH_TOL]
        if len(near) != 1:
            raise ValueError(f"{p} matches {len(near)} Blender vertices")
        return near[0]


def y_up(p: object) -> Vec:
    """Blender's Z-up world position as both exporters write it by default: (x, z, -y)."""
    if not (isinstance(p, list) and len(p) == 3 and all(isinstance(c, float | int) for c in p)):
        raise TypeError(f"expected a position, got {p!r}")
    x, y, z = (float(c) for c in p)
    return (x, z, -y)


def read_glb(path: Path) -> tuple[dict[str, object], bytes]:
    """The JSON and binary chunks of a GLB; raises on a malformed container."""
    data = path.read_bytes()
    magic, version, length = struct.unpack_from("<4sII", data, 0)
    js_len, js_type = struct.unpack_from("<I4s", data, 12)
    bin_at = 20 + js_len
    bin_len, bin_type = struct.unpack_from("<I4s", data, bin_at)
    if (magic, version, length, js_type, bin_type) != (b"glTF", 2, len(data), b"JSON", b"BIN\0"):
        raise ValueError(f"{path.name}: not a two-chunk GLB")
    doc: object = json.loads(data[20:bin_at])
    if not isinstance(doc, dict):
        raise TypeError(f"{path.name}: glTF JSON is not an object")
    return doc, data[bin_at + 8 : bin_at + 8 + bin_len]


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
    """One accessor as rows of numbers: f32 VEC2/VEC3, or u16/u32 SCALAR indices; any byte stride."""
    acc = as_dict(as_list(doc["accessors"], "accessors")[as_int(index, "accessor index")], "accessor")
    view = as_dict(as_list(doc["bufferViews"], "bufferViews")[as_int(acc["bufferView"], "bufferView")], "bufferView")
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3}[str(acc["type"])]
    fmt, size = {5126: ("f", 4), 5125: ("I", 4), 5123: ("H", 2)}[as_int(acc["componentType"], "componentType")]
    start = as_int(view.get("byteOffset", 0), "byteOffset") + as_int(acc.get("byteOffset", 0), "byteOffset")
    stride = as_int(view.get("byteStride", size * width), "byteStride")
    count = as_int(acc["count"], "count")
    return [struct.unpack_from(f"<{width}{fmt}", blob, start + i * stride) for i in range(count)]


def read_glb_polygons(path: Path, verts: Vertices) -> tuple[list[Polygon], list[str], int]:
    """Triangles with corner UVs (V flipped back to OBJ's convention), material names, and the vertex count."""
    doc, blob = read_glb(path)
    names = [str(as_dict(m, "material")["name"]) for m in as_list(doc.get("materials", []), "materials")]
    polygons: list[Polygon] = []
    vertices = 0
    mesh = as_dict(as_list(doc["meshes"], "meshes")[0], "mesh")
    for p in as_list(mesh["primitives"], "primitives"):
        prim = as_dict(p, "primitive")
        attrs = as_dict(prim["attributes"], "attributes")
        pos = accessor(doc, blob, attrs["POSITION"])
        tex = accessor(doc, blob, attrs["TEXCOORD_0"])
        idx = [int(i[0]) for i in accessor(doc, blob, prim["indices"])]
        vertices += len(pos)
        for n in range(0, len(idx), 3):
            corners = [(verts.index((pos[g][0], pos[g][1], pos[g][2])), (tex[g][0], 1.0 - tex[g][1])) for g in idx[n : n + 3]]
            polygons.append(corners)
    return polygons, names, vertices


def read_obj_polygons(path: Path, verts: Vertices) -> tuple[list[Polygon], list[str]]:
    """Polygons (any corner count) with corner UVs, and the `usemtl` names in order; `vn` and `s` are read past."""
    pos: list[Vec] = []
    uvs: list[UV] = []
    polygons: list[Polygon] = []
    materials: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        word, *rest = line.split() or [""]
        if word == "v":
            pos.append((float(rest[0]), float(rest[1]), float(rest[2])))
        elif word == "vt":
            uvs.append((float(rest[0]), float(rest[1])))
        elif word == "usemtl":
            materials.append(line[len("usemtl ") :])
        elif word == "f":
            corners = [s.split("/") for s in rest]
            polygons.append([(verts.index(pos[int(c[0]) - 1]), uvs[int(c[1]) - 1]) for c in corners])
        elif word not in ("", "#", "mtllib", "o", "vn", "s"):
            raise ValueError(f"{path.name}: unexpected OBJ line {line!r}")
    return polygons, materials


def edge_gaps(polygons: list[Polygon]) -> tuple[dict[frozenset[Key], float], int]:
    """For each interior edge, the largest corner-UV difference across it at either endpoint; and boundary edges."""
    sides: dict[frozenset[Key], list[dict[Key, UV]]] = {}
    for poly in polygons:
        uv_at = dict(poly)
        ring = [k for k, _ in poly]
        for a, b in zip(ring, ring[1:] + ring[:1]):
            sides.setdefault(frozenset((a, b)), []).append(uv_at)
    gaps: dict[frozenset[Key], float] = {}
    boundary = 0
    for edge, both in sides.items():
        if len(both) == 1:
            boundary += 1
            continue
        if len(both) != 2:
            raise ValueError(f"an edge with {len(both)} faces")
        left, right = both
        gaps[edge] = max(abs(left[v][k] - right[v][k]) for v in edge for k in (0, 1))
    return gaps, boundary


def report(polygons: list[Polygon], seams: list[frozenset[Key]] | None, poles: list[Key]) -> dict[str, object]:
    gaps, boundary = edge_gaps(polygons)
    cuts = {e for e, g in gaps.items() if g > UV_CUT}
    out: dict[str, object] = {"edges": len(gaps), "boundary_edges": boundary, "cuts": len(cuts)}
    out["pole_edges_cut"] = sum(1 for e in cuts if any(p in e for p in poles))
    degree: dict[Key, int] = {}
    for e in cuts:
        for v in e:
            degree[v] = degree.get(v, 0) + 1
    out["dead_end_tips"] = sum(1 for n in degree.values() if n == 1) if boundary == 0 else None  # closed surfaces only
    if seams is not None:
        marked = set(seams)
        uncut = [g for e, g in gaps.items() if e not in marked]
        out |= {
            "marked": len(marked),
            "marked_not_cut": len(marked - cuts),
            "cut_not_marked": len(cuts - marked),
            "largest_gap_off_seams": max(uncut) if uncut else 0.0,
            "smallest_gap_on_seams": min(gaps[e] for e in marked if e in gaps),
        }
    nonzero = sorted(g for g in gaps.values() if g > 0.0)
    out["smallest_nonzero_gap"] = nonzero[0] if nonzero else None
    return out


def distinct_corners(polygons: list[Polygon]) -> int:
    return len({c for poly in polygons for c in poly})


def tip_uv_counts(polygons: list[Polygon], tips: list[Key]) -> list[int]:
    """How many distinct UVs each tip vertex carries: 1 means the slit ends there, as a dead end does."""
    return [len({uv for poly in polygons for k, uv in poly if k == t}) for t in tips]


def flip_matches(glb: list[Polygon], obj: list[Polygon]) -> bool:
    """Every glTF corner UV, flipped back, is within 1e-6 of some OBJ corner UV on the same vertex."""
    at: dict[Key, list[UV]] = {}
    for poly in obj:
        for k, uv in poly:
            at.setdefault(k, []).append(uv)
    return all(
        any(max(abs(u - a), abs(v - b)) < 1e-6 for a, b in at.get(k, [])) for poly in glb for k, (u, v) in poly
    )


def positions(x: object, what: str) -> list[Vec]:
    return [y_up(p) for p in as_list(x, what)]


def measure(directory: Path) -> dict[str, object]:
    raw: object = json.loads((directory / "seams.json").read_text(encoding="utf-8"))
    record = as_dict(raw, "seams.json")
    out: dict[str, object] = {"blender": str(record["blender"]), "uv_cut": UV_CUT}
    for name in sorted(k for k in record if k != "blender"):
        model = as_dict(record[name], name)
        verts = Vertices(positions(model["vertices"], "vertices"))
        seams: list[frozenset[Key]] | None = None
        if model["seams"] is not None:
            pairs = [as_list(e, "seam edge") for e in as_list(model["seams"], "seams")]
            seams = [frozenset(verts.index(y_up(end)) for end in pair) for pair in pairs]
        poles = [verts.index(p) for p in positions(model["poles"], "poles")]
        glb, names, vertices = read_glb_polygons(directory / f"{name}.glb", verts)
        obj, usemtl = read_obj_polygons(directory / f"{name}.obj", verts)
        entry: dict[str, object] = {"glb": report(glb, seams, poles), "obj": report(obj, seams, poles)}
        entry["glb_vertices"], entry["glb_distinct_position_uv"] = vertices, distinct_corners(glb)
        entry["obj_polygon_sizes"] = sorted({len(p) for p in obj})
        entry["v_flipped_against_obj"] = flip_matches(glb, obj)
        entry["glb_materials"], entry["obj_usemtl"] = names, usemtl
        if "tips" in model:
            tips = [verts.index(p) for p in positions(model["tips"], "tips")]
            entry["tip_uv_counts"] = {"glb": tip_uv_counts(glb, tips), "obj": tip_uv_counts(obj, tips)}
        out[name] = entry
    return out

if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: python3 measure.py <export-dir>")
    print(json.dumps(measure(Path(sys.argv[1])), indent=2, sort_keys=True))
