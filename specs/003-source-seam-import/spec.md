# Feature Specification: Import Seams and Fabric Regions from the Source Model

**Feature Branch**: `003-source-seam-import`

**Created**: 2026-10-16

**Status**: Draft — clarifications closed 2026-10-16

**Input**: User description: "Import seams and fabric regions from the source model. When a maker imports an OBJ or glTF model that was UV-unwrapped and material-assigned in a modelling tool (e.g. Blender), the tool reads the model's UV cuts (edges whose two sides carry different texture coordinates) and material boundaries and offers them as seams and fabric regions. Decisions already made by the maintainer: (1) UV cuts become ordinary seams; an imported UV cut never becomes a dart. (2) A UV cut that dead-ends inside the surface is an error the maker fixes in the source tool — never silently dropped and never converted to a dart. (3) If the imported seams leave a piece that cannot lie flat, the engine computes the minimal extra seams and presents them already highlighted in the import preview with a single 'accept all' action; nothing is applied without the maker's acceptance (Constitution Principle VII). (4) Every material becomes a fabric region; region boundaries become seams. A material whose name matches no fabric profile gets a new profile with default properties, named after the material, listed in the import report and editable. (5) Imported seams and regions are shown as a preview the maker accepts or declines; after acceptance they behave exactly like hand-drawn seams and painted regions. Out of scope: reading Blender's native seam flags or .blend files, the Blender add-on, using the source UV layout as the flat pattern, automatic seam layout."

> Spec Kit rule: this file states **what** and **why** only. Technology choices live in `research.md` and `plan.md`. Data structures live in `data-model.md`.

Builds on feature 001 (`specs/001-pattern-pipeline/`): import, repair, seam drawing, flattenability validation (FR-013), painted fabric regions (FR-041), and fabric profiles (FR-050) already exist. Feature 001 keeps each triangle corner's texture coordinates through import and repair but never acts on them, and its FR-002 promise to keep material assignment is not yet carried by its design. This feature adds the material names and acts on both, so a maker who already cut and organised the model in a modelling tool doesn't have to do that work again.

## Clarifications

### Session 2026-10-16

- Q: Does an imported UV cut ever become a dart? → A: No. Every imported UV cut is an ordinary seam. *(Superseded 2026-10-17, below.)*
- Q: What happens to a UV cut that ends inside the surface instead of separating it? → A: An error. The maker fixes it in the modelling tool. It is never dropped and never converted to a dart. *(Superseded 2026-10-17, below.)*
- Q: If the imported seams leave a piece that cannot lie flat, does the engine fix it on its own? → A: The engine computes the minimal extra seams and shows them already highlighted in the import preview, with one "accept all" action. They are still proposals until the maker accepts them (Constitution VII).
- Q: What happens to a material whose name matches no fabric profile? → A: A new profile with default properties is created, named after the material, listed in the import report, and editable.
- Q: What settings does a profile created for an unmatched material start with? → A: The minky preset's values, plus an advisory warning printed on the pattern until the maker edits the profile (FR-025). Minky is the most common plush fabric; the warning covers the cases where it is wrong.
- Q: When a model is imported into a project that already has seams or regions, what happens to them? → A: The maker chooses Replace or Merge in the import preview each time, seeing the difference first. Neither is applied silently.

### Session 2026-10-17

- Q: Spike Q1 found that the usual way to unwrap a sphere in Blender, a seam from pole to equator, ends inside the surface: under the earlier answer it would fail import, yet Blender's unwrap opens it into exactly the wedge a dart closes. What happens to a UV cut that ends inside the surface? → A: It is offered as a **dart proposal**, highlighted in the preview and explained, never applied without the maker's acceptance. It replaces the 2026-10-16 answers that imported cuts never become darts and that a dead end is an error. Other UV cuts stay ordinary seams.
- Q: Spike Q1 found that a UV map made by automatic projection, without marked seams, gives every face around a sphere's pole its own texture coordinate there, so the pole reads as many short cuts and import would offer many darts nobody meant. What does import do? → A: It reads the cuts exactly as the file says, and the import report adds a hint that the texture coordinates look like an automatic projection rather than marked seams, naming where, and suggesting the maker mark seams and unwrap. The proposals are unchanged.
- Q: Spike Q2 found that a glTF export carries every UV map in the modelling tool's list order, and import reads the first, while an OBJ export carries only the map that was active. A maker who marked seams on a later map and exported glTF would get the wrong map's cuts. Does import warn? → A: Yes. When a model carries more than one set of texture coordinates, the import report warns that only the first was read, that it is the first map in the modelling tool's list, and that seams marked on another map are not the ones shown.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Bring my UV seams in as sewing seams (Priority: P1)

A maker unwraps a model in their modelling tool, marking the seams where the fabric will be cut, and exports it. On import, OpenPlush shows those cuts on the model as proposed seams. The maker accepts them and goes straight to the flat pattern without drawing a single seam.

**Why this priority**: This is the feature. Makers who already know their modelling tool place seams there. Making them redraw every seam by hand in a second tool is duplicate work, and a slip while redrawing gives a pattern that doesn't match the design they intended.

**Independent Test**: Import a capsule fixture whose UVs are cut along a known seam layout. Confirm the proposed seams match that layout edge for edge. Accept them, generate the pattern, and confirm one piece per UV island.

**Acceptance Scenarios**:

1. **Given** a model whose texture coordinates are cut along a set of edges, **When** the maker imports it, **Then** every one of those edges is shown as a proposed seam, and no other edge is.
2. **Given** proposed imported seams, **When** the maker accepts them, **Then** they become seams that behave exactly like hand-drawn seams from then on: editable, erasable, undoable.
3. **Given** proposed imported seams, **When** the maker declines them, **Then** the model has no seams from the import and is left exactly as a plain import would leave it.
4. **Given** a model with no texture coordinates, **When** it is imported, **Then** the import report says that no UV cuts were found and why, and nothing is proposed.

---

### User Story 2 - Pieces that can't lie flat are fixed in the same step (Priority: P1)

The maker's UV cuts leave a region, such as a body tube between a neck cut and a waist cut, that can't lie flat as one piece. Instead of a dead end, the import preview already shows the fewest extra seams that fix it, highlighted differently from the maker's own cuts. One action accepts everything.

**Why this priority**: UV unwrapping tolerates stretch that fabric can't. Without this, a typical import fails flattenability and leaves the maker to find the fix alone. With it, import reaches a valid pattern in one step.

**Independent Test**: Import a sphere fixture whose two ring-shaped UV cuts leave a band between them that can't lie flat. Confirm the preview contains the imported cut plus highlighted extra seams. After "accept all", every piece flattens.

**Acceptance Scenarios**:

1. **Given** imported seams that leave one or more pieces unflattenable, **When** the preview is shown, **Then** it includes the minimal extra seams, visually distinct from the imported ones, each explaining in plain language why it is needed.
2. **Given** that preview, **When** the maker chooses "accept all", **Then** both the imported seams and the extra seams become ordinary seams in one undoable step.
3. **Given** that preview, **When** the maker declines the extra seams but keeps the imported ones, **Then** only the imported seams are applied, and feature 001's flattenability validation reports the unflattenable pieces as it would for hand-drawn seams.
4. **Given** imported seams under which every piece already lies flat, **When** the preview is shown, **Then** no extra seams are proposed, and the preview says so.

---

### User Story 3 - My materials become my fabrics (Priority: P1)

The maker assigned materials named "Minky", "Fleece" and "Felt Nose" to parts of the model. On import, each material becomes a fabric region, the boundaries between them become seams, and each piece is assigned the fabric of its region. "Felt Nose" matches no profile, so a new profile named "Felt Nose" with default properties appears, flagged in the report for the maker to edit.

**Why this priority**: Material assignment is how makers already organise a model by fabric. Without this, every region has to be painted again by hand, and a missed spot gives a piece cut from the wrong fabric.

**Independent Test**: Import a fixture with three materials: two whose names match shipped presets and one that matches none. Confirm three regions, seams along every material boundary, the two matching presets assigned, and one new default profile named after the third material, listed in the import report.

**Acceptance Scenarios**:

1. **Given** a model with several materials, **When** it is imported, **Then** each material becomes one proposed fabric region, and every edge between two different materials becomes a proposed seam.
2. **Given** a material whose name matches an existing fabric profile, **When** it is imported, **Then** its region is assigned that profile.
3. **Given** a material whose name matches no fabric profile, **When** it is imported, **Then** a new profile with default properties is created, named after the material, listed in the import report as newly created, and editable like any other profile.
4. **Given** a model with exactly one material, **When** it is imported, **Then** it becomes one region with no boundary seams.
5. **Given** accepted imported regions, **When** the maker repaints a region, **Then** it behaves exactly like a region painted by hand.

---

### User Story 4 - A cut that stops partway is shown as a dart, not guessed at (Priority: P2)

The maker left a UV cut that stops partway across the surface. Often that is deliberate: a seam from a sphere's pole to its equator lets the unwrap open a wedge, and sewn shut that wedge is a dart. Sometimes it is a slip. Import shows each such cut, highlighted as a dart it would make, explains why, and lets the maker accept it or decline it and fix the model.

**Why this priority**: The commonest way to unwrap a round shape makes cuts like this, so refusing them would turn away correct models. Applying them silently would turn a slip into a dart the maker didn't design, discovered after cutting fabric (Constitution III). It ranks below P1 because it refines the main flow rather than delivering it.

**Independent Test**: Import a fixture with deliberate dead-ended UV cuts. Confirm each is proposed as a dart, located and explained, separately from the imported seams; that accepting it adds a dart; and that declining it leaves that cut out while the rest of the import stands.

**Acceptance Scenarios**:

1. **Given** a model with a UV cut that ends inside the surface, **When** it is imported, **Then** the preview shows that cut as a dart proposal, located on the model, with a plain-language explanation that the cut stops partway and will be sewn as a dart.
2. **Given** that proposal, **When** the maker declines it, **Then** that cut is not imported at all, the rest of the import is unaffected, and the maker may fix the cut in the modelling tool and re-import.
3. **Given** a UV cut that ends exactly where it meets another seam, a material boundary or the edge of an opening, **When** it is imported, **Then** it is not a dead end and no dart is proposed for it.

---

### Edge Cases

- A model exported with vertices split at every UV and normal discontinuity. Only texture-coordinate cuts count; a split for normals alone (a hard-shaded edge) is not a seam.
- Texture coordinates that differ by floating-point noise across an edge the maker never cut.
- A UV cut and a material boundary that run along the same edge: one seam, not two.
- A UV cut that ends on an opening or hole boundary, or on a material boundary (not a dead end), versus one that ends in the open surface (a dead end).
- Several dead-ended cuts in one model: each becomes its own dart proposal, all shown at once.
- A cut that stops partway by mistake: it is shown as a dart proposal the maker can decline, never applied silently.
- A closed UV cut loop that separates a small island below feature 001's minimum sewable size.
- Faces with no material, mixed into a model where other faces have one.
- Two materials whose names differ only in letter case or surrounding spaces.
- Two materials that map to the same fabric profile: their shared boundary is still a seam, because the maker assigned them separately.
- A material named like a profile the maker has since edited or deleted.
- Faces added by repair (a filled hole) carry no texture coordinates or material of their own.
- Edges that repair turned into mandatory seams (non-manifold splits) coinciding with an imported cut.
- A model with more than one set of texture coordinates.
- Texture coordinates from an automatic projection, with no marked seams: every face around a pole has its own texture coordinate there. The cuts are read as the file says, and the report adds a hint (FR-006).
- Mirrored UV islands, or several islands stacked on the same texture area: the layout doesn't matter, only the cuts.
- The imported cuts leave a closed surface uncut: the extra-seam proposal must still make at least one cut.
- A model already in a project with hand-drawn seams, imported again after editing in the modelling tool.

## Requirements *(mandatory)*

### Functional Requirements

#### Reading the source model

- **FR-001**: An edge is a **UV cut** when, at either of its endpoints, the texture coordinates on its two sides differ by more than a documented tolerance. Differences in normals or other attributes alone never make an edge a UV cut.
- **FR-002**: An edge is a **material boundary** when its two sides belong to different materials.
- **FR-003**: Edges on the surface's boundary (holes kept as openings) are never imported seams; they are already unsewn edges.
- **FR-004**: Only the first set of texture coordinates is read. When a model has more than one set, the import report warns, before acceptance, that only the first was read and how many others were ignored. It says the first set is the first UV map in the modelling tool's list, not necessarily the one being edited, and that if the seams were marked on another map the maker should make that map the first, or export in a format that carries only the edited map, and import again.
- **FR-005**: A model with no texture coordinates or no materials imports normally. The import report states which of the two sources was absent. With no texture coordinates, no UV-cut seams are proposed. With no material on any face, no regions or material-boundary seams are proposed.
- **FR-006**: When the texture coordinates look like an automatic projection rather than marked seams (an interior point where every one of at least a documented number of surrounding faces has its own texture coordinate), the import report says in plain language that they may come from an automatic projection, locates each such point, and suggests marking seams and unwrapping in the modelling tool. The hint changes no proposal: the cuts are still read exactly as FR-001 defines.

#### Seams from UV cuts

- **FR-010**: Every UV cut that does not end inside the surface becomes a proposed seam of the ordinary seam type. An imported UV cut is never an opening.
- **FR-011**: A **dead end** is an endpoint of an imported seam that lies inside the surface and touches no other seam: no other imported seam, material boundary, mandatory repair seam, or surface boundary. The chain of cut edges that runs from a dead end to the nearest seam junction or surface boundary is proposed as a **dart**, one proposal per chain, located on the model and explained in plain language: the cut stops partway, so it will be sewn as a dart, and if that was not intended the maker can decline it and close or remove the cut in the modelling tool.
- **FR-012**: A dart proposal's chain is not part of the imported seams. Accepting the proposal adds that chain as a dart; declining it leaves that chain uncut. Either way the rest of the import is unaffected.
- **FR-013**: Dead ends are judged on the combined set of imported seams, material-boundary seams, and mandatory repair seams, so a cut that runs into another seam is never reported. Seams already in the project are not part of that set: the source file must be consistent on its own, because a Replace would discard them.
- **FR-014**: A closed cut that separates a piece below the minimum sewable size is still proposed, and the preview marks that piece as below the minimum before acceptance, which stays the maker's call (as feature 002 FR-036 does for hints). The minimum is the one feature 001 uses for its own proposals.
- **FR-015**: Dart proposals are shown distinctly from imported seams and extra seams, with their count, and "accept all" includes them. Partial acceptance (FR-042) can leave them out.

#### Fabric regions from materials

- **FR-020**: When at least one face has a material, every material becomes one proposed fabric region, and faces from the source file with no material form one region named "No material", matched to a profile under FR-022 and FR-023 like any other name.
- **FR-021**: Every material boundary becomes a proposed seam of the fabric-boundary type. Where a material boundary and a UV cut share an edge, one seam is proposed.
- **FR-022**: A region is matched to a fabric profile whose name equals the material's name, ignoring letter case and leading or trailing spaces.
- **FR-023**: A material that matches no profile gets a new profile named after the material, with a documented set of default properties. Each new profile is listed in the import report as newly created and is editable like any other.
- **FR-025**: Until the maker edits it, each profile created under FR-023 carries an advisory warning that names it and says its settings are guesses. Like every unresolved warning, it is printed on the pattern, so an unreviewed guess can't reach the cutting table unnoticed. Editing the profile clears it.
- **FR-024**: Faces added by repair, which have no material, join the region that borders the largest share of their perimeter, with a deterministic tie-break. They never create a seam of their own, and the import report says which region each filled area joined.

#### Making every piece flat

- **FR-030**: When the proposed seams leave any piece that can't lie flat, the engine computes the minimal extra seams using feature 001's flattenability validation (001 FR-013) and includes them in the preview, highlighted distinctly from imported seams.
- **FR-031**: Each extra seam states in plain language why it is needed, as 001's proposed seams do.
- **FR-032**: Extra seams are proposals. They are applied only when the maker accepts them, never automatically (Constitution VII).

#### Preview and acceptance

- **FR-040**: The import preview shows imported seams, dart proposals, imported regions, and extra seams on the model, each distinguishable from the others with colour-blind-safe palettes, and lists their counts.
- **FR-041**: One "accept all" action applies the imported seams, dart proposals, regions, new profiles, and extra seams together as a single undoable step.
- **FR-042**: The maker can instead accept the imported seams and regions without the extra seams, or decline everything. Declining leaves the project exactly as a plain import would.
- **FR-043**: Once accepted, imported seams and regions are indistinguishable in behaviour from hand-drawn seams and painted regions: editable, erasable, saved, and undoable the same way.
- **FR-044**: The preview is keyboard-operable, including "accept all", partial acceptance, and declining.

#### Re-import

- **FR-050**: When a model is imported into a project that already has seams or regions, the import preview asks the maker to choose, for that import, between **Replace** (the imported seams and regions take the place of all existing ones) and **Merge** (imported seams and regions are added to the existing ones; an imported seam on an edge that is already a seam is not duplicated). Neither is preselected.
- **FR-051**: Before the maker commits either choice, the preview shows the difference against the project: seams and regions that would be added, and, for Replace, those that would be removed. Hand-drawn seams that Replace would remove are called out by count. Under Merge, existing seams and regions that cannot be placed on the newly imported model are listed and located on the previous model; none is dropped silently or moved to a nearby edge.
- **FR-052**: Replace and Merge are each a single undoable step, and declining leaves the project unchanged.

#### Platform

- **FR-060**: Seam and region import, the extra-seam proposal, and the import report are available headless, without the 3D view (Constitution II).
- **FR-061**: The same file with the same settings produces an identical set of proposed seams, regions, profiles, and extra seams in an identical order, on every platform and every run (Constitution V).
- **FR-062**: All reading runs on the maker's device. No model data leaves the machine and no network request is made (Constitution VIII).

### Key Entities *(include if feature involves data)*

- **UV Cut**: An edge whose two sides carry different texture coordinates in the source file. It is the import's reading of where the maker cut the model.
- **Material Boundary**: An edge between two differently-assigned materials.
- **Import Proposal**: The set of imported seams, imported regions, new fabric profiles, and extra seams awaiting the maker's decision.
- **Dead-End Cut**: An imported seam endpoint inside the surface that touches no other seam. The chain ending there is proposed as a dart.
- **Re-import Choice**: The maker's per-import decision to replace or merge with existing seams and regions, with the difference shown before it is applied.
- **Import Report**: What was found and proposed. It records which UV set was read, how many seams came from each source, regions and their matched or new profiles, extra seams, repair-filled areas and the region each joined, any projection hint (FR-006) with its locations, and any errors with their locations.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On every fixture with a known UV cut layout, the proposed seams match that layout exactly: no edge missing and no edge extra.
- **SC-002**: On every fixture whose model was exported with vertices split for hard shading, zero proposed seams come from shading splits alone.
- **SC-003**: On every fixture, "accept all" yields seams under which every piece lies flat, 100% of the time.
- **SC-004**: On every fixture with deliberately dead-ended cuts, every dead end gets exactly one dart proposal, and fixtures with none get no dart proposals.
- **SC-005**: On every fixture with materials, every face belongs to exactly one region, and every material boundary is a seam.
- **SC-006**: A maker whose model is already cut and material-assigned reaches a valid flat pattern from import in at most 3 actions (import, accept all, generate).
- **SC-007**: The same file and settings produce a byte-identical import report across repeated runs and across platforms.
- **SC-008**: Zero network requests are made during import, preview, or acceptance.
- **SC-009**: Reading UV cuts and material boundaries adds no more than 1 second to the import of a 50,000-triangle model on a 2020-era mid-range laptop.

## Out of scope

- Reading the modelling tool's own seam flags or native project files (such as `.blend`); only what an OBJ or glTF export carries.
- The Blender add-on (feature 001's post-MVP plan). It is the longer-term answer to most of this feature's guesswork; see the plan's Phasing.
- Using the source UV layout as the flat pattern. Pieces are flattened by feature 001 under its seam-length parity rule.
- Turning any imported cut into an opening or an inserted-piece seam, or into a dart other than by FR-011's proposal.
- Automatic seam layout (001 FR-015).
- Reading fabric properties (stretch, pile, thickness) from material settings; only the name is used.

## Assumptions

- Feature 001 provides import, repair, seam drawing, flattenability validation with minimal extra-seam proposals, fabric regions and profiles, the project file, and undo/redo. This feature adds a new source of seams and regions to those rather than reimplementing them.
- Per-corner texture coordinates survive feature 001's import and repair unchanged, so a UV cut is recoverable after welding. 001 FR-002 also promises to keep material assignment, but 001's design does not yet carry material names; this feature's data model adds that.
- Both supported formats record which material each face uses; how each one encodes that is a planning matter.
- A cut that ends inside the surface is far more often a deliberate dart than a slip in round shapes (spike Q1: the usual sphere unwrap makes one at each pole). Proposing it as a dart, visibly and with the maker's acceptance, serves both cases; failing the import would turn away correct models, and applying it silently would hide a slip.
- The texture-coordinate tolerance in FR-001 is documented with the engine's other tolerances and is tuned so exporter rounding never reads as a cut.
- Single maker working locally.
