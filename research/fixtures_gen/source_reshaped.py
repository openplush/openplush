"""Re-import fixtures: capsule-uv in two materials, before and after an edit that stretches the lower cap only.

v1 is capsule-uv's mesh and UV cuts with a `Head` upper hemisphere and a `Body` rest, so the tube's upper rim is
both a UV cut and a material boundary. v2 moves every vertex below the tube's lower rim further down and nothing
else, so the seams away from the cap re-place on v2 by position and the three touching it cannot.
"""

from generate import Face, Mesh, capsule
from source_seams import CUT_RING, FaceUV, Primitive, capsule_uv, glb_bytes

MATERIALS = ["Head", "Body"]
STRETCH = 1.5  # the lower cap's depth below its rim grows by this factor


def reshaped(stretch: float) -> tuple[Mesh, list[list[Face]], list[list[FaceUV]]]:
    """capsule-uv with faces grouped Head then Body, the lower cap's depth scaled by `stretch`."""
    m, face_uvs, _ = capsule_uv()
    _, rings = capsule()
    upper, lower = (m.pos[rings[r][0]][2] for r in (CUT_RING, len(rings) - CUT_RING - 1))  # the tube's two rims
    m.pos = [(x, y, lower + (z - lower) * stretch) if z < lower else (x, y, z) for x, y, z in m.pos]
    runs: list[list[Face]] = [[], []]
    uvs: list[list[FaceUV]] = [[], []]
    for f, uv in zip(m.faces, face_uvs, strict=True):
        k = 0 if min(m.pos[v][2] for v in f) >= upper else 1
        runs[k].append(f)
        uvs[k].append(uv)
    m.faces = runs[0] + runs[1]
    return m, runs, uvs


def reshaped_files() -> dict[str, bytes]:
    out: dict[str, bytes] = {}
    for name, stretch, what in (("reshaped-v1", 1.0, "as exported"), ("reshaped-v2", STRETCH, "lower cap stretched")):
        m, runs, uvs = reshaped(stretch)
        note = f"{name}: capsule-uv in Head and Body materials, {what}"
        prims = [Primitive(run, uv, material=k) for k, (run, uv) in enumerate(zip(runs, uvs, strict=True))]
        out[f"meshes/source-seams/{name}.glb"] = glb_bytes(name, note, m, prims, MATERIALS)
    return out
