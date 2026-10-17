"""Dead-end fixture: UV cuts that stop inside the surface, beside one that ends on a material boundary and an opening.

The capsule loses its bottom pole fan, so the lowest ring is an opening, and its upper hemisphere is a second
material, so the tube's upper rim is a material boundary. Cut A runs down meridian 3 from that boundary to the
opening. Two cuts stop short: D1 runs down meridian 6 from the boundary for two edges; D2 leaves A along the tube's
lower rim for two edges. Their far ends are the only dead-end tips.
"""

import json

from generate import Edge, Face, Mesh, capsule, seam_text
from source_seams import CUT_RING, UV, FaceUV, Primitive, glb_bytes, obj_source_text

MATERIALS = ("Head", "Body")
WEDGE_STEP = 0.01  # u offset between a vertex's corner groups; far above any UV tolerance, so every split reads


def path(vs: list[int]) -> list[Edge]:
    return list(zip(vs, vs[1:]))


def dead_end() -> tuple[Mesh, list[list[Face]], list[Edge], list[int]]:
    """The mesh (faces grouped Head then Body), the two runs, the UV cuts, and the two dead-end tips."""
    m, rings = capsule()
    south = len(m.pos) - 1
    m.faces = [f for f in m.faces if south not in f]
    m.pos = m.pos[:south]  # the opening: no bottom pole and no fan around it
    upper, lower = CUT_RING, len(rings) - CUT_RING - 1
    ring_of = {v: i for i, r in enumerate(rings) for v in r} | {0: -1}
    head = [f for f in m.faces if max(ring_of[v] for v in f) <= upper]
    body = [f for f in m.faces if max(ring_of[v] for v in f) > upper]
    m.faces = head + body
    a = [rings[r][3] for r in range(upper, len(rings))]
    d1 = [rings[r][6] for r in range(upper, upper + 3)]
    d2 = [rings[lower][c] for c in (3, 4, 5)]
    cuts = path(a) + path(d1) + path(d2)
    return m, [head, body], cuts, sorted([d1[-1], d2[-1]])


def wedge_uvs(m: Mesh, cuts: set[Edge]) -> list[FaceUV]:
    """Planar UVs, offset per wedge: around each vertex, faces joined across an uncut edge share one UV.

    So an edge reads as a UV cut exactly when it is in `cuts` and one of its ends has more than one wedge.
    """
    around: dict[int, list[int]] = {}
    for i, f in enumerate(m.faces):
        for v in f:
            around.setdefault(v, []).append(i)
    wedge: dict[tuple[int, int], int] = {}
    for v, fs in around.items():
        parent = {i: i for i in fs}

        def find(i: int) -> int:
            while parent[i] != i:
                i = parent[i]
            return i

        sharing: dict[int, list[int]] = {}
        for i in fs:
            for w in m.faces[i]:
                if w != v:
                    sharing.setdefault(w, []).append(i)
        for w, pair in sharing.items():
            if len(pair) == 2 and (min(v, w), max(v, w)) not in cuts:
                parent[find(pair[0])] = find(pair[1])
        roots = sorted({find(i) for i in fs}, key=lambda r: min(i for i in fs if find(i) == r))
        for i in fs:
            wedge[(v, i)] = roots.index(find(i))

    def uv(v: int, i: int) -> UV:
        x, _, z = m.pos[v]
        return (round(0.5 + x / 200 + WEDGE_STEP * wedge[(v, i)], 6) + 0.0, round(0.5 + z / 200, 6) + 0.0)

    return [(uv(f[0], i), uv(f[1], i), uv(f[2], i)) for i, f in enumerate(m.faces)]


def tips_text(mesh: str, note: str, tips: list[int]) -> str:
    """The dead-end oracle: tips as 0-based OBJ vertex indices, ascending."""
    doc = {"format": "openplush-dead-ends", "version": 1, "mesh": mesh, "vertexIndexBase": 0, "note": note, "tips": tips}
    return json.dumps(doc, indent=2) + "\n"


def dead_end_files() -> dict[str, bytes]:
    note = "dead-end: an opening, Head and Body materials, one cut ending on both, and two cuts that stop short"
    m, runs, cuts, tips = dead_end()
    face_uvs = wedge_uvs(m, {(min(e), max(e)) for e in cuts})
    split = len(runs[0])
    prims = [Primitive(runs[0], face_uvs[:split], material=0), Primitive(runs[1], face_uvs[split:], material=1)]
    mesh = "meshes/source-seams/dead-end.obj"
    counts = [(name, len(run)) for name, run in zip(MATERIALS, runs, strict=True)]
    return {
        mesh: obj_source_text(note, m, face_uvs, counts, "dead-end").encode("utf-8"),
        "meshes/source-seams/dead-end.glb": glb_bytes("dead-end", note, m, prims, list(MATERIALS)),
        "seams/non-disk/dead-end.json": seam_text(mesh, note, cuts).encode("utf-8"),
        "seams/non-disk/dead-end.tips.json": tips_text(mesh, note, tips).encode("utf-8"),
    }
