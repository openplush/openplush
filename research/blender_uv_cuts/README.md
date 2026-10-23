# Spike 003 T008: do Blender's UV cuts read as seams at `UV_CUT = 1e-5`?

> **License: GPL-3.0-or-later** (see [`LICENSE`](LICENSE)). `export_models.py` imports Blender's `bpy`, which is GPL-3.0, so this whole directory is GPL. Nothing outside it may import it ([ADR 0011](../../docs/adr/0011-blender-research-tool.md)). Copying code from here into the MIT/Apache parts of the project is not allowed.

Non-shipping research code for feature 003, task T008 (research R2, R6 T008).

**Status (2026-10-17): `UV_CUT = 1e-5` holds for marked seams, but the spike found two spec problems.** First, a meridian seam's ends at the poles are dead ends under FR-011, which rejected the usual sphere unwrap and three of the committed fixtures; decided 2026-10-17 (003 D-014): such a cut becomes a dart proposal. Second, Blender's default projection UVs put false cuts on pole fans. Numbers are in [`results.md`](results.md).

**Scope delivered**: UV spheres only. 003 T008 also asked for a capsule and a quadruped, and 003 T011 assumes this spike's quadruped; neither was built. Recording the results in 003 research.md (R1, R2, R6) is a separate spec change.

## The question

003's import reads a UV cut wherever, at either end of an interior edge, the corner UVs on the two sides differ by more than `UV_CUT` (research R2, provisional `1e-5`). On real Blender exports:

1. Does every marked seam read as a cut, and nothing else?
2. Does a seam that stops inside the surface (a dead end) read as a slit, with its tip carrying one UV?
3. Do pole fans read as false cuts?
4. Do the exports follow the conventions `research/fixtures_gen` imitates: vertex splits wherever an attribute differs, V flipped in glTF, one glTF primitive per material, `usemtl` switches inside one OBJ object?

## Method

- `export_models.py` (needs `bpy`) builds four 12-segment, 8-ring, 50 mm UV spheres:
  - `sphere-default`: Blender's built-in UV map, no seams marked;
  - `sphere-marked`: a pole-to-pole meridian plus the equator marked;
  - `sphere-dead-end`: the meridian plus a two-edge slit with both ends inside the surface;
  - `sphere-materials`: the marked sphere in `Head` and `Body` materials, split at the equator.

  Each marked sphere is unwrapped angle-based. Each is exported as GLB and OBJ with the exporters' defaults. The script also records the marked edges, poles and every vertex in Blender's own coordinates (`seams.json`).
- `measure.py` (no `bpy`) names every exported corner by the Blender vertex within 1e-3 mm, raising if there is not exactly one. It finds each interior edge's largest corner-UV gap and compares the edges over `UV_CUT` with the marked ones. It prints one JSON report.
- `test_measure.py` checks the measuring rules on hand-built polygons.

```sh
pip install --target <scratch> -r requirements.txt
PYTHONPATH=<scratch> python3 export_models.py <out>   # Blender exports + seams.json
python3 measure.py <out> > report.json                # no bpy needed
python3 -m unittest -v                                 # the measuring stick's own tests
```

## Limits

- One shape family: UV spheres, which have the pole fans R2 worried about. The capsule and quadruped named in R6 were not built; the questions turned on the sphere's poles and the slit, not on overall shape.
- One unwrap method (angle-based) and one Blender version (5.2.2). Smart UV Project and cylinder or sphere projections other than the primitive's default were not tried.
- Blender's exports are not byte-reproducible from run to run: normal order changes, and UVs differ by up to about 2.4e-6, which moves the slit's gap between 1.81e-4 and 1.84e-4. The counts are stable across eight runs.
