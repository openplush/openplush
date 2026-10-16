# Research: Import Seams and Fabric Regions from the Source Model

**Feature**: `003-source-seam-import` | **Date**: 2026-10-16 | **Status**: Active

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
| `tobj` (001's OBJ reader, T050) starts a new model at each `o`/`g` and gives each model one `material_id`; whether it also splits on a `usemtl` change inside one group is unconfirmed | [verify] spike Q3 |
| A dead-ended Blender seam (one that doesn't separate the UV island) still gives its edges different UVs on each side: it shows up in the UV editor as a slit. So a dead end is detectable from the export alone | [likely] spike Q1 |

**Decision**: read only what the file carries. UV cuts come from per-corner texture coordinates, and materials from the primitive a face came from. No `.blend` reading (out of scope).

## R2. Detecting a UV cut

**Decision**: an interior edge is a UV cut when, at either endpoint, the corner UV on one side differs from the corner UV on the other by more than `UV_CUT` (absolute, in UV units). Only edges with UVs on both sides are tested. An edge with a UV-less side is never a cut, and those faces are counted in the report.

**Rationale**:
- Testing per endpoint, not per edge midpoint, catches a cut that runs into a junction: one endpoint's UVs agree while the other's differ [solid].
- The tolerance is absolute because UV space is dimensionless. The model-space tolerances (relative to the bbox diagonal) and the physical ones (mm) both mean nothing here. glTF stores UVs as `f32`, and an exporter writes one value per deduplicated vertex, so an uncut edge normally carries bit-identical UVs [likely]. `1e-5` sits about two orders above `f32` spacing near 1.0 and far below any real cut. UDIM tiles (u up to ~10) still clear it [likely]. It is a provisional value until spike Q1 confirms it.
- Requiring UVs on both sides keeps repair-filled faces (T066, which have no source UVs) from creating a ring of false cuts around every filled hole. That is the mechanism behind spec FR-024's "never create a seam of their own".

**Alternatives rejected**:
- *Islands by UV connectivity and seams as island borders*: misses dead-ended slits, which are exactly what FR-011 must catch.
- *Tolerance relative to UV bounding box*: a model with one tiny UV island and one huge one would get a threshold that is right for neither.
- *Comparing normals too*: FR-001 forbids it. Hard-shaded edges are not cuts.

## R3. Dead ends

**Decision**: build the seam graph (001 T070) over the union of UV cuts, material boundaries, and mandatory repair seams, with surface-boundary vertices counted as endpoints. Seams already in the project are left out: the file must be consistent on its own, because Replace would discard them. Any interior vertex of seam-valence 1 is a dead-end tip. Collect all tips, walk each back to its chain, and fail with one error listing every tip and chain.

**Rationale**: 001's graph already classifies a "free endpoint" as a dart tip (data-model §4). Here the same class is an error, because FR-010 rules out imported darts. Reusing the classifier means no second definition of "endpoint" to drift. Reporting all tips at once is the spec's edge case ("not one per attempt").

**Alternative rejected**: *drop dead-ended chains and import the rest*. That gives a layout that differs from the maker's design with no clear sign of where, which is Constitution III's worst case (spec Assumptions).

## R4. Matching material names to fabrics

**Decision**: compare `trim(name)` with Unicode lowercasing (`str::to_lowercase`, which doesn't depend on locale) against each profile's name the same way. The first match in profile order wins; profile order is the project's stored order, so it is deterministic.

**New profiles**: 001 has no "new fabric" default. Its presets are the only fabric values (research R7 of 001). This feature needs one, and so will 001's own "add fabric" UI. **Proposed**: the quilting-cotton preset values under the material's name. Woven with near-zero stretch is the conservative assumption: distortion warnings fire earliest, and `frays = true` keeps the allowance non-zero (FR-057). [likely] that this is the safest default; **maintainer to confirm**.

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
| Q1 | On Blender 4.x exports (glTF and OBJ) of the capsule and a quadruped: does every marked seam, and only those, read as a UV cut at `UV_CUT = 1e-5`? Does a dead-ended seam read as a slit? | `UV_CUT` value; FR-011 detection | 0.5 day |
| Q2 | Which Blender UV map becomes `TEXCOORD_0`: active, or first listed? | FR-004 wording in the import report | 1 h |
| Q3 | Does `tobj` split a model on `usemtl` inside one group? If not, the OBJ importer must split primitives itself (S-002) | T050-side work | 1 h |
| Q4 | Edit one limb of a fixture in Blender, re-export, re-import: what share of untouched seams re-place by position? | R5 confidence | 0.5 day |
