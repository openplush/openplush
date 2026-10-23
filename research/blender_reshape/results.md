# Spike 003 T011: results

Run on 2026-10-17, Linux x86_64. Blender `bpy` 5.2.2 LTS (PyPI), CPython 3.13.16 for the export; `measure.py` and its tests on CPython 3.11.17, 3.12.3 and 3.13.16 (23 tests, OK); `mypy --strict --python-version 3.11 measure.py test_measure.py` clean (`export_models.py` is not type-checked, because `bpy` has no stubs). Two full export-and-measure runs gave byte-identical reports.

`export_models.py` reports a quadruped of 166 vertices and 52 marked seam edges. 8 of the edges touch leg 0 (its 4 sole edges and its 4-edge corner line) and 44 don't. GLB faces are triangles and OBJ faces are quads, so their face counts differ. Every count below comes from `measure.py`'s report.

## Placed by position, original against each re-export

| Edit | Positions kept bit-for-bit | Seams away from leg 0 | Seams on leg 0 | Faces away (GLB, OBJ) | Faces on leg 0 (GLB, OBJ) |
|---|---|---|---|---|---|
| unchanged | 166 / 166 | 44 / 44 | 8 / 8 | 294 / 294, 147 / 147 | 34 / 34, 17 / 17 |
| stretch-leg | 154 / 166 | **44 / 44** | 0 / 8 | 294 / 294, 147 / 147 | 0 / 34, 0 / 17 |
| loop-cut-leg | 166 / 166 | **44 / 44** | 5 / 8 | 294 / 294, 147 / 147 | 10 / 34, 5 / 17 |
| extrude-toe | 166 / 166 | **44 / 44** | 8 / 8 | 294 / 294, 147 / 147 | 32 / 34, 16 / 17 |
| nudge-sole-proportional | 156 / 166 | **41 / 44** | 2 / 8 | 287 / 294, 143 / 147 | 16 / 34, 8 / 17 |
| smooth-all | 46 / 166 | 0 / 44 | 0 / 8 | 35 / 294, 16 / 147 | 0 / 34, 0 / 17 |
| move-all | 0 / 166 | 0 / 44 | 0 / 8 | 0 / 294, 0 / 147 | 0 / 34, 0 / 17 |

GLB and OBJ gave the same seam and position counts in every case. Stretch moves all 16 of leg 0's vertices; 4 of them land on old positions of others, hence 154.

## Answers

| | Answer | Tag |
|---|---|---|
| Does Blender's export disturb vertices an edit didn't move? | **No.** After the loop cut and the extrusion every original position reappears bit-for-bit, so the weld tolerance has nothing to absorb | [solid] for Blender 5.2.2 |
| Do seams away from an edit re-place? | **Yes, if the edit moved none of their vertices**: 44 / 44 after the stretch, the loop cut (which renumbers the mesh) and the extrusion. A proportional edit of leg 0's sole lost 3: the neighbouring leg's sole edges, 15 mm away and inside the 20 mm falloff | [solid] for these edits; [likely] for hand edits, whose reach depends on the falloff the maker uses |
| Do faces away from it? | **Yes, under the same condition**, in both formats. Blender triangulated untouched quads the same way each time (a face whose corners survive but which is split does not count as placed) | [solid] for this model |
| What happens on the edited limb? | Moved edges don't re-place (stretch: 0 / 8). Edges a loop cut splits don't either (3 of 8 lost), because each old edge is now two and R5 never snaps. An extrusion leaves the old sole's edges in place (8 / 8) | [solid] |
| What breaks it? | **Edits that move most vertices.** One smoothing pass moved 120 of 166 vertices and re-placed no seams and few faces; moving the whole model 5 mm moved every vertex and re-placed nothing. Under Merge, every old seam would then be listed as not placeable | [solid] |

## What this means for 003

- R5's positional re-placement holds for any seam whose vertices an edit leaves alone: research R5's `[likely]` can become `[solid]` for that claim. How many seams a maker's edit leaves alone depends on the edit, so the overall share stays `[likely]`.
- Three costs belong in the spec's hands, not this spike's:
  - **Proportional editing reaches beyond the limb.** Seams within the falloff are lost and listed as not placeable (FR-051). That is honest, but the maker may not expect it.
  - **A loop cut on an edited limb loses the seams it splits.** Re-placing a split edge as its halves would need its new vertex to lie on the old edge within the weld tolerance. That held here because the leg's edges are axis-aligned; for general edges it is [speculative].
  - **Moving or smoothing the whole model loses everything.** Detecting a whole-model move is possible, but it guesses, so it is a decision for the maintainer.
