# Spike Q1: results

Run on 2026-10-17, Linux x86_64. Blender `bpy` 5.2.2 LTS (PyPI), CPython 3.13.16 for the export; `measure.py` and its tests on CPython 3.11.17, 3.12.3 and 3.13.16 (6 tests, OK); `mypy --strict --python-version 3.11 measure.py test_measure.py` clean (`export_models.py` is not type-checked, because `bpy` has no stubs). Two full export-and-measure runs gave the same counts; the gaps agree to the 3 significant figures shown.

## The four questions

| | Answer | Tag |
|---|---|---|
| 1. Marked seams, and only those, read as cuts at `1e-5` | **Yes**, in GLB and OBJ alike: 20 of 20 on `sphere-marked`, 10 of 10 on `sphere-dead-end`, 0 extra cuts. Across every unmarked edge the corner UVs are **bit-identical** (largest gap 0.0) | [solid] for these models |
| 2. A dead-ended seam reads as a slit | **Yes**: both slit edges read as cuts, and each tip carries 1 UV. But the slit barely opens: its smallest gap is **1.81e-4**, against 6.65e-2 for a seam that separates the surface | [solid] for this slit |
| 3. Pole fans read as false cuts | **Not after a marked unwrap**: only the meridian's 2 pole edges are cuts. **Yes in Blender's default UV sphere map**: 24 pole edges read as cuts, because every pole triangle has its own pole UV | [solid] |
| 4. The exports follow the generator's conventions | **Yes, with one addition**: the GLB splits a vertex wherever any attribute differs (360 vertices for 106 distinct position/UV pairs, because the primitive is flat-shaded); V is flipped (every GLB UV, flipped back, equals an OBJ UV on the same vertex); one GLB primitive per material, named `Head` and `Body`; OBJ switches `usemtl` inside one `o`. The addition: **Blender's OBJ keeps quads** (3- and 4-corner faces), while every committed fixture is triangles | [solid] |

## Numbers

Edges are interior edges of each export (the GLB has more because it is triangulated). Gaps are the largest corner-UV difference across an edge at either end.

| Model | Format | Edges | Cuts | Marked | Marked, not cut | Cut, not marked | Pole edges cut | Largest gap off seams | Smallest gap on seams |
|---|---|---|---|---|---|---|---|---|---|
| sphere-default | GLB | 252 | 30 | — | — | — | 24 | — | (smallest non-zero: 8.33e-2) |
| sphere-default | OBJ | 180 | 30 | — | — | — | 24 | — | (8.33e-2) |
| sphere-marked | GLB | 252 | 20 | 20 | 0 | 0 | 2 | 0.0 | 6.65e-2 |
| sphere-marked | OBJ | 180 | 20 | 20 | 0 | 0 | 2 | 0.0 | 6.65e-2 |
| sphere-dead-end | GLB | 252 | 10 | 10 | 0 | 0 | 2 | 0.0 | 1.81e-4 |
| sphere-dead-end | OBJ | 180 | 10 | 10 | 0 | 0 | 2 | 0.0 | 1.82e-4 |
| sphere-materials | GLB | 252 | 20 | 20 | 0 | 0 | 2 | 0.0 | 6.65e-2 |
| sphere-materials | OBJ | 180 | 20 | 20 | 0 | 0 | 2 | 0.0 | 6.65e-2 |

`sphere-default`'s 30 cuts are its 24 pole-fan edges plus the 6 interior edges of the u = 0/1 wrap meridian (the wrap's 2 pole edges are among the 24).

## What it means for `UV_CUT`

- Off seams the gap is exactly 0, so any positive threshold separates uncut edges. The binding constraint is the smallest real cut, a slit near its tip, at 1.81e-4.
- Below the threshold sit f32 spacing near 1.0 (1.19e-7) and OBJ's 6-decimal rounding (5e-7). So `1e-5` has about 18× margin to the slit and 20× to OBJ rounding [solid for these exports].
- Recommendation: **keep `UV_CUT = 1e-5`** [likely; one shape family, one unwrap method]. A slit that opens less than 1e-5 would read as no cut at all: an undetected dead end, not a false one. That is the safer failure, but it is a gap in FR-011. A slit's opening grows with its length and with how hard the unwrap relaxes it, so short slits are the risk.

## The bigger problem: an ordinary meridian seam is a "dead end" under FR-011

`dead_end_tips` in the report counts the vertices on exactly one cut edge, on these closed surfaces:

| Model | Dead-end tips | Where |
|---|---|---|
| sphere-marked, sphere-materials | 2 | the two poles |
| sphere-dead-end | 4 | the slit's 2 tips, plus the 2 poles |
| sphere-default | 22 | the ring-0 vertices of the pole fans (next section) |

A meridian marked from pole to pole ends at the poles, where nothing else meets it. FR-011 defines a dead end as an endpoint inside the surface that touches no other seam, so both poles are dead ends, and import would fail. Yet marking a meridian is the standard way to unwrap a sphere, and with the equator it gives each hemisphere a slit from its rim to its pole. In sewing terms that slit, sewn shut, is a **dart**: a seam ending inside a piece (001 FR-011). 003 FR-010 says an imported cut is never a dart, and FR-011 makes its tip an error. [solid: measured]

Consequences:
- **Fixtures**: `capsule-uv` (U001) and `reshaped-v1/v2` (U007) use exactly this layout, so the engine would reject them with `DeadEndCut`, while US1's independent test expects them to import. `dead-end` (U005) is unaffected; its cut A ends on a material boundary and an opening.
- **Makers**: a sphere or a capsule unwrapped the usual way in Blender would fail import.

Options for the spec, not decided here:
1. **Keep FR-011, change the fixtures.** Give capsule-uv a layout with no free ends, such as both rims plus a meridian on the tube only, like `seams/capsule.json`, and say in the docs that seams must end on another seam or an opening.
2. **Import a free end as a dart.** Reverse FR-010 for cuts that end in the interior. This is close to what the maker drew, but it is the case the maintainer ruled out.
3. **Keep the error, but make it teach.** Report the poles as a distinct, located case, with a remedy naming the fix in Blender: run the seam into another seam.

This needs the maintainer before U016–U019 (UV-cut detection) and U051 (dead ends) are built, and before #109 and #119 merge as US1's oracle.

## The open problem: pole fans in projection UVs

Blender's default UV sphere map gives each pole triangle its own pole UV, so the u-coordinate varies around the pole while v is 0 or 1. Under R2's per-endpoint rule every pole edge then reads as a cut. Each ring-0 vertex (one UV on both sides) would become a dead-end tip: 22 on this sphere, measured (`dead_end_tips` in the report), against the 2 that `sphere-dead-end`'s slit really has. Import would fail with `DeadEndCut` on a model whose maker never marked a seam.

- A pole-fan edge and a slit's last edge look the same locally: UVs agree at one end and differ at the other. The edge alone can't tell them apart.
- What differs is the far vertex. At a projection pole *every* incident face has its own UV, and those UVs share v and vary only in u.
- Options for the plan, not decided here:
  1. Accept it. Models must be unwrapped with marked seams, and the `DeadEndCut` remedy text says so.
  2. Recognise a projection pole (all corners distinct, with a shared v of 0 or 1) and treat its fan as uncut.
  3. Report it as a distinct, located error rather than as many dead ends.
- This needs a decision ledger entry before U019 builds detection.

## Not measured

- Hard-shaded edges in a smooth model, beyond the flat-shaded primitive's normal splits. `capsule-hardshade` covers that in the fixtures, and nothing here contradicts it.
- Smart UV Project, Follow Active Quads, or multiple UV maps (that is Q2).
