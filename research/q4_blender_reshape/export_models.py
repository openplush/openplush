# SPDX-License-Identifier: GPL-3.0-or-later
"""Builds a quadruped with marked seams, exports it, then re-exports it after each kind of edit to one leg.

GPL-3.0-or-later, because it imports Blender's `bpy` (ADR 0011). Run with the pinned `bpy` on the path:
    python3 export_models.py <output-dir>
Writes original.{glb,obj}, <edit>.{glb,obj} for each edit, and model.json: the original's marked seam edges and the
edited leg's vertices as positions in Blender's Z-up world space, the edit names, and Blender's version.
"""

import json
import sys
from collections.abc import Callable, Iterable
from pathlib import Path

import bpy
import bmesh  # only importable once bpy has loaded

Vec = tuple[float, float, float]
BODY = (60.0, 30.0, 24.0)  # mm, a box subdivided 3 times per side before the legs and head are extruded
LEG_SEGMENTS, LEG_STEP = 4, 10.0
HEAD_STEP = 16.0


def quadruped() -> tuple[bmesh.types.BMesh, list[bmesh.types.BMVert]]:
    """The model, with seams marked, and the vertices that belong to leg 0 below the body."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.scale(bm, vec=BODY, verts=bm.verts)
    bmesh.ops.subdivide_edges(bm, edges=bm.edges[:], cuts=3, use_grid_fill=True)
    bm.faces.ensure_lookup_table()
    bottom = sorted((f for f in bm.faces if f.normal.z < -0.99), key=lambda f: centre(f, 0, 1))
    front = sorted((f for f in bm.faces if f.normal.x > 0.99), key=lambda f: centre(f, 1, 2))
    legs = [bottom[0], bottom[3], bottom[12], bottom[15]]  # the four corner faces of the 4 x 4 underside
    leg0: list[bmesh.types.BMVert] = []
    for n, face in enumerate(legs):
        ring = list(face.edges)
        corner = face.verts[0].co.copy()
        top = face
        for _ in range(LEG_SEGMENTS):
            out = bmesh.ops.extrude_discrete_faces(bm, faces=[top])["faces"][0]
            bmesh.ops.translate(bm, vec=(0.0, 0.0, -LEG_STEP), verts=out.verts)
            if n == 0:
                leg0 += out.verts
            top = out
        # Marked after extruding: extrusion copies a face's edge flags onto every new ring.
        mark(ring)
        mark(list(top.edges))
        below = [e for e in bm.edges if all(v.co.x == corner.x and v.co.y == corner.y for v in e.verts)]
        mark(e for e in below if all(v.co.z <= corner.z for v in e.verts))
    neck = front[5]  # an inner face of the 4 x 4 front
    collar = list(neck.edges)
    head = bmesh.ops.extrude_discrete_faces(bm, faces=[neck])["faces"][0]
    bmesh.ops.translate(bm, vec=(HEAD_STEP, 0.0, 0.0), verts=head.verts)
    mark(collar)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-6)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm, sorted(set(leg0), key=lambda v: tuple(v.co))


def centre(face: bmesh.types.BMFace, a: int, b: int) -> tuple[float, float]:
    """Two coordinates of a face's centre, rounded so grid faces sort in a stable order."""
    c = face.calc_center_median()
    return (round(c[a], 3), round(c[b], 3))


def mark(edges: Iterable[bmesh.types.BMEdge]) -> None:
    for e in edges:
        e.seam = True


def stretch(bm: bmesh.types.BMesh, leg: list[bmesh.types.BMVert]) -> None:
    top = max(v.co.z for v in leg) + LEG_STEP
    for v in leg:
        v.co.z = top + 1.5 * (v.co.z - top)


def loop_cut(bm: bmesh.types.BMesh, leg: list[bmesh.types.BMVert]) -> None:
    on_leg = set(leg)
    vertical = [e for e in bm.edges if set(e.verts) <= on_leg and abs(e.verts[0].co.z - e.verts[1].co.z) > 1e-6]
    bmesh.ops.subdivide_edges(bm, edges=vertical, cuts=1, use_grid_fill=False)


def extrude_toe(bm: bmesh.types.BMesh, leg: list[bmesh.types.BMVert]) -> None:
    low = min(v.co.z for v in leg)
    sole = [f for f in bm.faces if all(abs(v.co.z - low) < 1e-6 and v in leg for v in f.verts)]
    out = bmesh.ops.extrude_discrete_faces(bm, faces=sole)["faces"]
    bmesh.ops.translate(bm, vec=(4.0, 0.0, -4.0), verts=[v for f in out for v in f.verts])


def smooth_all(bm: bmesh.types.BMesh, leg: list[bmesh.types.BMVert]) -> None:
    bmesh.ops.smooth_vert(bm, verts=bm.verts, factor=0.5, use_axis_x=True, use_axis_y=True, use_axis_z=True)


def nudge_sole(bm: bmesh.types.BMesh, leg: list[bmesh.types.BMVert]) -> None:
    """Leg 0's sole moved 5 mm down with proportional editing, as a maker dragging it would.

    Blender's Smooth falloff: weight 3t^2 - 2t^3 with t = 1 - d / r, for vertices within r = 20 mm of the sole.
    """
    low = min(v.co.z for v in leg)
    sole = [v for v in leg if abs(v.co.z - low) < 1e-6]
    for v in bm.verts:
        d = min((v.co - s.co).length for s in sole)
        if d < 20.0:
            t = 1.0 - d / 20.0
            v.co.z -= 5.0 * (3 * t * t - 2 * t * t * t)


def move_all(bm: bmesh.types.BMesh, leg: list[bmesh.types.BMVert]) -> None:
    bmesh.ops.translate(bm, vec=(5.0, 0.0, 0.0), verts=bm.verts)


EDITS: dict[str, Callable[[bmesh.types.BMesh, list[bmesh.types.BMVert]], None]] = {
    "unchanged": lambda bm, leg: None,
    "stretch-leg": stretch,
    "loop-cut-leg": loop_cut,
    "extrude-toe": extrude_toe,
    "nudge-sole-proportional": nudge_sole,
    "smooth-all": smooth_all,
    "move-all": move_all,
}


def export(bm: bmesh.types.BMesh, out: Path, name: str) -> None:
    """Unwraps along the marked seams, as a maker re-exporting would, and writes GLB and OBJ."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.unwrap(method="ANGLE_BASED", margin=0.02)
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.export_scene.gltf(filepath=str(out / f"{name}.glb"), export_format="GLB")
    bpy.ops.wm.obj_export(filepath=str(out / f"{name}.obj"), export_materials=False)


def position(v: bmesh.types.BMVert) -> list[float]:
    return [v.co.x, v.co.y, v.co.z]


def main(out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    bm, leg = quadruped()
    bm.verts.index_update()
    seams = [[position(e.verts[0]), position(e.verts[1])] for e in bm.edges if e.seam]
    edited = [position(v) for v in leg]
    export(bm, out, "original")
    for name, edit in EDITS.items():
        copy = bm.copy()
        copy.verts.ensure_lookup_table()
        edit(copy, [copy.verts[v.index] for v in leg])
        export(copy, out, name)
    record = {"blender": bpy.app.version_string, "seams": seams, "edited": edited, "edits": list(EDITS)}
    (out / "model.json").write_text(json.dumps(record, indent=1))
    print(f"Blender {bpy.app.version_string}: {len(bm.verts)} vertices, {len(seams)} seam edges, {len(leg)} on leg 0")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
