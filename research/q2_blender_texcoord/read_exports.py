# SPDX-License-Identifier: GPL-3.0-or-later
"""Names the Blender UV map behind each exported UV set, by the constant u that export_texcoord.py wrote into each map.

Run: python3 read_exports.py <export-dir>   (the directory export_texcoord.py wrote; needs no bpy)
Prints one JSON report. Every answer in results.md comes from it.
"""

import json
import struct
import sys
from pathlib import Path

MAP_U = {"Zeta": 0.125, "Alpha": 0.875}  # exact in f32 and in OBJ's 6 decimals, so matching is exact
FLOAT, VEC2_BYTES = 5126, 8


def as_dict(x: object, what: str) -> dict[str, object]:
    if not isinstance(x, dict):
        raise ValueError(f"{what}: expected an object")
    return {str(k): v for k, v in x.items()}


def as_list(x: object, what: str) -> list[object]:
    if not isinstance(x, list):
        raise ValueError(f"{what}: expected an array")
    return list(x)


def as_int(x: object, what: str) -> int:
    if not isinstance(x, int) or isinstance(x, bool):
        raise ValueError(f"{what}: expected an integer")
    return x


def map_name(u: float) -> str:
    names = [name for name, value in MAP_U.items() if value == u]
    if len(names) != 1:
        raise ValueError(f"u = {u} matches no UV map the spike wrote")
    return names[0]


def read_glb(data: bytes) -> tuple[dict[str, object], bytes]:
    magic, version, _ = struct.unpack_from("<4sII", data, 0)
    if magic != b"glTF" or version != 2:
        raise ValueError("not a glTF 2 binary")
    json_len, json_type = struct.unpack_from("<I4s", data, 12)
    if json_type != b"JSON":
        raise ValueError("first chunk is not JSON")
    bin_at = 20 + json_len
    bin_len, bin_type = struct.unpack_from("<I4s", data, bin_at)
    if bin_type != b"BIN\0":
        raise ValueError("second chunk is not BIN")
    return as_dict(json.loads(data[20:bin_at]), "glTF"), data[bin_at + 8 : bin_at + 8 + bin_len]


def constant_u(doc: dict[str, object], blob: bytes, index: int) -> float:
    """The one u value every corner of a TEXCOORD accessor carries; raises if they differ."""
    acc = as_dict(as_list(doc["accessors"], "accessors")[index], "accessor")
    if acc.get("componentType") != FLOAT or acc.get("type") != "VEC2":
        raise ValueError("TEXCOORD accessor is not float VEC2")
    view = as_dict(as_list(doc["bufferViews"], "bufferViews")[as_int(acc["bufferView"], "bufferView")], "bufferView")
    if view.get("byteStride", VEC2_BYTES) != VEC2_BYTES:
        raise ValueError("interleaved TEXCOORD data is not supported")
    start = as_int(view.get("byteOffset", 0), "byteOffset") + as_int(acc.get("byteOffset", 0), "byteOffset")
    count = as_int(acc["count"], "count")
    us: set[float] = {struct.unpack_from("<f", blob, start + i * VEC2_BYTES)[0] for i in range(count)}
    if len(us) != 1:
        raise ValueError(f"TEXCOORD accessor {index} holds {len(us)} distinct u values, not one")
    return us.pop()


def texture_texcoord(doc: dict[str, object]) -> int | None:
    """The TEXCOORD index the first material's base colour texture samples; None without one."""
    if "materials" not in doc:
        return None
    pbr = as_dict(as_dict(as_list(doc["materials"], "materials")[0], "material").get("pbrMetallicRoughness", {}), "pbr")
    if "baseColorTexture" not in pbr:
        return None
    return as_int(as_dict(pbr["baseColorTexture"], "baseColorTexture").get("texCoord", 0), "texCoord")


def glb_maps(data: bytes) -> dict[str, object]:
    doc, blob = read_glb(data)
    meshes = as_list(doc["meshes"], "meshes")
    prims = as_list(as_dict(meshes[0], "mesh")["primitives"], "primitives")
    if len(meshes) != 1 or len(prims) != 1:
        raise ValueError("expected one mesh with one primitive")
    attrs = as_dict(as_dict(prims[0], "primitive")["attributes"], "attributes")
    count = len([k for k in attrs if k.startswith("TEXCOORD_")])
    names = []
    for n in range(count):
        if f"TEXCOORD_{n}" not in attrs:
            raise ValueError(f"TEXCOORD_{n} is missing below a higher set")
        names.append(map_name(constant_u(doc, blob, as_int(attrs[f"TEXCOORD_{n}"], "attribute"))))
    return {"texcoords": names, "texture_texcoord": texture_texcoord(doc)}


def obj_map(text: str) -> str:
    us = {float(line.split()[1]) for line in text.splitlines() if line.startswith("vt ")}
    if len(us) != 1:
        raise ValueError(f"OBJ holds {len(us)} distinct u values, not one")
    return map_name(us.pop())


def measure(directory: Path) -> dict[str, object]:
    report: dict[str, object] = {}
    for case in as_list(json.loads((directory / "cases.json").read_text()), "cases"):
        c = as_dict(case, "case")
        name = str(c["name"])
        report[name] = {
            "blender": c,
            "glb": glb_maps((directory / f"{name}.glb").read_bytes()),
            "obj": obj_map((directory / f"{name}.obj").read_text()),
        }
    return report


if __name__ == "__main__":
    print(json.dumps(measure(Path(sys.argv[1])), indent=1, sort_keys=True))
