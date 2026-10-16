"""Source-seam fixtures: models whose UV cuts are the seam layout, written the way Blender's exporters write them.

glTF stores one vertex per distinct (position, UV), so the exporter splits a vertex wherever its corner UVs
differ, and it flips V because glTF's UV origin is top-left. OBJ keeps one `v` per position and indexes a
`vt` per corner. Both carry the same cuts; the seam set in fixtures/seams/ names them by the OBJ's indices.
"""

import json
import struct

from generate import HEADER, Edge, Mesh, capsule, num, seam_text

UV = tuple[float, float]
FaceUV = tuple[UV, UV, UV]  # the UV at each corner of one face, in the face's corner order

CUT_RING = 2  # the capsule ring cut all the way round: the tube's upper rim, where it meets the hemisphere (z = +30 mm)
SPLIT_V = (0.45, 0.55)  # the bottom island fills v up to the first, the top island from the second


def capsule_uv() -> tuple[Mesh, list[FaceUV], list[Edge]]:
    """Capsule cut along meridian 0 from pole to pole and around ring CUT_RING, so two UV islands.

    Each island is a cylindrical projection: u follows the meridian, v the ring. Pole corners in an
    island share one UV, so a pole fan carries no cut, as in an angle-based unwrap.
    """
    m, rings = capsule()
    north, south = 0, len(m.pos) - 1
    segments = len(rings[0])
    ring_of = {v: i for i, r in enumerate(rings) for v in r} | {north: -1, south: len(rings)}
    column = {v: j for r in rings for j, v in enumerate(r)}

    def v_coord(ring: int, top: bool) -> float:
        if top:
            return 1.0 - (1.0 - SPLIT_V[1]) * (ring + 1) / (CUT_RING + 1)
        return SPLIT_V[0] * (len(rings) - ring) / (len(rings) - CUT_RING)

    face_uvs: list[FaceUV] = []
    for f in m.faces:
        top = all(ring_of[v] <= CUT_RING for v in f)
        cols = {column[v] for v in f if v in column}
        wraps = cols == {0, segments - 1}  # the last column closes on meridian 0 at u = 1, not u = 0

        def uv(v: int) -> UV:
            if v in (north, south):
                return (0.5, round(v_coord(ring_of[v], top), 6))
            u = segments if wraps and column[v] == 0 else column[v]
            return (round(u / segments, 6), round(v_coord(ring_of[v], top), 6))

        face_uvs.append((uv(f[0]), uv(f[1]), uv(f[2])))
    meridian = [north] + [r[0] for r in rings] + [south]
    rim = rings[CUT_RING] + rings[CUT_RING][:1]
    cuts = list(zip(meridian, meridian[1:])) + list(zip(rim, rim[1:]))
    return m, face_uvs, cuts


def obj_uv_text(note: str, m: Mesh, face_uvs: list[FaceUV]) -> str:
    """OBJ with one `vt` per distinct UV, in order of first use."""
    index: dict[UV, int] = {}
    for corners in face_uvs:
        for uv in corners:
            index.setdefault(uv, len(index))
    lines = [HEADER, f"# {note}\n"]
    lines += [f"v {num(p[0])} {num(p[1])} {num(p[2])}\n" for p in m.pos]
    lines += [f"vt {num(u)} {num(v)}\n" for u, v in index]
    for f, corners in zip(m.faces, face_uvs):
        lines.append("f " + " ".join(f"{v + 1}/{index[uv] + 1}" for v, uv in zip(f, corners)) + "\n")
    return "".join(lines)


def f32(x: float) -> float:
    """The value an f32 holds, as the glTF stores it."""
    return float(struct.unpack("<f", struct.pack("<f", x))[0])


def glb_bytes(name: str, note: str, m: Mesh, face_uvs: list[FaceUV]) -> bytes:
    """One-primitive GLB: f32 POSITION and TEXCOORD_0, u32 indices, one vertex per distinct (position, UV)."""
    vertex: dict[tuple[int, UV], int] = {}
    indices = [vertex.setdefault((v, uv), len(vertex)) for f, cs in zip(m.faces, face_uvs) for v, uv in zip(f, cs)]
    pos = [tuple(f32(round(c, 6) + 0.0) for c in m.pos[v]) for v, _ in vertex]
    tex = [(f32(u), f32(1.0 - v)) for _, (u, v) in vertex]
    blob = b"".join(struct.pack("<3f", *p) for p in pos)
    tex_at = len(blob)
    blob += b"".join(struct.pack("<2f", *t) for t in tex)
    idx_at = len(blob)
    blob += struct.pack(f"<{len(indices)}I", *indices)
    views = [(0, tex_at, 34962), (tex_at, idx_at - tex_at, 34962), (idx_at, len(blob) - idx_at, 34963)]
    doc = {
        "asset": {"version": "2.0", "generator": "OpenPlush research/fixtures_gen", "extras": {"note": note, "units": "mm"}},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": name}],
        "meshes": [{"name": name, "primitives": [{"attributes": {"POSITION": 0, "TEXCOORD_0": 1}, "indices": 2}]}],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": len(pos), "type": "VEC3",
             "min": [min(p[k] for p in pos) for k in range(3)], "max": [max(p[k] for p in pos) for k in range(3)]},
            {"bufferView": 1, "componentType": 5126, "count": len(tex), "type": "VEC2"},
            {"bufferView": 2, "componentType": 5125, "count": len(indices), "type": "SCALAR"},
        ],
        "bufferViews": [{"buffer": 0, "byteOffset": o, "byteLength": n, "target": t} for o, n, t in views],
        "buffers": [{"byteLength": len(blob)}],
    }
    js = json.dumps(doc, separators=(",", ":")).encode("utf-8")
    js += b" " * (-len(js) % 4)
    blob += b"\0" * (-len(blob) % 4)
    chunks = struct.pack("<I4s", len(js), b"JSON") + js + struct.pack("<I4s", len(blob), b"BIN\0") + blob
    return struct.pack("<4sII", b"glTF", 2, 12 + len(chunks)) + chunks


def source_seam_files() -> dict[str, bytes]:
    """Every source-seam fixture, keyed by its path under fixtures/."""
    note = "capsule-uv: UV cuts along meridian 0 and the tube's upper rim, so two islands"
    m, face_uvs, cuts = capsule_uv()
    mesh = "meshes/source-seams/capsule-uv.obj"
    return {
        mesh: obj_uv_text(note, m, face_uvs).encode("utf-8"),
        "meshes/source-seams/capsule-uv.glb": glb_bytes("capsule-uv", note, m, face_uvs),
        "seams/capsule-uv.json": seam_text(mesh, note, cuts).encode("utf-8"),
    }
