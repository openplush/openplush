# SPDX-License-Identifier: GPL-3.0-or-later
"""Builds planes with two UV maps in known states and exports each as GLB and OBJ.

GPL-3.0-or-later, because it imports Blender's `bpy` (ADR 0011). Run with the pinned `bpy` on the path:
    python3 export_texcoord.py <output-dir>
Every corner of map "First" gets u = 0.125 and of "Second" u = 0.875, so each exported UV set names its map.
Writes <case>.glb, <case>.obj and cases.json (each case's active, render and texture map, read back from Blender).
"""

import json
import sys
from pathlib import Path

import bpy

MAP_U = {"First": 0.125, "Second": 0.875}
# (case, active map, render map, map the base colour texture samples or None)
CASES = [
    ("active-first-render-first", "First", "First", None),
    ("active-first-render-second", "First", "Second", None),
    ("active-second-render-first", "Second", "First", None),
    ("active-second-render-second", "Second", "Second", None),
    ("texture-second-active-first", "First", "First", "Second"),
    ("texture-second-active-second", "Second", "Second", "Second"),
]


def plane(active: str, render: str, texture: str | None) -> bpy.types.Mesh:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.mesh.primitive_plane_add()
    obj = bpy.context.active_object
    if obj is None:
        raise RuntimeError("Blender made no plane")
    mesh = obj.data
    mesh.uv_layers[0].name = "First"
    mesh.uv_layers.new(name="Second")
    for layer in mesh.uv_layers:
        for corner in layer.data:
            corner.uv = (MAP_U[layer.name], 0.5)
    mesh.uv_layers.active = mesh.uv_layers[active]
    mesh.uv_layers[render].active_render = True
    if texture is not None:
        obj.data.materials.append(textured_material(texture))
    return mesh


def textured_material(uv_map: str) -> bpy.types.Material:
    mat = bpy.data.materials.new("Textured")
    mat.use_nodes = True
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = bpy.data.images.new("Texture", 4, 4)
    uv = nodes.new("ShaderNodeUVMap")
    uv.uv_map = uv_map
    links.new(uv.outputs[0], tex.inputs[0])
    links.new(tex.outputs[0], nodes["Principled BSDF"].inputs[0])
    return mat


def main(out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    cases = []
    for name, active, render, texture in CASES:
        mesh = plane(active, render, texture)
        bpy.ops.export_scene.gltf(filepath=str(out / f"{name}.glb"), export_format="GLB")
        bpy.ops.wm.obj_export(filepath=str(out / f"{name}.obj"), export_materials=False)
        cases.append({
            "name": name,
            "maps": [layer.name for layer in mesh.uv_layers],
            "active": mesh.uv_layers.active.name,
            "render": [layer.name for layer in mesh.uv_layers if layer.active_render],
            "texture": texture,
        })
    (out / "cases.json").write_text(json.dumps(cases, indent=1))
    print(f"Blender {bpy.app.version_string}")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
