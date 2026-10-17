# SPDX-License-Identifier: GPL-3.0-or-later
"""Oracles for read_exports.py, on hand-built GLB bytes and OBJ text. Run: python3 -m unittest -v"""

import json
import struct
import unittest

from read_exports import glb_maps, obj_map

FIRST, SECOND = 0.125, 0.875


def glb(texcoords: dict[str, float], texture: int | None = None, material: bool = True, stride: int | None = None) -> bytes:
    """A one-triangle GLB whose TEXCOORD accessors each hold one constant u, built without read_exports."""
    blob = b""
    accessors: list[dict[str, object]] = []
    views: list[dict[str, object]] = []
    attributes: dict[str, int] = {}
    for name, u in texcoords.items():
        view: dict[str, object] = {"buffer": 0, "byteOffset": len(blob), "byteLength": 24}
        if stride is not None:
            view["byteStride"] = stride
        blob += struct.pack("<6f", u, 0.0, u, 0.5, u, 1.0)
        views.append(view)
        accessors.append({"bufferView": len(views) - 1, "componentType": 5126, "count": 3, "type": "VEC2"})
        attributes[name] = len(accessors) - 1
    doc: dict[str, object] = {
        "asset": {"version": "2.0"},
        "buffers": [{"byteLength": len(blob)}],
        "bufferViews": views,
        "accessors": accessors,
        "meshes": [{"primitives": [{"attributes": attributes}]}],
    }
    if material:
        pbr: dict[str, object] = {}
        if texture is not None:
            pbr["baseColorTexture"] = {"index": 0} if texture == 0 else {"index": 0, "texCoord": texture}
        doc["materials"] = [{"pbrMetallicRoughness": pbr}]
    text = json.dumps(doc).encode()
    text += b" " * (-len(text) % 4)
    body = struct.pack("<I4s", len(text), b"JSON") + text + struct.pack("<I4s", len(blob), b"BIN\0") + blob
    return struct.pack("<4sII", b"glTF", 2, 12 + len(body)) + body


class GlbMaps(unittest.TestCase):
    def test_order_comes_from_the_attribute_index(self) -> None:
        """Prevents reporting the maps in name order instead of TEXCOORD order, which would hide the answer to Q2."""
        maps = glb_maps(glb({"TEXCOORD_1": FIRST, "TEXCOORD_0": SECOND}))
        self.assertEqual(maps["texcoords"], ["Second", "First"])

    def test_texture_texcoord(self) -> None:
        """Prevents misreading which TEXCOORD a texture samples: glTF's default is 0 when texCoord is absent."""
        both = {"TEXCOORD_0": FIRST, "TEXCOORD_1": SECOND}
        self.assertEqual(glb_maps(glb(both, texture=1))["texture_texcoord"], 1)
        self.assertEqual(glb_maps(glb(both, texture=0))["texture_texcoord"], 0)
        self.assertIsNone(glb_maps(glb(both))["texture_texcoord"])
        self.assertIsNone(glb_maps(glb(both, material=False))["texture_texcoord"])

    def test_unknown_u_raises(self) -> None:
        """Prevents naming a map by the nearest value: a u that matches no map exactly is an export the spike didn't make."""
        with self.assertRaises(ValueError):
            glb_maps(glb({"TEXCOORD_0": 0.5}))

    def test_varying_u_raises(self) -> None:
        """Prevents naming a map from its first corner when the corners disagree."""
        data = bytearray(glb({"TEXCOORD_0": FIRST}))
        at = data.rindex(struct.pack("<f", FIRST))
        data[at : at + 4] = struct.pack("<f", SECOND)
        with self.assertRaises(ValueError):
            glb_maps(bytes(data))

    def test_gap_in_texcoords_raises(self) -> None:
        """Prevents reading TEXCOORD_1 as the first set when TEXCOORD_0 is missing."""
        with self.assertRaises(ValueError):
            glb_maps(glb({"TEXCOORD_1": FIRST}))

    def test_strided_view_raises(self) -> None:
        """Prevents misreading interleaved data: the reader handles tightly packed VEC2 only."""
        with self.assertRaises(ValueError):
            glb_maps(glb({"TEXCOORD_0": FIRST}, stride=16))

    def test_bad_magic_raises(self) -> None:
        with self.assertRaises(ValueError):
            glb_maps(b"glTX" + glb({"TEXCOORD_0": FIRST})[4:])


class ObjMap(unittest.TestCase):
    def test_one_map(self) -> None:
        """Prevents a wrong answer for OBJ, which carries a single UV map."""
        self.assertEqual(obj_map("v 0 0 0\nvt 0.875000 0.1\nvt 0.875000 0.9\nf 1/1 1/2 1/1\n"), "Second")

    def test_no_vt_raises(self) -> None:
        with self.assertRaises(ValueError):
            obj_map("v 0 0 0\nf 1 1 1\n")

    def test_mixed_u_raises(self) -> None:
        """Prevents naming one map when the OBJ mixes both."""
        with self.assertRaises(ValueError):
            obj_map("vt 0.125 0\nvt 0.875 0\n")


if __name__ == "__main__":
    unittest.main()
