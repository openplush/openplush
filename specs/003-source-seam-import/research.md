# Research: Import Seams and Fabric Regions from the Source Model

**Feature**: `003-source-seam-import` | **Date**: 2026-10-16, updated 2026-10-17 | **Status**: Active

Confidence tags: `[solid]` consensus or verified, `[likely]` reasoned inference, `[speculative]` guess, `[verify]` must be checked by a spike before the task that depends on it.

## R1. What the export files carry

**Question**: after a Blender export, where do the maker's seams and materials survive?

| Fact | Tag |
|---|---|
| Neither OBJ nor glTF has a "seam" flag. Blender's Mark Seam survives only as its effect on the UV map: the two sides of a marked edge get different texture coordinates after unwrapping | [solid] |
| Blender's glTF exporter splits a vertex wherever its corners differ in UV or normal, so a UV seam arrives as two coincident vertices. 001's weld (T060) merges them by position and keeps per-corner UVs, so the cut is recoverable after repair | [solid] |
| Blender's glTF exporter writes one primitive per material slot used by a mesh. A primitive has at most one material; one with none uses glTF's default material | [solid] |
| OBJ stores `vt` per face corner (`f v/vt`), so UV discontinuities survive without vertex splitting. `usemtl` switches the material for the faces that follow; faces before any `usemtl` have none | [solid] |
| glTF can carry several UV sets (`TEXCOORD_0`, `TEXCOORD_1`, …). Whether Blender writes the *active* UV map as `TEXCOORD_0` or the first in the list is unconfirmed | [verify] spike Q2 |
| 001's OBJ reader (its own, not `tobj`: 001 D-034) starts a new source primitive at each `o`, `g` and `usemtl` line, so a primitive has at most one material | [solid] 001 D-034, T050 |
| A dead-ended Blender seam (one that doesn't separate the UV island) still gives its edges different UVs on each side: it shows up in the UV editor as a slit. So a dead end is detectable from the export alone. Measured by spike Q1: both slit edges read as cuts, but the slit barely opens (smallest gap 1.81e-4 to 1.84e-4 UV units) | [solid] on UV spheres, spike Q1 |
| Blender's OBJ exporter keeps quads (3- and 4-corner faces). 001 triangulates polygons at import (001 D-021); a diagonal inside one face joins two corners of that face, so it carries the same UVs on both sides and is never a cut | [solid] spike Q1 for the quads; [likely] for the diagonals |

**Decision**: read only what the file carries. UV cuts come from per-corner texture coordinates, and materials from the primitive a face came from. No `.blend` reading (out of scope).

## R2. Detecting a UV cut

**Decision**: an interior edge is a UV cut when, at either endpoint, the corner UV on one side differs from the corner UV on the other by more than `UV_CUT` (absolute, in UV units). Only edges with UVs on both sides are tested. An edge with a UV-less side is never a cut, and those faces are counted in the report.

**Rationale**:
- Testing per endpoint, not per edge midpoint, catches a cut that runs into a junction: one endpoint's UVs agree while the other's differ [solid].
- The tolerance is absolute because UV space is dimensionless. The model-space tolerances (relative to the bbox diagonal) and the physical ones (mm) both mean nothing here. glTF stores UVs as `f32`, and an exporter writes one value per deduplicated vertex, so an uncut edge normally carries bit-identical UVs [likely]. `1e-5` sits about two orders above `f32` spacing near 1.0 and far below any real cut. UDIM tiles (u up to ~10) still clear it [likely].
- Spike Q1 confirmed the value on Blender 5.2 exports of UV spheres, in glTF and OBJ: every marked seam read as a cut and nothing else did. Across every unmarked edge the corner UVs were bit-identical (largest gap 0.0). The tightest real cut was a dead-ended slit near its tip, at 1.81e-4 to 1.84e-4, 18 times `UV_CUT`; a seam that separates the surface measured 6.65e-2 [solid] for these models. `UV_CUT = 1e-5` stands.
- Requiring UVs on both sides keeps repair-filled faces (T066, which have no source UVs) from creating a ring of false cuts around every filled hole. That is the mechanism behind spec FR-024's "never create a seam of their own".

**Alternatives rejected**:
- *Islands by UV connectivity and seams as island borders*: misses dead-ended slits, which are exactly what FR-011 must catch.
- *Tolerance relative to UV bounding box*: a model with one tiny UV island and one huge one would get a threshold that is right for neither.
- *Comparing normals too*: FR-001 forbids it. Hard-shaded edges are not cuts.

### Projection maps (pole fans)

Spike Q1 found that Blender's default UV sphere map, a projection made without marked seams, gives every pole triangle its own UV at the pole. Under the per-endpoint rule all 24 pole edges read as cuts, and their far ends become 22 dead-end tips, so import would offer 22 darts the maker never meant [solid]. After a marked unwrap there are none.

**Decision** (maintainer, 2026-10-17, plan S-015): the rule stays exact. The import report adds a hint. A **projection point** is an interior vertex with at least `PROJECTION_MIN_FACES = 5` incident faces whose corner UVs there are pairwise more than `UV_CUT` apart. When any exist, the report lists them, says the UVs may come from an automatic projection rather than marked seams, and suggests marking seams and unwrapping. The proposals are unchanged.

- Five faces: at a projection point every incident edge is cut, so every incident face is its own island there. Hand-marked layouts rarely bring five seams to one vertex; a pole fan on a 12-segment sphere has 12 [speculative] (the threshold is checked against U070's tests, and a false hint costs one sentence of advice).
- *Rejected: treat projection points as uncut*. It would also hide a real five-way seam junction, which is a silent wrong output (Constitution III).
- *Rejected: no hint*. The 22 darts are declinable, but the maker gets no clue why they appeared.

## R3. Dead ends

**Decision**: build the seam graph (001 T070) over the union of UV cuts, material boundaries, and mandatory repair seams, with surface-boundary vertices counted as endpoints. Seams already in the project are left out: the file must be consistent on its own, because Replace would discard them. Any interior vertex of seam-valence 1 is a dead-end tip. Collect all tips, and walk each back to its chain: the cut edges from the tip to the first vertex of seam-valence 3 or more, or to the surface boundary. Each chain becomes one dart proposal (spec FR-011, 2026-10-17).

**Rationale**: 001's graph already classifies a "free endpoint" as a dart tip (data-model §4), and that is what these are. Spike Q1 (`research/q1_blender_uv_cuts/`) found that Blender's usual sphere unwrap, a meridian to the pole, makes one at each pole, and that the unwrap opens it into the wedge a dart closes [solid on UV spheres]. So the 2026-10-16 choice to make them an error would have refused correct models. Reusing the classifier means no second definition of "endpoint" to drift.

**Superseded**: on 2026-10-16 a dead end failed the import with one error listing every tip. Replaced on 2026-10-17 by the maintainer's choice of dart proposals (spec Clarifications).

**Alternative rejected**: *drop dead-ended chains and import the rest*. That gives a layout that differs from the maker's design with no clear sign of where, which is Constitution III's worst case (spec Assumptions).

## R4. Matching material names to fabrics

**Decision**: compare `trim(name)` with Unicode lowercasing (`str::to_lowercase`, which doesn't depend on locale) against each profile's name the same way. The first match in profile order wins; profile order is the project's stored order, so it is deterministic.

**New profiles** (decided by the maintainer 2026-10-16): the **minky preset's values** under the material's name, plus an advisory warning until the profile is edited (spec FR-025). 001 has no "new fabric" default; its presets are the only fabric values (001 research R7), all still `[verify]`.

- Minky is the most common plush fabric, so the guess is right most often [likely].
- Its failure modes when wrong (a felt or cotton part): nap arrows and a restricted rotation set (only costs layout efficiency), a 10 mm allowance (too wide is trimmable; too narrow is not), and **mirrored printing**, because minky is a pile fabric (001 FR-054). Mirrored printing is the costly one, which is why the warning is printed on the pattern and not only shown in the report.
- Alternatives considered: quilting-cotton values (conservative for warnings, but nap and mirrored printing are silently off for fur); no default, so accept-all is blocked until a preset is picked (reverses the maintainer's earlier decision).

**Alternatives rejected**:
- *Fuzzy matching* ("minky_fabric" → Minky): it guesses, and a wrong guess silently assigns the wrong fabric.
- *Refusing unknown names*: the maintainer ruled it out.

## R5. Re-importing into a project that already has seams

**Problem the spec doesn't show**: seams and regions are stored by **source provenance** (001 data-model §12): primitive and vertex indices in the file. Re-exporting an edited model renumbers both [solid], so after a re-import an existing seam's key may name nothing, or the wrong edge.

**Decision**: when a project has existing seams or regions and a new model replaces the old one, the old ones are re-placed **by position**, not by key. The project still holds the original mesh chunk (project schema `model.meshChunk`), so each old seam edge has two endpoint positions. It maps to the new edge whose endpoints coincide within the weld tolerance. Faces map the same way, by their three corner positions.

- Under **Merge**, an old seam or region face that maps to nothing is listed as *not placeable* in the difference (FR-051) and located on the old model. It is never dropped silently and never snapped to a nearby edge.
- Under **Replace**, old seams are discarded by the maker's choice, so mapping only feeds the "would be removed" count.

This needs one sentence in spec FR-051 (made in the same change as this plan). [likely] that positional matching holds up for typical edits (sculpting a limb leaves the rest of the mesh bit-identical); spike Q4 measures it.

**Alternative rejected**: *nearest-edge snapping*. Plausible, wrong, and silent.

## R6. Spikes

| ID | Question | Blocks | Effort |
|---|---|---|---|
| Q1 | On Blender exports (5.x, the current release) (glTF and OBJ) of the capsule and a quadruped: does every marked seam, and only those, read as a UV cut at `UV_CUT = 1e-5`? Does a dead-ended seam read as a slit? **Done 2026-10-17 on UV spheres** (`research/q1_blender_uv_cuts/results.md`): yes to both (the dead end on one slit), so `UV_CUT` stands (R2); projection maps make pole fans (R2); dead ends become darts (R3). The capsule and quadruped were not built | `UV_CUT` value; FR-011 detection | 0.5 day |
| Q2 | Which Blender UV map becomes `TEXCOORD_0`: active, or first listed? | FR-004 wording in the import report | 1 h |
| Q3 | ~~Does `tobj` split a model on `usemtl` inside one group?~~ Answered by 001 D-034: 001 reads OBJ with its own reader, which splits at `usemtl` | — | none |
| Q4 | Edit one limb of a quadruped in Blender, re-export, re-import: what share of untouched seams re-place by position? The spike builds its own quadruped, since Q1 did not | R5 confidence | 0.5 day |
