"""Source-seam fixtures: models whose UV cuts are the seam layout, written the way Blender's exporters write them.

glTF stores one vertex per distinct (position, UV), so the exporter splits a vertex wherever its corner UVs
differ, and it flips V because glTF's UV origin is top-left. OBJ keeps one `v` per position and indexes a
`vt` per corner. Both carry the same cuts; the seam set in fixtures/seams/ names them by the OBJ's indices.
"""

import json
import math
import struct

from generate import HEADER, Edge, Face, Mesh, Vec, capsule, num, seam_text, sphere

UV = tuple[float, float]
FaceUV = tuple[UV, UV, UV]  # the UV at each corner of one face, in the face's corner order
FaceNormals = tuple[Vec, Vec, Vec]

CUT_RING = 2  # the capsule ring cut all the way round: the tube's upper rim, where it meets the hemisphere (z = +30 mm)
SPLIT_V = (0.45, 0.55)  # the bottom island fills v up to the first, the top island from the second
BAND_RINGS = (1, 5)  # sphere rings cut all the way round, like a neck and a waist: z = +-35.36 mm


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


def sphere_band() -> tuple[Mesh, list[FaceUV], list[Edge]]:
    """Sphere cut around rings BAND_RINGS: two caps and the band between them, so three UV islands.

    Each island is a polar projection about its own centre in UV space, so no meridian is cut. The band's
    island is therefore a ring, which is why it cannot lie flat in one piece without one more cut.
    """
    m, rings = sphere()
    north, south = 0, len(m.pos) - 1
    top_cut, bottom_cut = BAND_RINGS
    ring_of = {v: i for i, r in enumerate(rings) for v in r} | {north: -1, south: len(rings)}
    column = {v: j for r in rings for j, v in enumerate(r)}
    centre = ((0.25, 0.75), (0.5, 0.3), (0.75, 0.75))  # top cap, band, bottom cap

    def radius(ring: int, island: int) -> float:
        if island == 0:
            return 0.2 * (ring + 1) / (top_cut + 1)
        if island == 2:
            return 0.2 * (len(rings) - ring) / (len(rings) - bottom_cut)
        return 0.1 + 0.15 * (ring - top_cut) / (bottom_cut - top_cut)

    face_uvs: list[FaceUV] = []
    for f in m.faces:
        ringed = [ring_of[v] for v in f]
        island = 0 if max(ringed) <= top_cut else 2 if min(ringed) >= bottom_cut else 1

        def uv(v: int) -> UV:
            r, t = radius(ring_of[v], island), 2 * math.pi * column.get(v, 0) / len(rings[0])
            cx, cy = centre[island]
            turn = -1.0 if island == 2 else 1.0  # radius grows away from the south pole, so turn the other way
            return (round(cx + r * math.cos(t), 6) + 0.0, round(cy + turn * r * math.sin(t), 6) + 0.0)

        face_uvs.append((uv(f[0]), uv(f[1]), uv(f[2])))
    cuts = [e for ring in BAND_RINGS for e in zip(rings[ring], rings[ring][1:] + rings[ring][:1])]
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


def glb_bytes(name: str, note: str, m: Mesh, face_uvs: list[FaceUV], face_normals: list[FaceNormals] | None = None) -> bytes:
    """One-primitive GLB: f32 POSITION, TEXCOORD_0 and optional NORMAL, u32 indices.

    One vertex per distinct (position, UV, normal), so the glTF splits wherever any corner attribute differs.
    """
    per_face: list[FaceNormals | None] = list(face_normals) if face_normals is not None else [None] * len(m.faces)
    vertex: dict[tuple[int, UV, Vec | None], int] = {}
    indices = [
        vertex.setdefault((v, uv, n), len(vertex))
        for f, cs, ns in zip(m.faces, face_uvs, per_face)
        for v, uv, n in zip(f, cs, ns if ns is not None else (None, None, None))
    ]
    columns: list[tuple[str, str, list[tuple[float, ...]]]] = [
        ("POSITION", "VEC3", [tuple(f32(round(c, 6) + 0.0) for c in m.pos[v]) for v, _, _ in vertex]),
        ("TEXCOORD_0", "VEC2", [(f32(u), f32(1.0 - w)) for _, (u, w), _ in vertex]),
    ]
    if face_normals is not None:
        normals: list[tuple[float, ...]] = []
        for _, _, n in vertex:
            if n is None:
                raise ValueError(f"{name}: face_normals leaves a corner without a normal")
            normals.append(tuple(f32(c) for c in n))
        columns.append(("NORMAL", "VEC3", normals))
    blob = b""
    views: list[dict[str, int]] = []
    accessors: list[dict[str, object]] = []
    for _, kind, rows in columns:
        accessors.append({"bufferView": len(views), "componentType": 5126, "count": len(rows), "type": kind})
        views.append({"buffer": 0, "byteOffset": len(blob), "byteLength": 4 * len(rows) * len(rows[0]), "target": 34962})
        blob += b"".join(struct.pack(f"<{len(r)}f", *r) for r in rows)
    pos = columns[0][2]
    accessors[0] |= {"min": [min(p[k] for p in pos) for k in range(3)], "max": [max(p[k] for p in pos) for k in range(3)]}
    accessors.append({"bufferView": len(views), "componentType": 5125, "count": len(indices), "type": "SCALAR"})
    views.append({"buffer": 0, "byteOffset": len(blob), "byteLength": 4 * len(indices), "target": 34963})
    blob += struct.pack(f"<{len(indices)}I", *indices)
    attributes = {attr: k for k, (attr, _, _) in enumerate(columns)}
    doc = {
        "asset": {"version": "2.0", "generator": "OpenPlush research/fixtures_gen", "extras": {"note": note, "units": "mm"}},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": name}],
        "meshes": [{"name": name, "primitives": [{"attributes": attributes, "indices": len(columns)}]}],
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": len(blob)}],
    }
    return glb_container(doc, blob)


def glb_container(doc: dict[str, object], blob: bytes) -> bytes:
    """The GLB header, the JSON chunk padded with spaces and the binary chunk padded with zeros."""
    js = json.dumps(doc, separators=(",", ":")).encode("utf-8")
    js += b" " * (-len(js) % 4)
    blob += b"\0" * (-len(blob) % 4)
    chunks = struct.pack("<I4s", len(js), b"JSON") + js + struct.pack("<I4s", len(blob), b"BIN\0") + blob
    return struct.pack("<4sII", b"glTF", 2, 12 + len(chunks)) + chunks


def capsule_hardshade_normals(m: Mesh) -> list[FaceNormals]:
    """Smooth normals everywhere except the lower hemisphere, which is flat-shaded (each corner takes its face normal).

    Smooth vertex normals are the normalised sum of the unnormalised face normals around the vertex, so area-weighted.
    """
    _, rings = capsule()
    flat_from = len(rings) - CUT_RING - 1  # mirrors CUT_RING: the lower rim, where the tube meets the lower hemisphere
    lower = {v for r in rings[flat_from:] for v in r} | {len(m.pos) - 1}

    def face_normal(f: Face) -> Vec:
        a, b, c = (m.pos[v] for v in f)
        e1, e2 = [b[k] - a[k] for k in range(3)], [c[k] - a[k] for k in range(3)]
        return (e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0])

    def unit(n: Vec) -> Vec:
        length = math.sqrt(sum(c * c for c in n))
        return (round(n[0] / length, 6) + 0.0, round(n[1] / length, 6) + 0.0, round(n[2] / length, 6) + 0.0)

    summed: dict[int, Vec] = {}
    for f in m.faces:
        fn = face_normal(f)
        for v in f:
            s = summed.get(v, (0.0, 0.0, 0.0))
            summed[v] = (s[0] + fn[0], s[1] + fn[1], s[2] + fn[2])
    out: list[FaceNormals] = []
    for f in m.faces:
        if all(v in lower for v in f):
            n = unit(face_normal(f))
            out.append((n, n, n))
        else:
            out.append((unit(summed[f[0]]), unit(summed[f[1]]), unit(summed[f[2]])))
    return out


def source_seam_files() -> dict[str, bytes]:
    """Every source-seam fixture, keyed by its path under fixtures/."""
    note = "capsule-uv: UV cuts along meridian 0 and the tube's upper rim, so two islands"
    m, face_uvs, cuts = capsule_uv()
    mesh = "meshes/source-seams/capsule-uv.obj"
    return {
        mesh: obj_uv_text(note, m, face_uvs).encode("utf-8"),
        "meshes/source-seams/capsule-uv.glb": glb_bytes("capsule-uv", note, m, face_uvs),
        "seams/capsule-uv.json": seam_text(mesh, note, cuts).encode("utf-8"),
        "meshes/source-seams/capsule-hardshade.glb": glb_bytes(
            "capsule-hardshade",
            "capsule-hardshade: capsule-uv's UVs, with the lower hemisphere flat-shaded, so normals split vertices too",
            m,
            face_uvs,
            capsule_hardshade_normals(m),
        ),
    } | sphere_band_files()


def sphere_band_files() -> dict[str, bytes]:
    note = "sphere-band: UV cuts around two rings, so two caps and a band that is an annulus"
    m, face_uvs, cuts = sphere_band()
    mesh = "meshes/source-seams/sphere-band.obj"
    return {
        mesh: obj_uv_text(note, m, face_uvs).encode("utf-8"),
        "meshes/source-seams/sphere-band.glb": glb_bytes("sphere-band", note, m, face_uvs),
        "seams/non-disk/sphere-band.json": seam_text(mesh, note, cuts).encode("utf-8"),
    }
