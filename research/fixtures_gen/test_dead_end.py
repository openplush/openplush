"""Oracles for the dead-end fixture: UV cuts that stop inside the surface, beside cuts that end properly.

Run: python3 -m unittest -v   (from this directory)
Reads the committed files only and shares no code with generate.py or source_seams.py.
A tip follows the seam graph of 003 research R3: cuts, material boundaries and the surface boundary.
"""

import json
import unittest
from collections import Counter

from test_fixtures import FIXTURES, Face, Vec, array, edges_of, exact_int, field, read_seam_set, text
from test_source_materials import material_boundaries, read_obj_materials
from test_source_seams import (
    SOURCE,
    UV_TOL,
    accessor_floats,
    accessor_ints,
    match_positions,
    read_glb,
    read_obj_source,
    uv_cuts,
    with_uvs,
)

Edge = tuple[int, int]
OBJ = SOURCE / "dead-end.obj"


def surface_boundary(faces: list[Face]) -> set[Edge]:
    count = Counter((min(a, b), max(a, b)) for f in faces for a, b in edges_of(f))
    return {e for e, n in count.items() if n == 1}


def tips(seams: set[Edge], boundary_vertices: set[int]) -> set[int]:
    """Vertices on exactly one seam edge that are not on the surface boundary."""
    degree = Counter(v for e in seams for v in e)
    return {v for v, n in degree.items() if n == 1 and v not in boundary_vertices}


def read_tips(path: object) -> tuple[str, list[int]]:
    doc: object = json.loads(FIXTURES.joinpath(str(path)).read_text(encoding="utf-8"))
    header = (text(field(doc, "format"), "format"), exact_int(field(doc, "version"), "version"))
    if header != ("openplush-dead-ends", 1) or exact_int(field(doc, "vertexIndexBase"), "base") != 0:
        raise ValueError(f"unsupported dead-end header {header!r}")
    found = [exact_int(t, "tip") for t in array(field(doc, "tips"), "tips")]
    return text(field(doc, "mesh"), "mesh"), found


class DeadEnd(unittest.TestCase):
    """The capsule with an opening at the bottom, two materials, one properly ended cut and two dead ends."""

    def load(self) -> tuple[list[Vec], list[Face], set[Edge], set[Edge], set[Edge]]:
        pos, runs, _ = read_obj_materials(OBJ)
        faces = [f for _, fs in runs for f in fs]
        _, source_runs, _ = read_obj_source(OBJ)
        cuts, _ = uv_cuts(with_uvs([f for _, fs in source_runs for f in fs]))
        return pos, faces, cuts, material_boundaries(runs), surface_boundary(faces)

    def test_uv_cuts_are_exactly_the_reference_seam_set(self) -> None:
        """Prevents a fixture whose UVs and recorded cuts disagree."""
        seam_set = read_seam_set(FIXTURES / "seams" / "non-disk" / "dead-end.json")
        self.assertEqual(seam_set.mesh, "meshes/source-seams/dead-end.obj")
        self.assertEqual(seam_set.kinds, {"cut"})
        _, _, cuts, _, _ = self.load()
        self.assertEqual(sorted(cuts), seam_set.edges)
        self.assertEqual(len(cuts), 9)

    def test_the_opening_and_the_material_boundary_are_full_rings(self) -> None:
        """Prevents a fixture whose cuts have nothing to end on, so the not-a-dead-end cases would be missing."""
        pos, _, _, materials, boundary = self.load()
        self.assertEqual(len(boundary), 12, "one opening, a 12-edge loop")
        self.assertEqual({pos[v][2] for e in boundary for v in e}, {pos[next(iter(boundary))[0]][2]}, "at one height")
        self.assertEqual(len(materials), 12)
        self.assertEqual({pos[v][2] for e in materials for v in e}, {30.0}, "the tube's upper rim")

    def test_exactly_the_recorded_tips_are_dead_ends(self) -> None:
        """Prevents a dead-end oracle that disagrees with the cuts, or that lists a cut ending on a seam or opening."""
        mesh, recorded = read_tips("seams/non-disk/dead-end.tips.json")
        self.assertEqual(mesh, "meshes/source-seams/dead-end.obj")
        self.assertEqual(recorded, sorted(set(recorded)))
        _, _, cuts, materials, boundary = self.load()
        on_boundary = {v for e in boundary for v in e}
        self.assertEqual(sorted(tips(cuts | materials, on_boundary)), recorded)
        self.assertEqual(len(recorded), 2)

    def test_both_ways_a_cut_may_end_are_exercised(self) -> None:
        """Prevents a fixture where the material boundary or the opening never actually ends a cut."""
        _, _, cuts, materials, boundary = self.load()
        on_boundary = {v for e in boundary for v in e}
        proper = tips(cuts | materials, on_boundary)
        self.assertEqual(len(tips(cuts, on_boundary) - proper), 2, "two cuts end on the material boundary")
        self.assertEqual(len(tips(cuts | materials, set()) - proper), 1, "one cut ends on the opening")

    def test_the_gltf_has_the_obj_runs_as_primitives_with_their_uvs(self) -> None:
        """Prevents a glTF twin with other materials, triangles or UVs than the OBJ."""
        obj_pos, runs, _ = read_obj_source(OBJ)
        doc, blob = read_glb(SOURCE / "dead-end.glb")
        names = [field(m, "name") for m in array(field(doc, "materials"), "materials")]
        self.assertEqual(names, [name for name, _ in runs])
        prims = array(field(array(field(doc, "meshes"), "meshes")[0], "primitives"), "primitives")
        for k, (p, (_, faces)) in enumerate(zip(prims, runs, strict=True)):
            self.assertEqual(exact_int(field(p, "material"), "material"), k)
            attrs = field(p, "attributes")
            pos = [(q[0], q[1], q[2]) for q in accessor_floats(doc, blob, exact_int(field(attrs, "POSITION"), "P"), 3)]
            tex = accessor_floats(doc, blob, exact_int(field(attrs, "TEXCOORD_0"), "T"), 2)
            idx = accessor_ints(doc, blob, exact_int(field(p, "indices"), "indices"))
            to_obj = match_positions(pos, obj_pos)
            for n, face in enumerate(with_uvs(faces)):
                for c, (v, (u, w)) in enumerate(face):
                    g = idx[3 * n + c]
                    self.assertEqual(to_obj[g], v, f"primitive {k} face {n} corner {c}")
                    self.assertLess(max(abs(tex[g][0] - u), abs(tex[g][1] - (1.0 - w))), UV_TOL)


if __name__ == "__main__":
    unittest.main()
