"""Oracles for the material fixtures: models whose material assignment is their fabric layout.

Run: python3 -m unittest -v   (from this directory)
Reads the committed files only and shares no code with generate.py or source_seams.py.
PRESETS are the shipped fabric presets of 001 research R7, which import matches by trimmed, lower-cased name.
"""

import unittest
from collections import Counter
from pathlib import Path

from test_fixtures import FIXTURES, Face, Vec, array, chart_topology, edges_of, exact_int, field, read_obj, read_seam_set
from test_source_seams import SOURCE, accessor_floats, accessor_ints, match_positions, read_glb, read_obj_source

PRESETS = ("Quilting cotton", "Craft felt", "Polar fleece", "Minky / cuddle", "Velboa", "Short faux fur", "Long faux fur")
NAMES = ["minky / cuddle", "Polar fleece ", "Felt Nose"]


def read_obj_materials(path: Path) -> tuple[list[Vec], list[tuple[str, list[Face]]], int]:
    """Positions, the named `usemtl` runs with plain triangles, and the number of `o` lines.

    Faces before any `usemtl` are an error here, because every face of these fixtures has a material.
    """
    pos, runs, objects = read_obj_source(path)
    named: list[tuple[str, list[Face]]] = []
    for name, faces in runs:
        if name is None:
            raise ValueError(f"{path.name}: a face before any usemtl")
        named.append((name, [(a[0], b[0], c[0]) for a, b, c in faces]))
    return pos, named, objects


def material_boundaries(runs: list[tuple[str, list[Face]]]) -> set[tuple[int, int]]:
    """Interior edges whose two faces come from different runs; raises on an edge with more than two faces."""
    owner: dict[tuple[int, int], list[int]] = {}
    for r, (_, faces) in enumerate(runs):
        for f in faces:
            for a, b in edges_of(f):
                owner.setdefault((min(a, b), max(a, b)), []).append(r)
    if any(len(rs) > 2 for rs in owner.values()):
        raise ValueError("not a two-manifold")
    return {e for e, rs in owner.items() if len(rs) == 2 and rs[0] != rs[1]}


class ThreeMaterials(unittest.TestCase):
    """The capsule in three materials, top hemisphere to bottom, with no UVs."""

    def test_obj_switches_material_twice_inside_one_object(self) -> None:
        """Prevents an OBJ that never puts two materials in one object, which is the case a reader must split."""
        _, runs, objects = read_obj_materials(SOURCE / "three-materials.obj")
        _, source_runs, _ = read_obj_source(SOURCE / "three-materials.obj")
        self.assertEqual({uv for _, fs in source_runs for f in fs for _, uv in f}, {None}, "no UVs")
        self.assertEqual(objects, 1)
        self.assertEqual([name for name, _ in runs], NAMES)
        self.assertTrue(all(faces for _, faces in runs), "every run has faces")

    def test_two_names_match_a_preset_and_one_matches_none(self) -> None:
        """Prevents names that cover only the matched case or only the unmatched one."""
        folded = {p.strip().lower() for p in PRESETS}
        matched = [n for n in NAMES if n.strip().lower() in folded]
        self.assertEqual(matched, ["minky / cuddle", "Polar fleece "])
        self.assertTrue(all(n not in PRESETS for n in matched), "a match needs the case or space folding")

    def test_material_boundaries_are_exactly_the_reference_seam_set(self) -> None:
        """Prevents a recorded boundary layout that disagrees with the materials, or regions that are not as described."""
        seam_set = read_seam_set(FIXTURES / "seams" / "non-disk" / "three-materials.json")
        self.assertEqual(seam_set.mesh, "meshes/source-seams/three-materials.obj")
        self.assertEqual(seam_set.kinds, {"fabricBoundary"})
        pos, runs, _ = read_obj_materials(FIXTURES / seam_set.mesh)
        boundaries = material_boundaries(runs)
        self.assertEqual(sorted(boundaries), seam_set.edges)
        heights = sorted({pos[v][2] for e in boundaries for v in e})
        self.assertEqual(heights, [-30.0, 30.0], "the tube's two rims")
        self.assertEqual(len(boundaries), 24, "two full 12-edge rings")
        faces = [f for _, fs in runs for f in fs]
        uses = Counter((min(a, b), max(a, b)) for f in faces for a, b in edges_of(f))
        self.assertEqual(set(uses.values()), {2}, "closed surface")
        self.assertEqual(sorted(chart_topology(faces, boundaries)), [(0, 2), (1, 1), (1, 1)], "tube and two caps")

    def test_the_gltf_has_one_primitive_per_material_in_obj_order(self) -> None:
        """Prevents a glTF twin with other materials, merged primitives, or geometry other than the OBJ's."""
        obj_pos, runs, _ = read_obj_materials(SOURCE / "three-materials.obj")
        doc, blob = read_glb(SOURCE / "three-materials.glb")
        names = [field(m, "name") for m in array(field(doc, "materials"), "materials")]
        self.assertEqual(names, NAMES, "names exactly as written, trailing space included")
        prims = array(field(array(field(doc, "meshes"), "meshes")[0], "primitives"), "primitives")
        self.assertEqual([exact_int(field(p, "material"), "material") for p in prims], [0, 1, 2])
        for p, (_, faces) in zip(prims, runs, strict=True):
            attrs = field(p, "attributes")
            if not isinstance(attrs, dict):
                raise TypeError("attributes")
            self.assertEqual(sorted(attrs), ["POSITION"], "no UVs, so no TEXCOORD_0")
            at = exact_int(attrs["POSITION"], "POSITION")
            pos = [(q[0], q[1], q[2]) for q in accessor_floats(doc, blob, at, 3)]
            idx = accessor_ints(doc, blob, exact_int(field(p, "indices"), "indices"))
            to_obj = match_positions(pos, obj_pos)
            tris = [(to_obj[idx[n]], to_obj[idx[n + 1]], to_obj[idx[n + 2]]) for n in range(0, len(idx), 3)]
            self.assertEqual(tris, faces)
            self.assertEqual(len(pos), len({v for f in faces for v in f}), "one vertex per position in the primitive")
            acc = array(field(doc, "accessors"), "accessors")[at]
            self.assertEqual(field(acc, "min"), [min(q[k] for q in pos) for k in range(3)])
            self.assertEqual(field(acc, "max"), [max(q[k] for q in pos) for k in range(3)])



class NoUv(unittest.TestCase):
    """The capsule with one material and no texture coordinates."""

    def test_one_material_no_uvs_and_the_capsule_geometry(self) -> None:
        """Prevents a no-UV fixture that has UVs, more than one material, or other geometry than the plain capsule."""
        pos, runs, objects = read_obj_source(SOURCE / "no-uv.obj")
        self.assertEqual((objects, [name for name, _ in runs]), (1, ["Body"]))
        self.assertEqual({uv for _, fs in runs for f in fs for _, uv in f}, {None}, "no UVs")
        capsule_pos, capsule_faces = read_obj(FIXTURES / "meshes" / "capsule.obj")
        self.assertEqual(pos, capsule_pos)
        self.assertEqual([(a[0], b[0], c[0]) for _, fs in runs for a, b, c in fs], capsule_faces)


if __name__ == "__main__":
    unittest.main()
