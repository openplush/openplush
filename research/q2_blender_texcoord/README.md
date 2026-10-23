# Spike 003 T009: which Blender UV map becomes `TEXCOORD_0`?

> **License: GPL-3.0-or-later** (see [`LICENSE`](LICENSE)). `export_texcoord.py` imports Blender's `bpy`, which is GPL-3.0, so this whole directory is GPL. Nothing outside it may import it ([ADR 0011](../../docs/adr/0011-blender-research-tool.md)). Copying code from here into the MIT/Apache parts of the project is not allowed.

Non-shipping research code for feature 003, task T009 (research R1, R6 T009).

**Status (2026-10-17): answered.** glTF's `TEXCOORD_0` is the first UV map in Blender's list, whichever map is active or set to render. OBJ carries only the active map. So the two formats read different maps when the active one isn't first. Details are in [`results.md`](results.md). Recording the answer in 003 research.md R1 is a separate spec change.

## The question

003 reads only the first set of texture coordinates (spec FR-004), and its report says which set it read. In a Blender mesh with several UV maps, one is *active* (the one the UV editor shows) and one is set to *render*. Which one does each exporter write first?

## Method

- `export_texcoord.py` (needs `bpy`) builds a plane with two UV maps, `Zeta` created first and `Alpha` second, so list order and name order disagree. Every corner of `Zeta` has u = 0.125 and every corner of `Alpha` u = 0.875, so the u value names the map. It covers six cases:
  - active and render each set to either map (four cases);
  - a base colour texture that samples `Alpha` through a UV Map node, with `Zeta` or `Alpha` active.

  Each case is exported as GLB and OBJ with the exporters' defaults. `cases.json` records each case's map list, active, render and texture map, read back from Blender; the script raises if the UV Map node names no map of the mesh, which Blender itself accepts silently.
- `read_exports.py` (no `bpy`) names the map behind each `TEXCOORD_n` in order, the set the texture samples (`texCoord`), and the map in the OBJ. It raises on any u it can't name exactly.
- `test_read_exports.py` checks the reader on hand-built GLB bytes and OBJ text.

```sh
pip install --target <scratch> -r requirements.txt
PYTHONPATH=<scratch> python3 export_texcoord.py <out>   # Blender exports + cases.json
python3 read_exports.py <out> > report.json            # no bpy needed
python3 -m unittest -v                                  # the reader's own tests
```

## Limits

- One Blender version (5.2.2) and the exporters' default settings.
- Two UV maps; with three or more, the order rule is [likely] the same.
- Blender has no command to reorder UV maps, so "first in the list" means the first map created (or the first left after deleting earlier ones) [likely].
