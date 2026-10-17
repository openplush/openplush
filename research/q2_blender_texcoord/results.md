# Spike Q2: results

Run on 2026-10-17, Linux x86_64. Blender `bpy` 5.2.2 LTS (PyPI), CPython 3.13.16 for the export; `read_exports.py` and its tests on CPython 3.11.17, 3.12.3 and 3.13.16 (16 tests, OK); `mypy --strict --python-version 3.11 read_exports.py test_read_exports.py` clean (`export_texcoord.py` is not type-checked, because `bpy` has no stubs). Two full export-and-read runs gave byte-identical reports.

## What each export carries

Map `Zeta` was created first and `Alpha` second, so a name-sorted order would put `Alpha` first.

| Case | Active | Render | glTF `TEXCOORD_0`, `_1` | Texture samples | OBJ |
|---|---|---|---|---|---|
| active-zeta-render-zeta | Zeta | Zeta | Zeta, Alpha | — | Zeta |
| active-zeta-render-alpha | Zeta | Alpha | Zeta, Alpha | — | Zeta |
| active-alpha-render-zeta | Alpha | Zeta | Zeta, Alpha | — | Alpha |
| active-alpha-render-alpha | Alpha | Alpha | Zeta, Alpha | — | Alpha |
| texture-alpha-active-zeta | Zeta | Zeta | Zeta, Alpha | `TEXCOORD_1` | Zeta |
| texture-alpha-active-alpha | Alpha | Alpha | Zeta, Alpha | `TEXCOORD_1` | Alpha |

## Answers

| | Answer | Tag |
|---|---|---|
| Which map becomes glTF `TEXCOORD_0`? | **The first in Blender's list**, in every case, not the first by name. The active and render flags make no difference, and neither does a texture that samples another map: the texture points at `TEXCOORD_1` instead of reordering the sets | [solid] for Blender 5.2.2 |
| Which map does OBJ carry? | **The active one**, and only that one. The render flag makes no difference | [solid] for Blender 5.2.2 |
| Do the formats agree? | **Only when the active map is first.** When a maker unwraps for sewing on a second map and leaves it active, an OBJ import reads the seams, while a glTF import reads the first map's cuts | [solid] for Blender 5.2.2 |

## What this means for 003

- FR-004's report wording can be exact per format. glTF: "Read the first UV map in the modelling tool's list (`TEXCOORD_0`); N others were ignored." OBJ: "Read the model's one UV map (in Blender, the active map at export)."
- The disagreement is a real trap, but only for glTF models with several UV maps, and the report already says other maps were ignored. Whether the report should also suggest "make your seam map the first" is a spec decision for U021; this spike only records the facts.
