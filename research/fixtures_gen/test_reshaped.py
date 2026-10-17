"""Oracles for the re-import fixtures: one model before and after an edit that reshapes one end cap only.

Run: python3 -m unittest -v   (from this directory)
Reads the committed files only and shares no code with generate.py or the source_*.py modules.
Re-placement by position, not by provenance key, is 003 research R5.
"""

import unittest
from pathlib import Path

from test_fixtures import FIXTURES, Vec, array, exact_int, field, read_seam_set
from test_source_seams import (
    SOURCE,
    UV,
    Corner,
    UV_TOL,
    accessor_floats,
    accessor_ints,
    match_positions,
    read_glb,
    read_obj_uv,
    uv_cuts,
)

CornerAt = tuple[Vec, UV]  # a corner by its exact f32 position, and its UV as stored (V flipped)
LowerRim = -30.0  # z of the tube's lower rim, where the lower cap starts


def read_corners(path: Path) -> tuple[list[str], list[tuple[CornerAt, CornerAt, CornerAt]], list[int]]:
    """Material names, every face's corners across all primitives, and each face's material index."""
    doc, blob = read_glb(path)
    names = [str(field(m, "name")) for m in array(field(doc, "materials"), "materials")]
    faces: list[tuple[CornerAt, CornerAt, CornerAt]] = []
    materials: list[int] = []
    for p in array(field(array(field(doc, "meshes"), "meshes")[0], "primitives"), "primitives"):
        attrs = field(p, "attributes")
        pos = accessor_floats(doc, blob, exact_int(field(attrs, "POSITION"), "POSITION"), 3)
        tex = accessor_floats(doc, blob, exact_int(field(attrs, "TEXCOORD_0"), "TEXCOORD_0"), 2)
        idx = accessor_ints(doc, blob, exact_int(field(p, "indices"), "indices"))
        for n in range(0, len(idx), 3):
            a, b, c = (((pos[g][0], pos[g][1], pos[g][2]), (tex[g][0], tex[g][1])) for g in idx[n : n + 3])
            faces.append((a, b, c))
            materials.append(exact_int(field(p, "material"), "material"))
    return names, faces, materials


def cut_positions(faces: list[tuple[CornerAt, CornerAt, CornerAt]]) -> set[frozenset[Vec]]:
    """The UV cuts, each as the pair of its endpoint positions; corners are welded by exact position."""
    ids: dict[Vec, int] = {}
    indexed: list[tuple[Corner, Corner, Corner]] = []
    for (pa, ua), (pb, ub), (pc, uc) in faces:
        indexed.append(((ids.setdefault(pa, len(ids)), ua), (ids.setdefault(pb, len(ids)), ub), (ids.setdefault(pc, len(ids)), uc)))
    cuts, _ = uv_cuts(indexed)
    where = {i: p for p, i in ids.items()}
    return {frozenset((where[a], where[b])) for a, b in cuts}


class Reshaped(unittest.TestCase):
    """capsule-uv in two materials, as v1 and as v2 with the lower cap stretched downwards."""

    def test_v1_is_capsule_uv_in_two_materials(self) -> None:
        """Prevents a v1 whose seams are not the known layout, which would leave re-placement without an answer."""
        names, faces, materials = read_corners(SOURCE / "reshaped-v1.glb")
        self.assertEqual(names, ["Head", "Body"])
        obj_pos, _ = read_obj_uv(SOURCE / "capsule-uv.obj")
        positions = sorted({p for f in faces for p, _ in f})
        to_obj = dict(zip(positions, match_positions(positions, obj_pos)))
        cuts = {tuple(sorted(to_obj[p] for p in pair)) for pair in cut_positions(faces)}
        self.assertEqual(sorted(cuts), read_seam_set(FIXTURES / "seams" / "capsule-uv.json").edges)
        head = {p for f, m in zip(faces, materials) if m == 0 for p, _ in f}
        self.assertAlmostEqual(min(p[2] for p in head), 30.0, places=4, msg="Head is the upper hemisphere, down to the rim")

    def test_v2_moves_only_the_lower_cap(self) -> None:
        """Prevents an edit that moves vertices outside the cap, which would make the placeable count arbitrary."""
        _, v1, m1 = read_corners(SOURCE / "reshaped-v1.glb")
        _, v2, m2 = read_corners(SOURCE / "reshaped-v2.glb")
        self.assertEqual((len(v1), m1), (len(v2), m2), "same faces in the same materials")
        for n, (f1, f2) in enumerate(zip(v1, v2, strict=True)):
            for (p1, uv1), (p2, uv2) in zip(f1, f2):
                self.assertLess(max(abs(uv1[0] - uv2[0]), abs(uv1[1] - uv2[1])), UV_TOL, f"face {n}: same UVs")
                if p1[2] >= LowerRim:
                    self.assertEqual(p1, p2, f"face {n}: outside the cap, bit-identical")
                else:
                    self.assertLess(p2[2], p1[2] - 1.0, f"face {n}: the cap is stretched down")

    def test_exactly_the_seams_away_from_the_cap_replace_by_position(self) -> None:
        """Prevents a fixture where every seam, or none, would re-place: the known answer is 18 of 21."""
        _, v1, _ = read_corners(SOURCE / "reshaped-v1.glb")
        _, v2, _ = read_corners(SOURCE / "reshaped-v2.glb")
        old = cut_positions(v1)
        new_positions = {p for f in v2 for p, _ in f}
        placed = {pair for pair in old if pair <= new_positions}
        self.assertEqual((len(old), len(placed)), (21, 18))
        self.assertTrue(all(min(p[2] for p in pair) < LowerRim for pair in old - placed), "only edges touching the cap")
        self.assertEqual(placed, cut_positions(v2) & placed, "re-placed seams are still cuts in v2")


if __name__ == "__main__":
    unittest.main()
