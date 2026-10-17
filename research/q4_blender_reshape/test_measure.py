# SPDX-License-Identifier: GPL-3.0-or-later
"""Oracles for measure.py on hand-built meshes, GLB bytes and OBJ text. Run: python3 -m unittest -v"""

import json
import struct
import unittest

from measure import (
    Vec,
    placed_edges,
    placed_faces,
    read_glb_polygons,
    read_obj_polygons,
    shares,
    tolerance,
    weld,
    y_up,
)

# Two quads sharing an edge: a 2 x 1 strip in the z = 0 plane.
A, B, C, D, E, F = (0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 0.0), (2.0, 1.0, 0.0)
STRIP = [[A, B, E, D], [B, C, F, E]]
EPS = 1e-6


def moved(p: Vec, dx: float) -> Vec:
    return (p[0] + dx, p[1], p[2])


class Weld(unittest.TestCase):
    def test_split_vertices_merge(self) -> None:
        """Prevents counting a glTF seam split as two vertices, which would leave the seam edge unjoined."""
        mesh = weld([[A, B, E], [A, E, D], [B, C, F], [B, F, E]], EPS)
        self.assertEqual(len(mesh.vertices), 6)
        self.assertEqual(len(mesh.edges), 9)

    def test_tolerance_is_relative_to_the_diagonal(self) -> None:
        """Prevents an absolute tolerance, which 001's weld does not use."""
        self.assertAlmostEqual(tolerance(STRIP), 1e-6 * 5**0.5)


class Placement(unittest.TestCase):
    def test_identical_meshes_place_everything(self) -> None:
        new = weld(STRIP, EPS)
        self.assertEqual(placed_edges([(B, E), (A, B)], new), [True, True])
        self.assertEqual(placed_faces(STRIP, new), [True, True])

    def test_renumbered_mesh_still_places(self) -> None:
        """Prevents matching by index: a re-export renumbers vertices and reorders faces."""
        new = weld([[F, E, B, C], [E, D, A, B]], EPS)
        self.assertEqual(placed_edges([(B, E), (C, F)], new), [True, True])
        self.assertEqual(placed_faces(STRIP, new), [True, True])

    def test_moved_vertex_beyond_tolerance_unplaces_its_edges_and_faces(self) -> None:
        """Prevents snapping to a nearby vertex, which research R5 rules out."""
        new = weld([[A, B, E, D], [B, moved(C, 10 * EPS), F, E]], EPS)
        self.assertEqual(placed_edges([(B, C), (B, E)], new), [False, True])
        self.assertEqual(placed_faces(STRIP, new), [True, False])

    def test_moved_vertex_within_tolerance_still_places(self) -> None:
        """Prevents a tolerance tighter than the weld's, which would reject f32 noise."""
        new = weld([[A, B, E, D], [B, moved(C, 0.1 * EPS), F, E]], EPS)
        self.assertEqual(placed_edges([(B, C)], new), [True])

    def test_endpoints_present_but_unjoined_is_unplaced(self) -> None:
        """Prevents placing an edge whose endpoints survive but are no longer joined (a flipped diagonal)."""
        new = weld([[A, B, D], [B, E, D]], EPS)
        self.assertEqual(placed_edges([(A, E)], new), [False])
        self.assertEqual(placed_edges([(B, D)], new), [True])


class Shares(unittest.TestCase):
    def test_seams_split_by_the_edited_region(self) -> None:
        """Prevents counting a seam that touches an edited vertex as one away from the edit."""
        new = [[A, B, E, D], [B, moved(C, 1.0), moved(F, 1.0), E]]
        got = shares(STRIP, [(A, B), (B, E), (B, C)], [C, F], new)
        self.assertEqual(got, {
            "seams_away": 2, "seams_away_placed": 2, "seams_edited": 1, "seams_edited_placed": 0,
            "faces_away": 1, "faces_away_placed": 1, "faces_edited": 1, "faces_edited_placed": 0,
        })


class Readers(unittest.TestCase):
    def test_y_up(self) -> None:
        """Prevents comparing Blender's Z-up positions with the exporters' Y-up ones."""
        self.assertEqual(y_up([1.0, 2.0, 3.0]), (1.0, 3.0, -2.0))

    def test_obj_polygons(self) -> None:
        """Prevents losing quads or reading `v/vt/vn` corners wrong."""
        text = "o q\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvn 0 0 1\ns 0\nf 1/1/1 2/1/1 3/1/1 4/1/1\n"
        self.assertEqual(read_obj_polygons(text), [[A, B, E, D]])

    def test_obj_unknown_line_raises(self) -> None:
        with self.assertRaises(ValueError):
            read_obj_polygons("v 0 0 0\nl 1 1\n")

    def test_glb_polygons_across_primitives(self) -> None:
        """Prevents reading only the first primitive, since Blender writes one per material."""
        blob = struct.pack("<9f", *A, *B, *E) + struct.pack("<3H", 0, 1, 2) + b"\0\0"
        blob += struct.pack("<9f", *B, *C, *F) + struct.pack("<3H", 0, 1, 2) + b"\0\0"
        views, accessors, prims = [], [], []
        for base in (0, 44):
            views += [
                {"buffer": 0, "byteOffset": base, "byteLength": 36},
                {"buffer": 0, "byteOffset": base + 36, "byteLength": 6},
            ]
            accessors += [
                {"bufferView": len(views) - 2, "componentType": 5126, "count": 3, "type": "VEC3"},
                {"bufferView": len(views) - 1, "componentType": 5123, "count": 3, "type": "SCALAR"},
            ]
            prims.append({"attributes": {"POSITION": len(accessors) - 2}, "indices": len(accessors) - 1})
        doc = {"asset": {"version": "2.0"}, "buffers": [{"byteLength": len(blob)}], "bufferViews": views,
               "accessors": accessors, "meshes": [{"primitives": prims}]}
        text = json.dumps(doc).encode()
        text += b" " * (-len(text) % 4)
        body = struct.pack("<I4s", len(text), b"JSON") + text + struct.pack("<I4s", len(blob), b"BIN\0") + blob
        glb = struct.pack("<4sII", b"glTF", 2, 12 + len(body)) + body
        self.assertEqual(read_glb_polygons(glb), [[A, B, E], [B, C, F]])

    def test_glb_bad_magic_raises(self) -> None:
        with self.assertRaises(ValueError):
            read_glb_polygons(b"glTX" + b"\0" * 24)


if __name__ == "__main__":
    unittest.main()
