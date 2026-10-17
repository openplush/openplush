# Spike Q4: results

Run on 2026-10-17, Linux x86_64. Blender `bpy` 5.2.2 LTS (PyPI), CPython 3.13.16 for the export; `measure.py` and its tests on CPython 3.11.17, 3.12.3 and 3.13.16 (13 tests, OK); `mypy --strict --python-version 3.11 measure.py test_measure.py` clean (`export_models.py` is not type-checked, because `bpy` has no stubs). Two full export-and-measure runs gave byte-identical reports.

The quadruped has 166 vertices and 52 marked seam edges. 8 of the edges touch leg 0 (its 4 sole edges and its 4-edge corner line) and 44 don't. GLB faces are triangles and OBJ faces are quads, so their face counts differ.

## Placed by position, original against each re-export

| Edit | Seams away from leg 0 | Seams on leg 0 | Faces away (GLB / OBJ) | Faces on leg 0 (GLB / OBJ) |
|---|---|---|---|---|
| unchanged | 44 / 44 | 8 / 8 | 294 / 294, 147 / 147 | 34 / 34, 17 / 17 |
| stretch-leg | **44 / 44** | 0 / 8 | 294 / 294, 147 / 147 | 0 / 34, 0 / 17 |
| loop-cut-leg | **44 / 44** | 5 / 8 | 294 / 294, 147 / 147 | 10 / 34, 5 / 17 |
| extrude-toe | **44 / 44** | 8 / 8 | 294 / 294, 147 / 147 | 32 / 34, 16 / 17 |
| smooth-all | 0 / 44 | 0 / 8 | 35 / 294, 16 / 147 | 0 / 34, 0 / 17 |
| move-all | 0 / 44 | 0 / 8 | 0 / 294, 0 / 147 | 0 / 34, 0 / 17 |

GLB and OBJ gave the same seam counts in every case.

## Answers

| | Answer | Tag |
|---|---|---|
| Do seams away from an edited limb re-place by position? | **Yes, all of them**, after a stretch, a loop cut that renumbers the mesh, and an extrusion. Blender wrote every untouched vertex bit-identically: all 166 original positions reappear exactly after the loop cut and the extrusion, and 154 after the stretch (the 16 leg vertices moved, and 4 of them landed on old positions). So the weld tolerance has nothing to absorb | [solid] for this model and these edits |
| Do faces away from it? | **Yes, all of them**, in both formats. Blender triangulated the untouched quads the same way each time | [solid] for this model |
| What happens on the edited limb? | Edges that moved don't re-place (stretch: 0 / 8). Edges a loop cut split don't either (3 of 8 lost), because each old edge is now two edges and R5 never snaps. An extrusion leaves the old sole's edges in place (8 / 8) | [solid] |
| What breaks it? | **Any edit that moves every vertex**: one smoothing pass (0 / 44) or moving the whole model 5 mm (0 / 44). Under Merge, every old seam would then be listed as not placeable | [solid] |

## What this means for 003

- R5's positional re-placement is sound for the common case, a local edit to one part: research R5's `[likely]` can become `[solid]` for local edits.
- Two costs belong in the spec's hands, not this spike's:
  - **A loop cut on an edited limb loses the seams it splits.** Those seams are listed as not placeable (FR-051), which is honest, but the maker has to redraw them. Re-placing a split edge as its two halves is a possible refinement. It would not be snapping, because both halves lie exactly on the old edge.
  - **Moving or smoothing the whole model loses everything.** Merge would then list every old seam as not placeable. Detecting a whole-model move is possible, but it guesses, so it is a decision for the maintainer.
