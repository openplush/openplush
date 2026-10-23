# SPDX-License-Identifier: GPL-3.0-or-later
"""Builds Blender scenes with known seams, unwraps them, and exports each as GLB and OBJ.

GPL-3.0-or-later, because it imports Blender's `bpy` (ADR 0011). Run with the pinned `bpy` on the path:
    python3 export_models.py <output-dir>
Writes <name>.glb, <name>.obj and seams.json (each model's marked seam edges as endpoint positions in Blender's
Z-up world space, its pole positions, and every vertex position) into <output-dir>. Nothing here is committed as a fixture.
"""

import json
import sys
from pathlib import Path

import bpy
import bmesh  # only importable once bpy has loaded

Vec = tuple[float, float, float]
SEGMENTS, RINGS, RADIUS = 12, 8, 50.0


def fresh_sphere() -> bpy.types.Object:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.mesh.primitive_uv_sphere_add(segments=SEGMENTS, ring_count=RINGS, radius=RADIUS)
    obj = bpy.context.active_object
    if obj is None:
        raise RuntimeError("no sphere was created")
    return obj


def mark_and_unwrap(obj: bpy.types.Object, seam: list[tuple[Vec, Vec]]) -> None:
    """Marks the edges whose endpoints are the given positions as seams, then unwraps angle-based."""
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    wanted = {frozenset((tuple(round(c, 4) for c in a), tuple(round(c, 4) for c in b))) for a, b in seam}
    found = 0
    for e in bm.edges:
        key = frozenset(tuple(round(c, 4) for c in v.co) for v in e.verts)
        e.seam = key in wanted
        found += e.seam
    if found != len(wanted):
        raise RuntimeError(f"marked {found} of {len(wanted)} seam edges")
    bm.to_mesh(me)
    bm.free()
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.unwrap(method="ANGLE_BASED", margin=0.02)
    bpy.ops.object.mode_set(mode="OBJECT")


def rings_and_poles(obj: bpy.types.Object) -> tuple[list[list[Vec]], Vec, Vec]:
    """Vertex positions grouped into rings by height (top to bottom, each sorted by angle), and the two poles."""
    import math

    pos = [tuple(v.co) for v in obj.data.vertices]
    north, south = max(pos, key=lambda p: p[2]), min(pos, key=lambda p: p[2])
    by_z: dict[float, list[Vec]] = {}
    for p in pos:
        if p not in (north, south):
            by_z.setdefault(round(p[2], 4), []).append(p)

    def angle(p: Vec) -> float:
        a = math.atan2(p[1], p[0]) % (2 * math.pi)
        return 0.0 if a > 2 * math.pi - 1e-6 else a  # float noise must not move meridian 0 to the end

    rings = [sorted(by_z[z], key=angle) for z in sorted(by_z, reverse=True)]
    return rings, north, south


def vertices(obj: bpy.types.Object) -> list[Vec]:
    return [tuple(v.co) for v in obj.data.vertices]


def chain(points: list[Vec]) -> list[tuple[Vec, Vec]]:
    return list(zip(points, points[1:]))


def export(obj: bpy.types.Object, out: Path, name: str) -> None:
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.ops.export_scene.gltf(filepath=str(out / f"{name}.glb"), export_format="GLB", use_selection=True)
    bpy.ops.wm.obj_export(filepath=str(out / f"{name}.obj"), export_selected_objects=True, export_materials=True)


def build_all(out: Path) -> dict[str, object]:
    record: dict[str, object] = {}

    obj = fresh_sphere()
    export(obj, out, "sphere-default")  # Blender's built-in UV map, no seams marked
    rings, north, south = rings_and_poles(obj)
    record["sphere-default"] = {"seams": None, "poles": [north, south], "vertices": vertices(obj)}

    obj = fresh_sphere()
    rings, north, south = rings_and_poles(obj)
    meridian = chain([north] + [r[0] for r in rings] + [south])
    equator = chain(rings[3] + rings[3][:1])
    mark_and_unwrap(obj, meridian + equator)
    export(obj, out, "sphere-marked")
    record["sphere-marked"] = {"seams": meridian + equator, "poles": [north, south], "vertices": vertices(obj)}

    obj = fresh_sphere()
    rings, north, south = rings_and_poles(obj)
    slit = chain([rings[2][6], rings[3][6], rings[4][6]])  # two edges, both ends inside the surface
    mark_and_unwrap(obj, meridian + slit)
    export(obj, out, "sphere-dead-end")
    record["sphere-dead-end"] = {
        "seams": meridian + slit, "poles": [north, south], "tips": [rings[2][6], rings[4][6]], "vertices": vertices(obj)
    }

    obj = fresh_sphere()
    rings, north, south = rings_and_poles(obj)
    mark_and_unwrap(obj, meridian + equator)
    top, bottom = bpy.data.materials.new("Head"), bpy.data.materials.new("Body")
    obj.data.materials.append(top)
    obj.data.materials.append(bottom)
    equator_z = rings[3][0][2]
    for poly in obj.data.polygons:
        poly.material_index = 0 if min(obj.data.vertices[v].co[2] for v in poly.vertices) >= equator_z - 1e-6 else 1
    export(obj, out, "sphere-materials")
    record["sphere-materials"] = {
        "seams": meridian + equator, "poles": [north, south], "materials": ["Head", "Body"], "vertices": vertices(obj)
    }
    return record


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: python3 export_models.py <output-dir>")
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    record = build_all(out)
    record["blender"] = bpy.app.version_string
    (out / "seams.json").write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
