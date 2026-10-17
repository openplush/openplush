# Spike Q4: do old seams re-place by position after a model is edited?

> **License: GPL-3.0-or-later** (see [`LICENSE`](LICENSE)). `export_models.py` imports Blender's `bpy`, which is GPL-3.0, so this whole directory is GPL. Nothing outside it may import it ([ADR 0011](../../docs/adr/0011-blender-research-tool.md)). Copying code from here into the MIT/Apache parts of the project is not allowed.

Non-shipping research code for feature 003, task U011 (research R5, R6 Q4). U011 named Q1's quadruped, but Q1 built none, so this spike builds its own.

**Status (2026-10-17): answered for this model.** After an edit that moves only one leg's vertices, every seam and face away from that leg re-placed by position. A proportional edit also moves vertices around the leg, and lost the 3 seams within its reach. Smoothing the whole model or moving it re-placed no seams. Numbers are in [`results.md`](results.md). Updating 003 research.md R5's tag is a separate spec change.

## The question

003 re-places a project's existing seams and painted faces on a re-imported model **by position** (research R5): an old seam edge maps to the new edge whose endpoints coincide within 001's weld tolerance (1e-6 of the bbox diagonal), never by index and never to a nearby edge. When a maker edits one limb in Blender and exports again, what share of the seams away from that limb still re-place?

## Method

- `export_models.py` (needs `bpy`) builds a quadruped:
  - a 60 × 30 × 24 mm box body, subdivided into a 4 × 4 grid on each side;
  - four legs, each extruded from a corner face of the underside in four 10 mm steps;
  - a head extruded 16 mm from the front.

  It marks 52 seam edges: each leg's ring where it meets the body, its sole, a line down one corner, and the neck. It unwraps along them and exports `original.glb` and `.obj`. Then it re-exports a copy after each edit, re-unwrapping as a maker would:
  - `unchanged`: no edit (control);
  - `stretch-leg`: leg 0 made 1.5 × longer;
  - `loop-cut-leg`: the 12 vertical edges below leg 0's first ring split once (new vertices, renumbered mesh);
  - `extrude-toe`: leg 0's sole extruded into a toe;
  - `nudge-sole-proportional`: leg 0's sole moved 5 mm down with proportional editing (Blender's Smooth falloff, 20 mm radius), as a maker dragging it would;
  - `smooth-all`: one smoothing pass over every vertex;
  - `move-all`: the whole model moved 5 mm.

  `model.json` records the original's seam edges and leg 0's 16 vertices as positions in Blender's coordinates.
- `measure.py` (no `bpy`) welds each export by position within the weld tolerance. It re-places every original seam edge and face on each re-export of the same format, and counts the placed ones, split by whether they touch a vertex of leg 0. It also counts how many original positions reappear bit-for-bit.
- `test_measure.py` checks welding, placement and the readers on hand-built meshes.

```sh
pip install --target <scratch> -r requirements.txt
PYTHONPATH=<scratch> python3 export_models.py <out>   # Blender exports + model.json
python3 measure.py <out> > report.json                # no bpy needed
python3 -m unittest -v                                 # the measuring code's own tests
```

## Limits

- One synthetic model with a box-modelled topology, and one Blender version (5.2.2). A sculpted or retopologised model could behave differently [likely the same for edits that leave other vertices alone].
- Edits are scripted, not made by hand in the UI. `nudge-sole-proportional` reproduces Blender's Smooth falloff in `bmesh` rather than calling the transform operator, so its exact reach can differ from a UI drag [likely small].
- "Away from leg 0" means touching none of its 16 vertices. Leg 0's own ring where it meets the body counts as away.
- The edits change mesh data. An object transform (moving the object rather than its vertices) is baked into the export too, so it behaves like `move-all` [likely].
