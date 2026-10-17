# SPDX-License-Identifier: GPL-3.0-or-later
"""Oracles for read_exports.py, on hand-built GLB bytes and OBJ text. Run: python3 -m unittest -v"""

import json
import struct
import unittest
from collections.abc import Callable
from typing import Any

from read_exports import glb_maps, obj_map

ZETA, ALPHA = 0.125, 0.875  # Zeta is created first, so list order and name order disagree


def glb(
    texcoords: dict[str, float],
    texture: int | None = None,
    material: bool = True,
    stride: int | None = None,
    acc_offset: int = 0,
) -> bytes:
    """A one-triangle GLB whose TEXCOORD accessors each hold one constant u, built without read_exports."""
    blob = b""
    accessors: list[dict[str, object]] = []
    views: list[dict[str, object]] = []
    attributes: dict[str, int] = {}
    for name, u in texcoords.items():
        view: dict[str, object] = {"buffer": 0, "byteOffset": len(blob), "byteLength": 24 + acc_offset}
        if stride is not None:
            view["byteStride"] = stride
        blob += b"\xff" * acc_offset + struct.pack("<6f", u, 0.0, u, 0.5, u, 1.0)
        views.append(view)
        accessor: dict[str, object] = {"bufferView": len(views) - 1, "componentType": 5126, "count": 3, "type": "VEC2"}
        if acc_offset:
            accessor["byteOffset"] = acc_offset
        accessors.append(accessor)
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


def with_doc(data: bytes, edit: Callable[[dict[str, Any]], None]) -> bytes:
    """`data` with its glTF JSON edited and re-packed, so chunk lengths stay right."""
    json_len = struct.unpack_from("<I", data, 12)[0]
    doc = json.loads(data[20 : 20 + json_len])
    edit(doc)
    text = json.dumps(doc).encode()
    text += b" " * (-len(text) % 4)
    body = struct.pack("<I4s", len(text), b"JSON") + text + data[20 + json_len :]
    return struct.pack("<4sII", b"glTF", 2, 12 + len(body)) + body


class GlbMaps(unittest.TestCase):
    def test_order_comes_from_the_attribute_index(self) -> None:
        """Prevents reporting the maps in name order instead of TEXCOORD order, which would hide the answer to Q2."""
        maps = glb_maps(glb({"TEXCOORD_1": ZETA, "TEXCOORD_0": ALPHA}))
        self.assertEqual(maps["texcoords"], ["Alpha", "Zeta"])

    def test_texture_texcoord(self) -> None:
        """Prevents misreading which TEXCOORD a texture samples: glTF's default is 0 when texCoord is absent."""
        both = {"TEXCOORD_0": ZETA, "TEXCOORD_1": ALPHA}
        self.assertEqual(glb_maps(glb(both, texture=1))["texture_texcoord"], 1)
        self.assertEqual(glb_maps(glb(both, texture=0))["texture_texcoord"], 0)
        self.assertIsNone(glb_maps(glb(both))["texture_texcoord"])
        self.assertIsNone(glb_maps(glb(both, material=False))["texture_texcoord"])

    def test_boolean_texcoord_raises(self) -> None:
        """Prevents JSON `true` passing as texCoord 1, since Python's bool is an int."""
        def boolean_texcoord(doc: dict[str, Any]) -> None:
            doc["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["texCoord"] = True

        data = with_doc(glb({"TEXCOORD_0": ZETA, "TEXCOORD_1": ALPHA}, texture=1), boolean_texcoord)
        with self.assertRaises(ValueError):
            glb_maps(data)

    def test_unknown_u_raises(self) -> None:
        """Prevents naming a map by the nearest value: a u that matches no map exactly is an export the spike didn't make."""
        for u in (0.5, 0.126):
            with self.subTest(u=u), self.assertRaises(ValueError):
                glb_maps(glb({"TEXCOORD_0": u}))

    def test_varying_u_raises(self) -> None:
        """Prevents naming a map from its first corner when the corners disagree."""
        data = bytearray(glb({"TEXCOORD_0": ZETA}))
        at = data.rindex(struct.pack("<f", ZETA))
        data[at : at + 4] = struct.pack("<f", ALPHA)
        with self.assertRaises(ValueError):
            glb_maps(bytes(data))

    def test_gap_in_texcoords_raises(self) -> None:
        """Prevents reading TEXCOORD_1 as the first set when TEXCOORD_0 is missing."""
        with self.assertRaises(ValueError):
            glb_maps(glb({"TEXCOORD_1": ZETA}))

    def test_strided_view_raises(self) -> None:
        """Prevents misreading interleaved data: the reader handles tightly packed VEC2 only."""
        with self.assertRaises(ValueError):
            glb_maps(glb({"TEXCOORD_0": ZETA}, stride=16))

    def test_bad_magic_raises(self) -> None:
        with self.assertRaises(ValueError):
            glb_maps(b"glTX" + glb({"TEXCOORD_0": ZETA})[4:])

    def test_wrong_chunk_types_raise(self) -> None:
        """Prevents reading a chunk as JSON or BIN by position alone."""
        data = glb({"TEXCOORD_0": ZETA})
        json_len = struct.unpack_from("<I", data, 12)[0]
        for at, wrong in ((16, b"JSOX"), (20 + json_len + 4, b"BIX\0")):
            with self.subTest(chunk=wrong), self.assertRaises(ValueError):
                glb_maps(data[:at] + wrong + data[at + 4 :])

    def test_non_vec2_float_accessor_raises(self) -> None:
        """Prevents reading integer or 3-component data as UVs."""
        for key, value in (("componentType", 5123), ("type", "VEC3")):
            def corrupt(doc: dict[str, Any], key: str = key, value: object = value) -> None:
                doc["accessors"][0][key] = value

            data = with_doc(glb({"TEXCOORD_0": ZETA}), corrupt)
            with self.subTest(**{key: value}), self.assertRaises(ValueError):
                glb_maps(data)

    def test_accessor_byte_offset_is_honoured(self) -> None:
        """Prevents reading from the view's start when the accessor begins later in it."""
        self.assertEqual(glb_maps(glb({"TEXCOORD_0": ALPHA}, acc_offset=8))["texcoords"], ["Alpha"])

    def test_two_primitives_raise(self) -> None:
        """Prevents reporting one primitive's sets as the whole mesh's."""
        def duplicate_primitive(doc: dict[str, Any]) -> None:
            prims = doc["meshes"][0]["primitives"]
            prims.append(dict(prims[0]))

        data = with_doc(glb({"TEXCOORD_0": ZETA}), duplicate_primitive)
        with self.assertRaises(ValueError):
            glb_maps(data)


class ObjMap(unittest.TestCase):
    def test_one_map(self) -> None:
        """Prevents a wrong answer for OBJ, which carries a single UV map."""
        self.assertEqual(obj_map("v 0 0 0\nvt 0.875000 0.1\nvt 0.875000 0.9\nf 1/1 1/2 1/1\n"), "Alpha")

    def test_no_vt_raises(self) -> None:
        with self.assertRaises(ValueError):
            obj_map("v 0 0 0\nf 1 1 1\n")

    def test_mixed_u_raises(self) -> None:
        """Prevents naming one map when the OBJ mixes both."""
        with self.assertRaises(ValueError):
            obj_map("vt 0.125 0\nvt 0.875 0\n")


if __name__ == "__main__":
    unittest.main()
