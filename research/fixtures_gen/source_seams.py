"""Source-seam fixtures: models whose UV cuts are the seam layout, written the way Blender's exporters write them.

glTF stores one vertex per distinct (position, UV), so the exporter splits a vertex wherever its corner UVs
differ, and it flips V because glTF's UV origin is top-left. OBJ keeps one `v` per position and indexes a
`vt` per corner. Both carry the same cuts; the seam set in fixtures/seams/ names them by the OBJ's indices.
"""

import json
import math
import struct
from dataclasses import dataclass, field
from typing import TypeVar

from generate import HEADER, Edge, Face, Mesh, Vec, capsule, num, seam_text, sphere

UV = tuple[float, float]
FaceUV = tuple[UV, UV, UV]  # the UV at each corner of one face, in the face's corner order
FaceNormals = tuple[Vec, Vec, Vec]
T = TypeVar("T")

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


MATERIALS = ("minky / cuddle", "Polar fleece ", "Felt Nose")  # case, a trailing space, and a name no preset has


def three_materials() -> tuple[Mesh, list[list[Face]], list[Edge]]:
    """The capsule in three materials: upper hemisphere, tube, lower hemisphere. Faces are regrouped by material.

    Returns the mesh, each material's faces in MATERIALS order, and the material boundaries (both tube rims).
    """
    m, rings = capsule()
    upper, lower = CUT_RING, len(rings) - CUT_RING - 1  # the tube's rims
    ring_of = {v: i for i, r in enumerate(rings) for v in r} | {0: -1, len(m.pos) - 1: len(rings)}
    runs: list[list[Face]] = [[], [], []]
    for f in m.faces:
        ringed = [ring_of[v] for v in f]
        runs[0 if max(ringed) <= upper else 2 if min(ringed) >= lower else 1].append(f)
    m.faces = [f for run in runs for f in run]
    boundaries = [e for ring in (upper, lower) for e in zip(rings[ring], rings[ring][1:] + rings[ring][:1])]
    return m, runs, boundaries


def obj_source_text(
    note: str, m: Mesh, face_uvs: list[FaceUV] | None = None, materials: list[tuple[str, int]] | None = None
) -> str:
    """OBJ of `m`, optionally with one `vt` per distinct UV (in order of first use) and material runs.

    `materials` is (name, face count) per run in face order. With it, the file is one object, named by the note's
    first word, with a `usemtl` before each run, so the materials switch inside one object.
    """
    index: dict[UV, int] = {}
    for uvs in face_uvs or []:
        for uv in uvs:
            index.setdefault(uv, len(index))
    lines = [HEADER, f"# {note}\n"]
    starts: dict[int, str] = {}
    if materials is not None:
        lines.append(f"o {note.split(':')[0]}\n")
        first = 0
        for name, count in materials:
            starts[first] = name
            first += count
    lines += [f"v {num(p[0])} {num(p[1])} {num(p[2])}\n" for p in m.pos]
    lines += [f"vt {num(u)} {num(v)}\n" for u, v in index]
    for k, f in enumerate(m.faces):
        if k in starts:
            lines.append(f"usemtl {starts[k]}\n")
        if face_uvs is None:
            lines.append(f"f {f[0] + 1} {f[1] + 1} {f[2] + 1}\n")
        else:
            lines.append("f " + " ".join(f"{v + 1}/{index[uv] + 1}" for v, uv in zip(f, face_uvs[k])) + "\n")
    return "".join(lines)


def f32(x: float) -> float:
    """The value an f32 holds, as the glTF stores it."""
    return float(struct.unpack("<f", struct.pack("<f", x))[0])


@dataclass(frozen=True)
class Primitive:
    """One glTF primitive: faces of the mesh, with optional per-corner UVs and normals and a material index."""

    faces: list[Face]
    uvs: list[FaceUV] | None = None
    normals: list[FaceNormals] | None = None
    material: int | None = None


@dataclass
class Buffer:
    """The binary chunk being built, with its buffer views and accessors."""

    blob: bytes = b""
    views: list[dict[str, int]] = field(default_factory=list)
    accessors: list[dict[str, object]] = field(default_factory=list)

    def add(self, kind: str, component: int, data: bytes, target: int) -> int:
        """Appends one tightly packed accessor of 4-byte components; its count follows from the data's length."""
        count = len(data) // (4 * {"SCALAR": 1, "VEC2": 2, "VEC3": 3}[kind])
        self.accessors.append({"bufferView": len(self.views), "componentType": component, "count": count, "type": kind})
        self.views.append({"buffer": 0, "byteOffset": len(self.blob), "byteLength": len(data), "target": target})
        self.blob += data
        return len(self.accessors) - 1

    def floats(self, kind: str, rows: list[tuple[float, ...]]) -> int:
        return self.add(kind, 5126, b"".join(struct.pack(f"<{len(r)}f", *r) for r in rows), 34962)


def present(values: list[T | None], what: str) -> list[T]:
    """`values` with None ruled out; raises naming `what` instead of dropping one."""
    out: list[T] = []
    for x in values:
        if x is None:
            raise ValueError(what)
        out.append(x)
    return out


def primitive_json(buf: Buffer, name: str, m: Mesh, prim: Primitive) -> dict[str, object]:
    """Writes one primitive's attributes and indices; one vertex per distinct (position, UV, normal) in it."""
    uvs: list[FaceUV | None] = list(prim.uvs) if prim.uvs is not None else [None] * len(prim.faces)
    nrm: list[FaceNormals | None] = list(prim.normals) if prim.normals is not None else [None] * len(prim.faces)
    vertex: dict[tuple[int, UV | None, Vec | None], int] = {}
    indices = [
        vertex.setdefault((v, uv, n), len(vertex))
        for f, cs, ns in zip(prim.faces, uvs, nrm, strict=True)
        for v, uv, n in zip(f, cs if cs is not None else (None,) * 3, ns if ns is not None else (None,) * 3)
    ]
    pos = [tuple(f32(round(c, 6) + 0.0) for c in m.pos[v]) for v, _, _ in vertex]
    attributes = {"POSITION": buf.floats("VEC3", pos)}
    buf.accessors[-1] |= {"min": [min(p[k] for p in pos) for k in range(3)], "max": [max(p[k] for p in pos) for k in range(3)]}
    if prim.uvs is not None:
        tex = present([uv for _, uv, _ in vertex], f"{name}: a corner without a UV")
        attributes["TEXCOORD_0"] = buf.floats("VEC2", [(f32(u), f32(1.0 - w)) for u, w in tex])
    if prim.normals is not None:
        normals = present([n for _, _, n in vertex], f"{name}: a corner without a normal")
        attributes["NORMAL"] = buf.floats("VEC3", [tuple(f32(c) for c in n) for n in normals])
    out: dict[str, object] = {"attributes": attributes}
    out["indices"] = buf.add("SCALAR", 5125, struct.pack(f"<{len(indices)}I", *indices), 34963)
    if prim.material is not None:
        out["material"] = prim.material
    return out


def glb_bytes(name: str, note: str, m: Mesh, prims: list[Primitive], materials: list[str] | None = None) -> bytes:
    """GLB with one mesh of `prims`, f32 attributes and u32 indices, and named materials if given."""
    buf = Buffer()
    primitives = [primitive_json(buf, name, m, p) for p in prims]
    doc: dict[str, object] = {
        "asset": {"version": "2.0", "generator": "OpenPlush research/fixtures_gen", "extras": {"note": note, "units": "mm"}},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": name}],
        "meshes": [{"name": name, "primitives": primitives}],
    }
    if materials is not None:
        doc["materials"] = [{"name": n} for n in materials]
    doc |= {"accessors": buf.accessors, "bufferViews": buf.views, "buffers": [{"byteLength": len(buf.blob)}]}
    return glb_container(doc, buf.blob)


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
        mesh: obj_source_text(note, m, face_uvs).encode("utf-8"),
        "meshes/source-seams/capsule-uv.glb": glb_bytes("capsule-uv", note, m, [Primitive(m.faces, face_uvs)]),
        "seams/capsule-uv.json": seam_text(mesh, note, cuts).encode("utf-8"),
        "meshes/source-seams/capsule-hardshade.glb": glb_bytes(
            "capsule-hardshade",
            "capsule-hardshade: capsule-uv's UVs, with the lower hemisphere flat-shaded, so normals split vertices too",
            m,
            [Primitive(m.faces, face_uvs, capsule_hardshade_normals(m))],
        ),
    } | sphere_band_files() | three_materials_files()


def three_materials_files() -> dict[str, bytes]:
    note = "three-materials: the capsule's upper hemisphere, tube and lower hemisphere in three materials, no UVs"
    m, runs, boundaries = three_materials()
    mesh = "meshes/source-seams/three-materials.obj"
    prims = [Primitive(run, material=k) for k, run in enumerate(runs)]
    return {
        mesh: obj_source_text(note, m, materials=[(n, len(r)) for n, r in zip(MATERIALS, runs, strict=True)]).encode("utf-8"),
        "meshes/source-seams/three-materials.glb": glb_bytes("three-materials", note, m, prims, list(MATERIALS)),
        "seams/non-disk/three-materials.json": seam_text(mesh, note, boundaries, "fabricBoundary").encode("utf-8"),
    }


def sphere_band_files() -> dict[str, bytes]:
    note = "sphere-band: UV cuts around two rings, so two caps and a band that is an annulus"
    m, face_uvs, cuts = sphere_band()
    mesh = "meshes/source-seams/sphere-band.obj"
    return {
        mesh: obj_source_text(note, m, face_uvs).encode("utf-8"),
        "meshes/source-seams/sphere-band.glb": glb_bytes("sphere-band", note, m, [Primitive(m.faces, face_uvs)]),
        "seams/non-disk/sphere-band.json": seam_text(mesh, note, cuts).encode("utf-8"),
    }
