# Feature Specification: Seam-Placement Hints

**Feature Branch**: `002-seam-placement-hints`

**Created**: 2026-10-13

**Status**: Draft — clarifications Q1–Q3 closed 2026-10-13

**Input**: User description: "Seam-placement hints: the tool inspects the model and highlights places where a seam, dart, or edge would likely help, so makers (especially first-timers) know where to draw seams instead of guessing. Four toggleable hint categories with sensitivity control: sharp edges and creases, concave creases, dart candidates, symmetry. Hints appear as a 3D overlay and an inspection panel, explain themselves in plain language, convert to a seam or dart in one action with preview, are never applied on their own, and update when the model or its size changes. Out of scope: automatic seam layout, changing geometry, mesh defect detection."

> Rule: this file states **what** and **why** only. Technology choices live in `research.md` and `plan.md`. Data structures live in `data-model.md`.

Builds on feature 001 (`specs/001-pattern-pipeline/`): import, real-world scaling, seam drawing, and flattening already exist. This feature adds guidance about *where* to draw, and resolves the gap identified during 001's clarification — that its warnings tell a maker something is wrong without telling them where a seam belongs in the first place.

## Clarifications

### Session 2026-10-13

- Q: What fraction of hand-placed seam length must a hint cover, and how near counts as "on" a hint? → A: 70% of hand-placed seam length within 5 mm of a hint. Set below full coverage deliberately, because hand patterns include seams driven by fabric economy and sewing convenience that no geometric hint can infer.
- Q: When the model changes so much that a dismissed hint can no longer be matched, what happens to the dismissal? → A: Surfaced for the maker to re-confirm or release — neither kept silently nor discarded silently.
- Q: Must feature 001's flattening be complete before this feature ships? → A: Yes. The flat-pattern preview is the point of FR-031, so this feature depends on 001's flattening stage and ships after it works.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See where seams belong (Priority: P1)

A first-time maker imports a model and, instead of facing a blank surface, sees the model's natural seam lines marked on it: the fold between muzzle and cheek, the crease at the neck, the tight dome at the top of the head. A panel lists each one by category, and clicking an entry moves the view to it. Each one explains, in a sentence, why it's marked and what a seam there would do.

**Why this priority**: This is the feature. Drawing good seams is the hardest skill a new plush maker faces, and the blank model is where people stall. Everything else here acts on hints that this story produces.

**Independent Test**: On a quadruped fixture with a known-good hand-made pattern, turn each category on in turn and confirm hints appear along the muzzle fold, the neck crease, and the head dome, each with an explanation; confirm a smooth sphere fixture produces no sharp-edge or concave-crease hints at default sensitivity.

**Acceptance Scenarios**:

1. **Given** an imported, scaled model, **When** the maker opens the hints view, **Then** hints appear as an overlay on the model and as a list grouped by category with a count per category.
2. **Given** a hint in the list, **When** the maker selects it, **Then** the 3D view moves to show it and the matching overlay element is distinguished from the others.
3. **Given** any hint, **When** the maker inspects it, **Then** it states in plain language why it was flagged and what a seam or dart there would accomplish.
4. **Given** a hint category, **When** the maker turns it off, **Then** its hints disappear from both the overlay and the list and no other category is affected.
5. **Given** a perfectly smooth model with no hard folds, **When** hints are computed at default sensitivity, **Then** the sharp-edge and concave-crease categories report zero hints rather than noise.

---

### User Story 2 - Turn a hint into a seam (Priority: P1)

The maker likes a hint, accepts it in one action, sees what it does to the flat pattern, and keeps or discards it. Nothing is ever applied without that acceptance.

**Why this priority**: A hint the maker has to redraw by hand teaches but doesn't help. The accept-and-preview loop is what turns guidance into a pattern, and it's the half of the feature that saves time.

**Independent Test**: For every hint on the capsule and quadruped fixtures, accept it and confirm a valid seam or dart results, editable afterward exactly like a hand-drawn one, with a preview shown before it is kept.

**Acceptance Scenarios**:

1. **Given** a sharp-edge or concave-crease hint, **When** the maker accepts it, **Then** a seam follows the full flagged line, and it behaves identically to a hand-drawn seam from then on.
2. **Given** a dart-candidate hint, **When** the maker accepts it, **Then** a dart is placed in the flagged region.
3. **Given** an accepted hint, **When** the preview is shown, **Then** the maker sees the effect before committing, and declining leaves the model exactly as it was.
4. **Given** any hint, **When** the maker takes no action, **Then** nothing is added to the model — hints never apply themselves.
5. **Given** an accepted hint, **When** the maker undoes the action, **Then** the seam is removed and the hint returns to its un-acted state.

---

### User Story 3 - Mirror work across a symmetric model (Priority: P2)

The maker works on one side of a symmetric model and gets the other side for free: the tool identifies the left/right mirror plane and offers it as the plane for mirrored seam drawing, so mirror-image pieces get cut twice instead of drawn twice.

**Why this priority**: It roughly halves the drawing work on the most common kind of plush model, but a correct pattern is reachable without it.

**Independent Test**: On a symmetric quadruped fixture the mirror plane is found and matches the model's actual plane; on a deliberately asymmetric fixture the category reports that no plane was found rather than guessing one.

**Acceptance Scenarios**:

1. **Given** a left/right symmetric model, **When** the symmetry category is on, **Then** the mirror plane is shown on the model and offered as the plane for mirrored seam drawing.
2. **Given** a model with no mirror symmetry, **When** the symmetry category is on, **Then** it reports that no plane was found, and does not offer a misleading one.
3. **Given** a near-symmetric model (small asymmetries such as a tilted ear), **When** the plane is detected, **Then** the maker is told how closely the model matches it, so they can decide whether to mirror.

---

### User Story 4 - Dismiss a hint and have it stay dismissed (Priority: P2)

The maker decides a hint is wrong for their design, dismisses it, and never sees it again in that project — but can review and undo dismissals later.

**Why this priority**: Without this, the panel stays cluttered with rejected suggestions and makers stop reading it, which defeats the teaching purpose. Not required for a correct pattern.

**Independent Test**: Dismiss a hint, save and reopen the project, and confirm it is still dismissed; confirm dismissals are listed somewhere the maker can restore them.

**Acceptance Scenarios**:

1. **Given** a hint, **When** the maker dismisses it, **Then** it leaves the active list and does not return on recomputation.
2. **Given** dismissed hints, **When** the project is saved and reopened, **Then** the same hints are still dismissed.
3. **Given** a dismissed hint, **When** the maker reviews dismissals, **Then** they can restore it to the active list.

---

### User Story 5 - Tune how much is flagged (Priority: P3)

A maker working on a highly detailed model turns sensitivity down to see only the strongest seam lines; a maker on a soft, rounded model turns it up to find the subtle ones.

**Why this priority**: Defaults should serve most models. Tuning matters for the extremes — very faceted or very smooth input — but is refinement, not core.

**Independent Test**: On a faceted low-poly fixture, lowering sensitivity reduces the hint count; raising it only ever adds hints, never removes one that was already shown.

**Acceptance Scenarios**:

1. **Given** a hint category, **When** the maker raises its sensitivity, **Then** the hints already shown remain and additional, weaker ones appear.
2. **Given** a hint category, **When** the maker lowers its sensitivity, **Then** only the strongest hints remain.
3. **Given** changed sensitivity, **When** the project is saved and reopened, **Then** the maker's settings are preserved.

---

### Edge Cases

- Perfectly smooth model (sphere, egg): no sharp-edge or concave-crease hints, but dart candidates still expected at tight regions.
- Faceted or low-poly model (cube, icosphere) where nearly every edge reads as a hard fold — a hint flood that would bury the panel.
- Dense scanned or sculpted mesh with surface noise producing many short, spurious creases.
- Near-symmetric model: one ear tilted, a held prop on one side only.
- Model with several plausible mirror planes (a ball, a radially symmetric shape) or none at all.
- Multiple disconnected components: hints per component, and what symmetry means across the whole model versus one limb.
- Hints landing on a small part the maker already marked as a non-sewn detail in feature 001 (eyes, buttons).
- A hint that coincides with, crosses, or runs parallel-and-close-to a seam the maker already drew.
- Two hints that cross each other, or a dart candidate inside a region already covered by an accepted seam.
- Accepting a hint that would produce a piece below the minimum sewable size, or a dart too tight to sew.
- Real-world size changed just enough to push a hint across a sensitivity threshold, making it appear or disappear.
- A dismissed hint whose surface region is later removed, re-seamed, or substantially reshaped.
- The same model re-imported at a different mesh resolution, where hint positions shift slightly.
- Model with unresolved health defects from feature 001's import checks (non-manifold edges, holes) where "inward fold" is not well defined.
- A model whose longest dimension is near the handheld-to-large-plush bounds, where physical thresholds behave differently at each extreme.

## Requirements *(mandatory)*

### Functional Requirements

#### Detection

- **FR-001**: Four independent hint categories, each separately toggleable and separately tunable: sharp edges and creases, concave creases, dart candidates, and symmetry.
- **FR-002**: Sharp-edge hints mark hard folds in the surface — places where two surfaces meet at an angle rather than curving smoothly — as connected lines rather than scattered points, each carrying a strength value so the strongest can be told from the weakest.
- **FR-003**: Concave crease hints mark inward folds specifically, distinguished from outward ones, because stuffing pushes an inward fold open and loses the shape unless a seam holds it.
- **FR-004**: Dart-candidate hints mark regions curved too tightly in every direction for one flat piece to cover without puckering.
- **FR-005**: Symmetry detection reports at most one dominant left/right mirror plane, together with how closely the model matches it; when no plane fits, it reports none rather than offering the best bad guess.
- **FR-006**: Each category has a sensitivity control with a documented default. Raising sensitivity only ever adds hints — it never removes one that was already shown.
- **FR-007**: Sensitivity thresholds are expressed as physical quantities (millimetres and angles), not as fractions of the model, so the same control means the same thing on a keychain and on a large plush.
- **FR-008**: Every hint is locatable on the model and carries an identity stable enough to be dismissed, restored, and re-matched after the model changes.
- **FR-009**: The same model at the same size with the same settings produces an identical set of hints in an identical order, on every platform and every run.
- **FR-010**: Hints never alter the model's geometry. This feature only reads the surface.

#### Presentation

- **FR-020**: Hints appear as an overlay on the 3D model, with each category visually distinguishable, using palettes that remain distinguishable for color-blind makers.
- **FR-021**: An inspection panel lists hints grouped by category with a count per category; selecting an entry moves the 3D view to that hint and highlights it.
- **FR-022**: Every hint states in plain language why it was flagged and what a seam or dart there would accomplish. No hint is shown without an explanation.
- **FR-023**: The panel is fully keyboard-operable, including navigating between hints, accepting, and dismissing.
- **FR-024**: When a category produces no hints, it says so explicitly rather than appearing broken or empty.
- **FR-025**: When a category produces more than 50 hints (as on a faceted model), the maker is told why and pointed at the sensitivity control, rather than being handed an unusable list.

#### Acting on hints

- **FR-030**: Any hint converts to a seam or a dart in one action. The result follows the whole flagged feature — the full crease line, not a starting point — and is thereafter editable exactly like a hand-drawn seam.
- **FR-031**: Accepting a hint shows its effect before it is kept; declining the preview leaves the model unchanged.
- **FR-032**: Hints are never applied automatically. They remain proposals until the maker accepts them.
- **FR-033**: Any hint can be dismissed. Dismissed hints do not reappear on recomputation, and dismissals are reviewable and reversible.
- **FR-034**: Accepted and dismissed decisions, along with per-category toggles and sensitivity settings, are saved with the project and restored on reopen.
- **FR-035**: Accepting, declining, and dismissing are all undoable.
- **FR-036**: A hint whose acceptance would produce an unsewable result (a piece below minimum sewable size, a dart too tight to sew) says so before acceptance, and acceptance is still the maker's call.

#### Lifecycle

- **FR-040**: Hints recompute when the model or its real-world size changes. Seams already accepted are unaffected — they are now ordinary seams, not hints.
- **FR-041**: After a recomputation, dismissals are re-matched to the hints they referred to. A dismissal that can no longer be matched is surfaced for the maker to re-confirm or release — it is neither kept silently nor discarded silently.
- **FR-042**: Recomputation reports what changed since the previous set — hints added, hints no longer present — so a maker who adjusts the size understands why the overlay moved.

#### Platform

- **FR-050**: Hint detection and its full report are available headless, without the 3D view, so hints can be generated and checked automatically.
- **FR-051**: All detection runs on the maker's device. No model data leaves the machine and no network request is made.

### Key Entities *(include if feature involves data)*

- **Hint** — one suggested seam line, dart region, or plane, with its category, location on the surface, strength, plain-language explanation, and current decision state.
- **Hint Category** — one of the four kinds of hint, with its on/off state and sensitivity setting.
- **Symmetry Plane** — the detected left/right mirror plane and how closely the model matches it.
- **Hint Decision** — a maker's accept or dismiss, attached to a hint durably enough to survive recomputation and reopening.
- **Hint Report** — the complete set of hints for a model at a given size and settings, suitable for review in the panel or headless inspection.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On reference models that have known-good hand-made patterns, at least 70% of hand-placed seam length falls within 5 mm of a hint. Seams the pattern's author placed for reasons no geometric hint can infer — fabric economy, sewing convenience — count toward the total, which is why the target sits below full coverage.
- **SC-002**: In testing with first-time makers (n ≥ 5), makers place their first seam faster with hints on than with hints off, by at least 30% median time.
- **SC-003**: Hints for a 50,000-triangle model appear within 5 seconds on a 2020-era mid-range laptop, and changing a sensitivity setting updates them within 1 second.
- **SC-004**: On a smooth sphere fixture at default sensitivity, the sharp-edge and concave-crease categories report zero hints — no noise on input with no features.
- **SC-005**: Symmetry detection is correct on every fixture in the reference corpus: the plane is found on symmetric models, and reported absent on the asymmetric ones, with no false planes.
- **SC-006**: Accepting any hint produces a valid, editable seam or dart 100% of the time across the fixture corpus — never a malformed or self-crossing path.
- **SC-007**: Every hint presented carries an explanation; a hint without one is a defect, measured at zero occurrences.
- **SC-008**: The same model, size, and settings produce a byte-identical hint report across repeated runs and across platforms.
- **SC-009**: Zero network requests are made while hints are computed or displayed.
- **SC-010**: On a faceted low-poly fixture (cube, icosphere) at default sensitivity, no category produces more than 50 hints. Above that count the maker is told why and pointed at the sensitivity control (FR-025) rather than handed the full list.

## Out of scope

Named here so they are not assumed to be part of this feature:

- Generating a complete seam layout automatically — hints are the groundwork for that, not that feature. It remains deferred (feature 001, FR-015).
- Changing the model's geometry in any way, including smoothing, remeshing, or repair.
- Detecting mesh defects — non-manifold edges, holes, flipped normals, self-intersections — which feature 001's import health checks already cover.
- Suggesting which fabric to use, or hinting based on fabric properties such as pile length or stretch.
- Hints about stuffing openings, assembly order, or where to place markings.
- Learned or trained models for detection; a deterministic geometric baseline is what this feature delivers.

## Assumptions

- Feature 001 provides import, real-world scaling, seam and dart drawing, the project file, and undo/redo. This feature adds to those rather than reimplementing them.
- Hints attach to the model surface the same way feature 001's markings do, so a hint survives view changes and refers to a place on the model rather than a place on screen.
- The symmetry plane this feature detects is the plane that feature 001's optional symmetry-mirroring mode uses; the two are the same plane, discovered here instead of assumed.
- The flat-pattern preview that FR-031 shows comes from feature 001's flattening, which makes that a hard dependency: this feature ships after 001's flattening stage works, rather than falling back to a 3D-only preview. Seeing the pattern consequence is the part that teaches.
- Default sensitivities are tuned against feature 001's fixture corpus so that a typical plush model yields useful hints without adjustment.
- Hints are advisory throughout. They produce no warnings in feature 001's sense and therefore never gate export.
- Single maker working locally; no sharing, collaboration, or multi-user review of hints.
- Model size stays within feature 001's supported range — handheld to large plush — which is what makes physical sensitivity thresholds (FR-007) meaningful.
- Substantial model changes are infrequent enough that FR-041's re-confirmation prompt is an acceptable interruption rather than a recurring nuisance.
