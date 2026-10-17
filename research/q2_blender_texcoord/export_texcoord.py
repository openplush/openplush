# SPDX-License-Identifier: GPL-3.0-or-later
"""Builds planes with two UV maps in known states and exports each as GLB and OBJ.

GPL-3.0-or-later, because it imports Blender's `bpy` (ADR 0011). Run with the pinned `bpy` on the path:
    python3 export_texcoord.py <output-dir>
Map "Zeta" is created first and "Alpha" second, so list order and name order disagree. Every corner of Zeta gets
u = 0.125 and of Alpha u = 0.875, so each exported UV set names its map. Writes <case>.glb, <case>.obj and
cases.json (each case's map list, active, render and texture map, all read back from Blender).
"""

import json
import sys
from pathlib import Path

import bpy

MAP_U = {"Zeta": 0.125, "Alpha": 0.875}
# (case, active map, render map, map the base colour texture samples or None)
CASES = [
    ("active-zeta-render-zeta", "Zeta", "Zeta", None),
    ("active-zeta-render-alpha", "Zeta", "Alpha", None),
    ("active-alpha-render-zeta", "Alpha", "Zeta", None),
    ("active-alpha-render-alpha", "Alpha", "Alpha", None),
    ("texture-alpha-active-zeta", "Zeta", "Zeta", "Alpha"),
    ("texture-alpha-active-alpha", "Alpha", "Alpha", "Alpha"),
]


def plane(active: str, render: str, texture: str | None) -> bpy.types.Mesh:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.mesh.primitive_plane_add()
    obj = bpy.context.active_object
    if obj is None:
        raise RuntimeError("Blender made no plane")
    mesh = obj.data
    mesh.uv_layers[0].name = "Zeta"
    mesh.uv_layers.new(name="Alpha")
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


def texture_map(mesh: bpy.types.Mesh) -> str | None:
    """The UV map the material's UV Map node names; raises if it names no map, which Blender accepts silently."""
    if not mesh.materials:
        return None
    names = [node.uv_map for node in mesh.materials[0].node_tree.nodes if node.bl_idname == "ShaderNodeUVMap"]
    if len(names) != 1 or names[0] not in mesh.uv_layers:
        raise RuntimeError(f"UV Map node names {names}, not one of the mesh's maps")
    return names[0]


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
            "texture": texture_map(mesh),
        })
    (out / "cases.json").write_text(json.dumps(cases, indent=1))
    print(f"Blender {bpy.app.version_string}")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
