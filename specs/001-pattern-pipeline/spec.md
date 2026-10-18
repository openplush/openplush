# Feature Specification: 3D Model → Plush Sewing Pattern

**Feature Branch**: `001-pattern-pipeline`
**Created**: 2026-10-12
**Status**: Draft — blocking clarifications Q1–Q3 closed 2026-10-13; minor defaults still open
**Input**: "Take an arbitrary 3D mesh (OBJ/glTF), mark cut seams, flatten the surface patches into 2D with minimal distortion, add seam allowances, map surface details onto the flat pieces, and lay out printable patterns. Must cover Plushify's high-level features. Free, open source, open to contributors."

> Spec Kit rule: this file states **what** and **why** only. Technology choices live in [research.md](./research.md) and [plan.md](./plan.md). Data structures live in [data-model.md](./data-model.md).

## Parity Baseline

Publicly described Plushify capabilities this project must match (sources in [research.md](./research.md#r5-parity-sources)):

| Capability | Covered by |
|---|---|
| Import a 3D model (OBJ) | US1 · FR-001 |
| Mark seams in a 3D view | US1 · FR-010 |
| Generate the flat pattern automatically from seams | US1 · FR-020 |
| Quality visualization showing where darts or extra seams help | US2 · FR-021, FR-022 |
| Markings: user images placed on the 3D model | US5 · FR-040 |
| Colors / fabric regions | US5 · FR-041 |
| Any size, with human-scale preview | US1 · FR-006 |
| Printing auto-scaled to the user's paper | US1 · FR-061 |
| PDF output | US1 · FR-061 |
| Save / reload a project file | US7 · FR-070 |

Beyond parity: seam allowance, match notches, mirrored-piece deduplication, grain/nap orientation, fabric-aware layout, glTF, offline/private by construction, headless automation, open project format.

---

## Clarifications

### Session 2026-10-13

- Q: Is the imported model the finished stuffed toy, or the fabric shell that puffs out when stuffed? → A: The fabric shell (WYSIWYG). The model is understood as a basic, nearly-stuffed shape; the tool makes no promise of dimensional accuracy after stuffing.
- Q: Must paired seam edges match exactly in length, or may the tool leave a small difference for the sewist to ease in? → A: Exact — a hard constraint; distortion is absorbed in the piece interior. Fabric stretch is treated as uniform in all directions for this feature.
- Q: Does the first release propose seams automatically, or does the maker draw them all by hand? → A: Manual seams in v0.1, with validation and dart/extra-seam suggestions; automatic layout deferred. Upper size bound is handheld-to-large plush, not wearable/costume scale.
- Q: Does a warning block export, or can the maker print anyway? → A: Never blocks. Unresolved warnings are acknowledged once and printed on the pattern's first page, so flawed output is possible but never undocumented. Violated invariants remain errors and do block.
- Q: Must every warning come with a fix the maker can accept? → A: Every warning must name a remedy in plain language; where the tool can perform it, it is offered as a preview-then-accept action. Warnings whose real remedy is a material or sizing choice say so without acting.

---

## User Scenarios & Testing *(mandatory)*

### US1 — Seams in, printable pattern out (Priority: P1)
A maker imports a model, sets its real-world size, draws seams on it, and prints a true-scale pattern with seam allowances.

**Why P1**: This is the product. Every other story decorates this loop.
**Independent test**: Run the cube, capsule, and sphere fixtures through import → seams → print; measure printed pieces against expected dimensions.

**Acceptance scenarios**
1. **Given** a clean OBJ or glTF model, **When** the user imports it and sets height = 30 cm, **Then** the model displays at that size beside a human-scale reference.
2. **Given** a model with health problems, **When** it is imported, **Then** each problem is listed, highlighted in 3D, and offered a repair; nothing changes until the user confirms.
3. **Given** a closed model, **When** the user draws seams that leave a region unflattenable, **Then** the system highlights the region and proposes the minimal extra seam(s) to fix it.
4. **Given** valid seams, **When** the user generates the pattern, **Then** each piece shows a sewing line and a cut line offset by the seam allowance.
5. **Given** a pattern larger than one sheet, **When** the user exports for Letter or A4, **Then** pages tile with overlap, registration marks, a page map, and a scale-check square on every page.

### US2 — See where the pattern will fail (Priority: P1)
The maker sees, before cutting, where fabric would stretch, pucker, or not fit, and gets concrete suggestions (dart here, seam there).

**Why P1**: Parity feature, and the main way non-experts learn seam placement.
**Independent test**: Sphere fixture with one seam must show high distortion near the poles and suggest darts there.

**Acceptance scenarios**
1. **Given** generated pieces, **When** the user toggles the quality view, **Then** stretch/compression shows as a heatmap on both the 3D model and the 2D pieces, with a per-piece summary.
2. **Given** a region over the fabric's tolerance, **When** the user requests suggestions, **Then** the system proposes darts or seams, previews the improvement, and applies them only on acceptance.

### US3 — Sewing marks that make assembly unambiguous (Priority: P2)
Pieces carry match notches, partner labels ("A3 → C1"), cut counts, mirror flags, and a marked stuffing opening.

**Why P2**: The difference between a shape and a usable pattern.
**Independent test**: For every seam in the capsule fixture, both sides carry matching notch codes at equal arc-length positions.

**Acceptance scenarios**
1. **Given** a left/right symmetric model with mirrored seams, **When** the pattern is generated, **Then** mirror-image pieces appear once, labeled "cut 2 (mirrored)".
2. **Given** a seam segment marked as the opening, **When** exported, **Then** that segment is labeled and its allowance uses the opening allowance.
3. **Given** a tight concave curve, **When** exported, **Then** clip marks are drawn in its allowance; tight convex curves get notch-out marks.

### US4 — One-click starting seams (Priority: P2 — deferred past v0.1)
The maker requests an automatic seam layout, then edits it.

**Why P2**: Removes the blank-canvas problem; not required for a correct pattern.
**Independent test**: On every fixture, auto-seams produce flattenable pieces under the default distortion tolerance with no piece smaller than the minimum sewable size.

### US5 — Markings, colors, and detail parts (Priority: P2)
The maker places images/markings on the model, paints fabric regions, and marks small separate parts (eyes, nose, buttons) as details. These appear on the pieces as placement marks or appliqué outlines.

**Acceptance scenarios**
1. **Given** a marking that crosses a seam, **When** the pattern is generated, **Then** it is split across both pieces with alignment marks.
2. **Given** separate eye meshes, **When** the user marks them as details, **Then** each produces a placement mark on the underlying piece and no pattern piece of its own.
3. **Given** two painted fabric regions, **When** generated, **Then** pieces are grouped by fabric in the output.

### US6 — Fabric-aware orientation and layout (Priority: P3, partially in v0.1)
The maker chooses a fabric (e.g., faux fur with nap), sets fur direction on the model, and gets pieces oriented so nap runs correctly, laid out to minimize fabric use.

**Acceptance scenarios**
1. **Given** a nap direction drawn on the model, **When** generated, **Then** every piece shows grain and nap arrows consistent with the 3D direction.
2. **Given** a piece whose nap direction varies strongly across it, **When** generated, **Then** the system warns and suggests a split.
3. **Given** a napped fabric, **When** laid out, **Then** no piece is rotated against the nap.

### US7 — Save, reopen, share a project (Priority: P2)
The maker saves one portable file, reopens it later (or on another machine), and continues editing with full undo history from that point.

### Edge Cases
- Non-manifold edges (3+ faces share an edge) and bowtie vertices; flipped normals; non-orientable surfaces.
- glTF exports that split vertices at UV seams (model appears full of holes until welded).
- Multiple disconnected components: limbs modeled separately, eyes as spheres, stray debris.
- Intentional openings (puppet hand hole) vs. accidental holes.
- Closed model with zero seams (must cut at least once).
- Seams that form a closed loop vs. seams that dead-end inside a piece (darts).
- Pieces below minimum sewable size; seam curves tighter than the allowance.
- Ears or fins sandwiched into a seam (more than two layers meeting along one seam line).
- Markings straddling a seam; markings on a region later re-seamed.
- Extreme scales (below 75 mm or above 1000 mm, FR-006), very dense meshes, quads/n-gons.
- Concave regions without a seam (stuffing will push them outward — FR-024, FR-025).
- Seams joining fabrics with different stretch (fur to minky).
- Pile longer than a detail it covers.
- Thick backing on a small plush.

---

## Requirements *(mandatory)*

### Import & model health
- **FR-001**: Accept OBJ (+MTL, textures) and glTF 2.0 (`.gltf`/`.glb`) static meshes. *2026-10-16: MTL and texture files are read once texture transfer (FR-043) needs them; until then OBJ import keeps geometry and per-corner texture coordinates (D-034).* Skinned/animated data is ignored with a warning. [NEEDS CLARIFICATION: STL/PLY in MVP? Recommended yes (cheap). FBX recommended no.]
- **FR-002**: Triangulate polygons; preserve per-corner texture coordinates and material assignment.
- **FR-003**: Detect and report non-manifold edges/vertices, inconsistent orientation, degenerate/duplicate faces, holes, disconnected components, and self-intersections; each defect is locatable in 3D.
- **FR-004**: Offer repair per defect class with a before/after report; never modify geometry without confirmation (or an explicit per-project auto-repair setting).
- **FR-005**: Classify small disconnected components as candidate detail parts the user can mark as non-sewn.
- **FR-006**: User sets real-world size (by a dimension or a measured reference distance) and units; a human-scale reference is shown. Supported range is 75 mm to 1000 mm in the longest finished dimension, the smallest plush that can still be turned and stuffed up to large plush; outside it the maker is warned, never blocked. Wearable/costume scale is out of scope.

### Seams
- **FR-010**: Draw and erase seams along the surface in 3D (point-to-point with automatic shortest path), with an optional symmetry-mirroring mode.
- **FR-011**: Support darts (seams that end inside a piece).
- **FR-012**: Mark any seam segment as the stuffing/turning opening.
- **FR-013**: Validate that seams split the surface into flattenable pieces; otherwise propose the minimal additional seams, highlighted for approval.
- **FR-014**: Piece previews update incrementally as seams change (only affected pieces recompute).
- **FR-015**: Generate an automatic starting seam layout, fully editable afterwards. Deferred past v0.1 — the first release ships manual seam drawing with validation (FR-013) and dart suggestions (FR-022), which an automatic layout is expected to build on.
- **FR-016**: Support inserted pieces (more than two layers along one seam, e.g., ears sandwiched into a head seam). Deferred past v0.1; the project format must represent such seams from the start so earlier files stay valid.

### Flattening & quality
- **FR-020**: Flatten each piece to 2D with no folds and no self-overlap.
- **FR-021**: Show per-region stretch/compression on 3D and 2D, with a numeric per-piece summary.
- **FR-022**: Suggest darts or seams where distortion exceeds the selected fabric's tolerance, with preview before applying.
- **FR-023**: Both sides of every seam yield equal sewing lengths within max(1.0 mm, 0.5%) — a hard constraint, never a residual handed to the sewist as ease. The unavoidable distortion is absorbed in the piece interior instead, where FR-021 shows it and FR-022 can relieve it. A pairing that cannot meet this is an error, not a warning.
- **FR-024**: The imported model is the fabric shell: pieces are cut to match its surface, and the model is read as a basic, nearly-stuffed shape rather than a prediction of the stuffed result. The maker is told this plainly — a stuffed toy reads fuller than the model, most visibly at concavities, and no dimensional accuracy is promised after stuffing. The optional uniform shrink percentage (FR-056) is the only compensation offered.
- **FR-025**: Warn when a concave region has no seam passing through it.
- **FR-026**: Fabric stretch is treated as uniform in all directions for this feature. FR-050's per-direction percentages are recorded and drive warnings (FR-058) but do not steer flattening; direction-dependent flattening is out of scope.

### Pattern features
- **FR-030**: Seam allowance outside the sewing line: global default, per-seam override, separate opening allowance, hem allowance for unsewn edges. [NEEDS CLARIFICATION: default — 6 mm vs. ¼ in, locale-dependent?]
- **FR-031**: Sewing line and cut line drawn distinctly.
- **FR-032**: Match notches on paired seams with unambiguous pairing codes, at matching arc-length positions.
- **FR-033**: Piece labels: ID, name, how many to cut as printed and how many flipped, fabric, grain and nap arrows, seam-partner labels.
- **FR-034**: Identical and mirror-image pieces deduplicated into one template ("cut N", "cut N mirrored"). Pieces that only nearly match are merged only when the maker accepts, after seeing how far apart they are (D-048).
- **FR-035**: Clip marks on tight concave curves; notch-out marks on tight convex curves.
- **FR-036**: Assembly order hints. (P3)

### Markings & texture
- **FR-040**: Place images/markings on the 3D surface; they map onto pieces, splitting across seams with alignment marks.
- **FR-041**: Paint fabric/color regions on the model; region boundaries become seams, and pieces are grouped by fabric.
- **FR-042**: Detail parts (FR-005) become placement marks on the piece beneath them.
- **FR-043**: Transfer the model's texture onto pieces as printable images for print-on-fabric, with bleed into the allowance. Deferred past v0.1.

### Fabric & grain
- **FR-050**: Fabric profiles: name, category (woven, knit, pile, felt), stretch along grain / across grain / bias (%), nap (yes/no), pile length (mm), thickness (mm), frays (yes/no), default seam allowance, roll width. Ship editable presets for quilting cotton, felt, fleece, minky, velboa, short faux fur and long faux fur, each only once someone who sews has measured it; fleece, minky and long faux fur come first. Profiles are saved in the project.
- **FR-051**: User-defined nap/grain direction on the model via strokes, with a sensible default when none is given.
- **FR-052**: Each piece is oriented so grain/nap align with the fabric; poor nap coherence within a piece triggers a warning and split suggestion.
- **FR-053**: Layout honors allowed rotations per fabric (napped: none; non-napped: 180°; felt, which has no grain: any quarter turn). No piece is placed on the bias.
- **FR-054**: For pile fabrics, export asks the maker each time whether to print pieces mirrored, labeled "trace on wrong side", or as-is, labeled "lay face down on wrong side"; there is no default.
- **FR-055**: Default seam allowance scales with pile length; warn when a piece, detail part, or curve is smaller than twice the pile length.
- **FR-056**: Optional uniform shrink compensation (%) for stretch fabrics so the stuffed toy matches the model size; direction-dependent compensation is out of scope for this feature.
- **FR-057**: Non-fraying fabrics may use zero allowance for edge-stitched or appliqué pieces.
- **FR-058**: Warn when a seam joins fabrics whose stretch differs by more than 20 percentage points, stated to the maker as "over a 100 mm span, one fabric stretches N mm further".

### Output
- **FR-060**: SVG export at true scale (document dimensions in mm; 1 user unit = 1 mm).
- **FR-061**: Print export tiled for Letter or A4 (the maker picks, defaulting by region) with overlap, registration marks, page map, and scale-check square; also a single-sheet large-format option. Pages are joined by overlapping without trimming: a 15 mm overlap band carries matching marks and the pattern lines, each page is labeled by row letter and column number, page 1 carries the page map, and the scale-check square is 50 mm with a labeled 1 in square inside it. [verify: the 15 mm overlap on a test print (D-045)] [NEEDS CLARIFICATION: PDF in MVP — recommended yes; browsers print SVG at unreliable scale]
- **FR-062**: Fabric-saving layout within a given fabric width. v0.1 places pieces by a simple rule and reports the fabric length needed; genuinely optimized nesting is deferred.
- **FR-063**: Export pattern geometry and metadata as open JSON; DXF for cutting machines. (P3)
- **FR-064**: A warning never blocks export. Before exporting a pattern that still has unresolved warnings, the maker acknowledges them once, and a summary of each unresolved warning is printed on the pattern's first page — so a printed pattern always carries its own known problems. A violated geometric invariant (e.g. FR-023) is an error, not a warning, and does block export.
- **FR-065**: Every warning states in plain language what the maker can do about it; none describes only the problem. Where the tool can carry out the remedy it is offered as a preview-then-accept action (extra seams per FR-013, darts per FR-022, piece splits per FR-052). Where the real remedy is a material or sizing choice (FR-055, FR-058), the warning names that choice without acting on it.

### Project
- **FR-070**: Save/load a single portable, versioned project file containing the model and all edits; older files migrate forward.
- **FR-071**: Undo/redo for every edit.

### Platform & privacy
- **FR-080**: Runs in current desktop browsers without installation; works offline after first load.
- **FR-081**: No account and no upload; model data never leaves the device.
- **FR-082**: Every capability is also available headless for automation and testing.
- **FR-083**: Heatmaps use color-blind-safe palettes; non-3D controls are keyboard-operable.

### Key Entities
- **Model** — the imported surface, its size, and its source attributes (textures, materials).
- **Health Report** — defects found and repairs applied.
- **Seam / Dart / Opening** — user-drawn or proposed cut paths with a type.
- **Piece** — one flat region of fabric, with sewing line, cut line, marks, labels.
- **Seam Pairing** — which piece edges sew together (two sides, or more for inserted pieces).
- **Fabric** — material properties that constrain distortion, orientation, allowance, layout; includes pile length, thickness, fray, category.
- **Marking / Detail Part** — surface-attached decoration or separate small part.
- **Sheet** — a printed page or fabric layout holding placed pieces.
- **Project** — everything above, saved as one file.

---

## Out of scope (future features)

Named here so they are not silently assumed in scope:

- Stuffed-shape preview (inflation simulation).
- Inverse inflation — the pattern adjusted so the stuffed result matches the model.
- Direction-dependent stretch compensation (see FR-056, which is uniform only).
- Thickness compensation.
- Shave-zone markings.

---

## Success Criteria *(mandatory)*

- **SC-001**: On the reference fixture set, every seam pairing matches in length within max(1.0 mm, 0.5%). Hard pass/fail — no residual is reported as ease.
- **SC-002**: On the reference fixture set with reference seams, ≥ 95% of surface area has stretch within ±5%.
- **SC-003**: Every exported page's scale-check square is exactly 50.00 mm in the file; printed at 100%, it measures within ±0.5 mm on a typical home printer.
- **SC-004**: In a usability test (n ≥ 5), first-time users with a prepared model produce a printable pattern in ≤ 15 minutes.
- **SC-005**: A 50k-triangle model goes from seam edit to updated preview in ≤ 10 s in a current browser on a 2020-era mid-range laptop.
- **SC-006**: Zero network requests during a working session after first load (automated test).
- **SC-007**: The defect corpus (one fixture per defect class) is detected and reported at 100%.
- **SC-008**: Identical project + settings produce byte-identical exports across runs and platforms.

---

## Open Clarifications

The three decisions that blocked `/plan` finalization (Q1–Q3) closed on 2026-10-13; see [Clarifications](#clarifications).

Remaining minor decisions, with the default that applies if nobody answers:
- STL/PLY import (default: yes) — FR-001.
- PDF print export in v0.1 (default: yes) — FR-061.
- Default seam allowance (default: 6 mm in metric locales, ¼ in in the US) — FR-030.
