"""Oracles for the source-seam fixtures: UV-cut models written the way Blender's exporters write them.

Run: python3 -m unittest -v   (from this directory)
Reads the committed files only and shares no code with generate.py or source_seams.py.
"""

import json
import math
import struct
import unittest
from pathlib import Path

from test_fixtures import FIXTURES, Face, Vec, array, chart_topology, edges_of, exact_int, field, read_seam_set

UV = tuple[float, float]
Corner = tuple[int, UV]  # (OBJ vertex index, UV)

SOURCE = FIXTURES / "meshes" / "source-seams"
UV_TOL = 1e-6  # the OBJ prints 6 decimals; the glTF holds f32, which is finer than that at these magnitudes
POS_TOL = 1e-4  # mm; f32 spacing near 90 mm is about 8e-6


def read_obj_uv(path: Path) -> tuple[list[Vec], list[tuple[Corner, Corner, Corner]]]:
    """`v`, `vt` and `f v/vt` lines only; anything else is an error, never skipped."""
    pos: list[Vec] = []
    uvs: list[UV] = []
    faces: list[tuple[Corner, Corner, Corner]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        word, *rest = line.split() or [""]
        if word == "v":
            x, y, z = (float(s) for s in rest)
            pos.append((x, y, z))
        elif word == "vt":
            u, v = (float(s) for s in rest)
            uvs.append((u, v))
        elif word == "f":
            corners: list[Corner] = []
            for s in rest:
                vi, ti = (int(n) for n in s.split("/"))
                if not (1 <= vi <= len(pos) and 1 <= ti <= len(uvs)):
                    raise ValueError(f"{path.name}: corner {s!r} indexes outside the v or vt lines before it")
                corners.append((vi - 1, uvs[ti - 1]))
            a, b, c = corners
            faces.append((a, b, c))
        elif word not in ("", "#"):
            raise ValueError(f"{path.name}: unexpected OBJ line {line!r}")
    return pos, faces


def uv_cuts(faces: list[tuple[Corner, Corner, Corner]]) -> tuple[set[tuple[int, int]], int]:
    """Interior edges whose corner UVs differ across the edge at either endpoint; also counts boundary edges."""
    sides: dict[tuple[int, int], list[dict[int, UV]]] = {}
    for face in faces:
        uv_at = dict(face)
        f: Face = (face[0][0], face[1][0], face[2][0])
        for a, b in edges_of(f):
            sides.setdefault((min(a, b), max(a, b)), []).append(uv_at)
    cuts: set[tuple[int, int]] = set()
    boundary = 0
    for edge, both in sides.items():
        if len(both) == 1:
            boundary += 1
            continue
        if len(both) != 2:
            raise ValueError(f"edge {edge} has {len(both)} faces")
        left, right = both
        if any(abs(left[v][k] - right[v][k]) > UV_TOL for v in edge for k in (0, 1)):
            cuts.add(edge)
    return cuts, boundary


def accessor_view(doc: object, index: int, kind: str, component: int) -> tuple[int, int]:
    """(count, byte offset) of a tightly packed accessor, after checking its type and component type."""
    acc = array(field(doc, "accessors"), "accessors")[index]
    if (field(acc, "type"), exact_int(field(acc, "componentType"), "componentType")) != (kind, component):
        raise ValueError(f"accessor {index}: expected {kind} of component type {component}")
    view = array(field(doc, "bufferViews"), "bufferViews")[exact_int(field(acc, "bufferView"), "bufferView")]
    return exact_int(field(acc, "count"), "count"), exact_int(field(view, "byteOffset"), "byteOffset")


def accessor_floats(doc: object, blob: bytes, index: int, width: int) -> list[tuple[float, ...]]:
    count, start = accessor_view(doc, index, f"VEC{width}", 5126)
    flat = struct.unpack_from(f"<{count * width}f", blob, start)
    return [tuple(flat[i : i + width]) for i in range(0, len(flat), width)]


def accessor_ints(doc: object, blob: bytes, index: int) -> list[int]:
    count, start = accessor_view(doc, index, "SCALAR", 5125)
    return list(struct.unpack_from(f"<{count}I", blob, start))


def read_glb(path: Path) -> tuple[object, bytes]:
    """Checks the GLB container (header, two 4-byte-aligned chunks) and returns the JSON and the binary chunk."""
    data = path.read_bytes()
    magic, version, length = struct.unpack_from("<4sII", data, 0)
    if (magic, version, length) != (b"glTF", 2, len(data)):
        raise ValueError(f"{path.name}: bad GLB header {(magic, version, length)}")
    js_len, js_type = struct.unpack_from("<I4s", data, 12)
    bin_at = 20 + js_len
    bin_len, bin_type = struct.unpack_from("<I4s", data, bin_at)
    if (js_type, bin_type) != (b"JSON", b"BIN\0") or js_len % 4 or bin_len % 4 or bin_at + 8 + bin_len != len(data):
        raise ValueError(f"{path.name}: bad GLB chunks")
    doc: object = json.loads(data[20:bin_at])
    return doc, data[bin_at + 8 :]


def glb_primitive(path: Path) -> tuple[object, int, list[Vec], list[UV], list[int]]:
    """The document, the POSITION accessor index, and the one primitive's positions, UVs and indices."""
    doc, blob = read_glb(path)
    mesh = array(field(doc, "meshes"), "meshes")[0]
    (prim,) = array(field(mesh, "primitives"), "primitives")
    attrs = field(prim, "attributes")
    at = exact_int(field(attrs, "POSITION"), "POSITION")
    pos = accessor_floats(doc, blob, at, 3)
    tex = accessor_floats(doc, blob, exact_int(field(attrs, "TEXCOORD_0"), "TEXCOORD_0"), 2)
    idx = accessor_ints(doc, blob, exact_int(field(prim, "indices"), "indices"))
    return doc, at, [(p[0], p[1], p[2]) for p in pos], [(t[0], t[1]) for t in tex], idx


def glb_normals(path: Path) -> list[Vec]:
    doc, blob = read_glb(path)
    (prim,) = array(field(array(field(doc, "meshes"), "meshes")[0], "primitives"), "primitives")
    normals = accessor_floats(doc, blob, exact_int(field(field(prim, "attributes"), "NORMAL"), "NORMAL"), 3)
    return [(n[0], n[1], n[2]) for n in normals]


def match_positions(pos: list[Vec], obj_pos: list[Vec]) -> list[int]:
    """For each glTF vertex, the one OBJ vertex at the same position; raises unless there is exactly one."""
    out: list[int] = []
    for p in pos:
        near = [i for i, q in enumerate(obj_pos) if max(abs(p[k] - q[k]) for k in range(3)) < POS_TOL]
        if len(near) != 1:
            raise ValueError(f"glTF vertex {p} matches {len(near)} OBJ vertices")
        out.append(near[0])
    return out


def check_gltf_twin(tc: unittest.TestCase, obj: Path, glb: Path) -> None:
    """The glTF has the OBJ's triangles and corner UVs (V flipped), with one vertex per distinct (position, UV)."""
    obj_pos, faces = read_obj_uv(obj)
    _, _, pos, tex, idx = glb_primitive(glb)
    tc.assertEqual(len(idx), 3 * len(faces))
    to_obj = match_positions(pos, obj_pos)
    for n, face in enumerate(faces):
        for c, (v, (u, w)) in enumerate(face):
            g = idx[3 * n + c]
            tc.assertEqual(to_obj[g], v, f"face {n} corner {c}")
            # glTF's UV origin is top-left, OBJ's bottom-left; Blender flips V on export.
            tc.assertLess(max(abs(tex[g][0] - u), abs(tex[g][1] - (1.0 - w))), UV_TOL)
    distinct = {(v, uv) for face in faces for v, uv in face}
    tc.assertEqual(len(pos), len(distinct), "one glTF vertex per distinct (position, UV), no more")
    tc.assertEqual(set(idx), set(range(len(pos))), "every glTF vertex is used")
    tc.assertEqual(len({(to_obj[g], tex[g]) for g in range(len(pos))}), len(pos), "no two share a pair")


def check_position_bounds(tc: unittest.TestCase, glb: Path) -> None:
    doc, at, pos, _, _ = glb_primitive(glb)
    acc = array(field(doc, "accessors"), "accessors")[at]
    tc.assertEqual(field(acc, "min"), [min(p[k] for p in pos) for k in range(3)])
    tc.assertEqual(field(acc, "max"), [max(p[k] for p in pos) for k in range(3)])


class CapsuleUv(unittest.TestCase):
    """The capsule cut along one meridian and one ring, as an OBJ and as a Blender-style glTF."""

    def test_uv_cuts_in_the_obj_are_exactly_the_reference_seam_set(self) -> None:
        """Prevents a fixture whose UVs and recorded layout disagree, which would leave seam-import accuracy unmeasurable."""
        seam_set = read_seam_set(FIXTURES / "seams" / "capsule-uv.json")
        self.assertEqual(seam_set.mesh, "meshes/source-seams/capsule-uv.obj")
        self.assertEqual(seam_set.kinds, {"cut"})
        _, faces = read_obj_uv(FIXTURES / seam_set.mesh)
        cuts, boundary = uv_cuts(faces)
        self.assertEqual(boundary, 0, "closed surface")
        self.assertEqual(sorted(cuts), seam_set.edges)

    def test_the_layout_is_one_meridian_and_the_upper_tube_rim(self) -> None:
        """Prevents the generator changing the cut layout and its seam set together, unnoticed."""
        pos, faces = read_obj_uv(SOURCE / "capsule-uv.obj")
        cuts, _ = uv_cuts(faces)

        def on_meridian(v: int) -> bool:  # meridian 0 lies in the half-plane y = 0, x >= 0
            return pos[v][1] == 0.0 and pos[v][0] >= 0.0

        rim_z = max(p[2] for p in pos if abs(math.hypot(p[0], p[1]) - 30.0) < POS_TOL)  # highest full-radius ring
        meridian = {e for e in cuts if on_meridian(e[0]) and on_meridian(e[1])}
        rim = {e for e in cuts if pos[e[0]][2] == rim_z == pos[e[1]][2]}
        self.assertEqual(rim_z, 30.0, "the tube's upper rim, where it meets the hemisphere")
        self.assertEqual((len(meridian), len(rim), meridian | rim), (9, 12, cuts), "pole to pole, plus one full ring")
        tris: list[Face] = [(f[0][0], f[1][0], f[2][0]) for f in faces]
        self.assertEqual(chart_topology(tris, cuts), [(1, 1), (1, 1)], "two disk-shaped islands")

    def test_the_cut_rule_sees_a_cut_that_differs_at_one_endpoint_only(self) -> None:
        """Prevents an oracle that only compares whole edges and misses a cut running into a junction."""
        same, other = (0.0, 0.0), (0.5, 0.0)
        left = ((0, same), (1, same), (2, same))
        right = ((1, other), (0, same), (3, same))
        self.assertEqual(uv_cuts([left, right])[0], {(0, 1)})
        self.assertEqual(uv_cuts([left, ((1, same), (0, same), (3, same))])[0], set())

    def test_the_gltf_is_the_obj_split_where_corner_uvs_differ(self) -> None:
        """Prevents a glTF twin with other geometry, other UVs, or splits that a Blender export would not make."""
        check_gltf_twin(self, SOURCE / "capsule-uv.obj", SOURCE / "capsule-uv.glb")

    def test_position_bounds_match_the_data(self) -> None:
        """Prevents a glTF that strict loaders reject: POSITION min and max are required and must be exact."""
        check_position_bounds(self, SOURCE / "capsule-uv.glb")


class CapsuleHardshade(unittest.TestCase):
    """The capsule-uv layout exported with a flat-shaded lower hemisphere, so vertices also split for normals."""

    def corners(self) -> tuple[list[tuple[Corner, Corner, Corner]], list[tuple[Vec, Vec, Vec]]]:
        """Each glTF face as OBJ-indexed corners with OBJ-oriented UVs, and the face's corner normals."""
        obj_pos, obj_faces = read_obj_uv(SOURCE / "capsule-uv.obj")
        _, _, pos, tex, idx = glb_primitive(SOURCE / "capsule-hardshade.glb")
        normals = glb_normals(SOURCE / "capsule-hardshade.glb")
        to_obj = match_positions(pos, obj_pos)
        faces: list[tuple[Corner, Corner, Corner]] = []
        face_normals: list[tuple[Vec, Vec, Vec]] = []
        for n in range(0, len(idx), 3):
            a, b, c = ((to_obj[g], (round(tex[g][0], 6), round(1.0 - tex[g][1], 6))) for g in idx[n : n + 3])
            faces.append((a, b, c))
            face_normals.append((normals[idx[n]], normals[idx[n + 1]], normals[idx[n + 2]]))
        for n, (mine, theirs) in enumerate(zip(faces, obj_faces, strict=True)):
            for (v, uv), (w, obj_uv) in zip(mine, theirs):
                self.assertEqual(v, w, f"face {n}: same triangle as capsule-uv")
                self.assertLess(max(abs(uv[0] - obj_uv[0]), abs(uv[1] - obj_uv[1])), UV_TOL, f"face {n}: same UVs")
        distinct = {(v, uv, nrm) for f, ns in zip(faces, face_normals) for (v, uv), nrm in zip(f, ns)}
        self.assertEqual(len(normals), len(distinct), "one glTF vertex per distinct (position, UV, normal), no more")
        return faces, face_normals

    def test_normal_splits_add_no_uv_cut(self) -> None:
        """Prevents a hard-shading fixture whose UV cuts differ from capsule-uv, which would not isolate shading splits."""
        faces, _ = self.corners()
        cuts, _ = uv_cuts(faces)
        self.assertEqual(sorted(cuts), read_seam_set(FIXTURES / "seams" / "capsule-uv.json").edges)

    def test_it_has_hard_edges_that_are_not_uv_cuts(self) -> None:
        """Prevents a fixture with no normal-only splits, which would let a cut detector that reads normals pass."""
        faces, face_normals = self.corners()
        cuts, _ = uv_cuts(faces)
        normal_at: dict[tuple[int, int], list[dict[int, Vec]]] = {}
        for f, ns in zip(faces, face_normals):
            at = {v: nrm for (v, _), nrm in zip(f, ns)}
            for e in edges_of((f[0][0], f[1][0], f[2][0])):
                normal_at.setdefault((min(e), max(e)), []).append(at)
        hard = {e for e, (l, r) in normal_at.items() if any(max(abs(l[v][k] - r[v][k]) for k in range(3)) > 1e-3 for v in e)}
        self.assertGreater(len(hard - cuts), 50, "many shading-only splits")
        self.assertGreater(len(glb_normals(SOURCE / "capsule-hardshade.glb")), len(glb_primitive(SOURCE / "capsule-uv.glb")[2]))

    def test_normals_are_unit_length(self) -> None:
        """Prevents a glTF that validators reject: NORMAL must be unit length."""
        for n in glb_normals(SOURCE / "capsule-hardshade.glb"):
            self.assertAlmostEqual(math.sqrt(sum(c * c for c in n)), 1.0, places=5)



class SphereBand(unittest.TestCase):
    """A sphere cut around two rings, so the band between them is an annulus that cannot lie flat in one piece."""

    def test_uv_cuts_in_the_obj_are_exactly_the_reference_seam_set(self) -> None:
        """Prevents a fixture whose UVs and recorded layout disagree."""
        seam_set = read_seam_set(FIXTURES / "seams" / "non-disk" / "sphere-band.json")
        self.assertEqual(seam_set.mesh, "meshes/source-seams/sphere-band.obj")
        self.assertEqual(seam_set.kinds, {"cut"})
        _, faces = read_obj_uv(FIXTURES / seam_set.mesh)
        cuts, boundary = uv_cuts(faces)
        self.assertEqual(boundary, 0, "closed surface")
        self.assertEqual(sorted(cuts), seam_set.edges)

    def test_the_layout_is_two_full_rings_leaving_two_caps_and_a_band(self) -> None:
        """Prevents a band that is secretly a disk, which would let a closing-seam proposal never be exercised."""
        pos, faces = read_obj_uv(SOURCE / "sphere-band.obj")
        cuts, _ = uv_cuts(faces)
        heights = sorted({pos[v][2] for e in cuts for v in e})
        self.assertEqual(len(heights), 2, "every cut vertex lies on one of two rings")
        self.assertAlmostEqual(heights[0], -heights[1], places=5)
        for z in heights:
            self.assertEqual(len([e for e in cuts if pos[e[0]][2] == z == pos[e[1]][2]]), 12, f"full ring at z = {z}")
        self.assertEqual(len(cuts), 24)
        tris: list[Face] = [(f[0][0], f[1][0], f[2][0]) for f in faces]
        self.assertEqual(sorted(chart_topology(tris, cuts)), [(0, 2), (1, 1), (1, 1)], "an annulus and two disks")

    def test_the_gltf_is_the_obj_split_where_corner_uvs_differ(self) -> None:
        """Prevents a glTF twin with other geometry, other UVs, or splits that a Blender export would not make."""
        check_gltf_twin(self, SOURCE / "sphere-band.obj", SOURCE / "sphere-band.glb")

    def test_position_bounds_match_the_data(self) -> None:
        """Prevents a glTF that strict loaders reject."""
        check_position_bounds(self, SOURCE / "sphere-band.glb")


if __name__ == "__main__":
    unittest.main()
