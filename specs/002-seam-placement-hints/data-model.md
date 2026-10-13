# Data Model — 002-seam-placement-hints

**Feature**: `002-seam-placement-hints` | **Date**: 2026-10-13 | **Status**: Draft

Covers the hint types, the signal/threshold split, identity and re-matching, persistence, and the edge cases each design choice exists to handle. Confidence tags as in [research.md](./research.md).

Reuses 001's structures throughout: `ManifoldMesh`, provenance keys, `Proposal`, `Scale`. Nothing here duplicates them.

## 1. Representations by stage

| Stage | Type | Accepts | Guarantees |
|---|---|---|---|
| Signals | `HintSignals` | `&ManifoldMesh` | Scale-independent per-element measures; cached by mesh content hash |
| Filter | `Vec<Hint>` | signals + `HintOptions` + `Scale` | Monotone in sensitivity; deterministically ordered |
| Decisions | `HintReport` | hints + stored decisions | Every hint carries a category, a location, an explanation, and a decision state |
| Accept | `Proposal` (001) | one hint | Preview-then-accept; produces an ordinary `SeamSet` edit |

**Detection takes `&ManifoldMesh` and offers no mutable path.** FR-010's "hints never alter geometry" is therefore enforced by the borrow checker, not by review.

## 2. Core types

```rust
pub enum HintCategory { SharpEdge, ConcaveCrease, DartCandidate, Symmetry }

pub struct HintId(pub u64);          // hash of (category, provenance key set) — stable, not positional

pub enum HintKind {
    /// A chained polyline of crease edges. Accepting creates a seam along the whole chain.
    Line { edges: Vec<EdgeId>, length_mm: f64, mean_dihedral_deg: f64 },
    /// A clustered region of concentrated curvature. Accepting asks 001 for a dart here.
    Region { faces: Vec<FaceId>, peak: VertexId, integrated_curvature: f64, extent_mm: f64 },
    /// At most one per model (FR-005).
    Plane { normal: DVec3, point: DVec3, match_score: f64 },
}

pub struct Hint {
    pub id: HintId,
    pub category: HintCategory,
    pub kind: HintKind,
    pub strength: f64,               // normalized 0..1, for overlay intensity and sorting
    pub explanation: Explanation,    // never a bare String — see section 5
    pub decision: Decision,
}

pub enum Decision { Open, Accepted { seam_edit: EditId }, Dismissed }

pub struct HintReport {
    pub hints: Vec<Hint>,                        // sorted by (category, id) — deterministic
    pub counts: BTreeMap<HintCategory, usize>,
    pub absent: Vec<HintCategory>,               // categories that ran and found nothing (FR-024)
    pub flooded: Vec<HintCategory>,              // categories over the readability cap (FR-025, SC-010)
    pub orphaned_dismissals: Vec<OrphanedDismissal>,  // FR-041
}
```

### Why `absent` is explicit

FR-024 requires a category that found nothing to say so rather than appearing broken. An empty list is indistinguishable from a category that failed to run, so absence is recorded positively. This matters most on the smooth-sphere case (SC-004), where zero crease hints is the *correct* answer and a silent empty panel would read as a bug.

### Why `flooded` is separate from `counts`

A faceted model can produce hundreds of crease hints. SC-010 caps readability at 50 per category. The cap is reported rather than applied by truncation, because silently dropping hints is the one behaviour that would make the panel untrustworthy — the maker would never know what they were not shown. The UI shows the count and points at the sensitivity control (FR-025).

## 3. Signals and thresholds

```rust
pub struct HintSignals {
    pub dihedral_deg: Vec<f64>,          // per EdgeId; signed: positive convex, negative concave
    pub gauss_curvature: Vec<f64>,       // per VertexId; angle defect
    pub integrated_curvature: Vec<f64>,  // per VertexId; over a geodesic disk of physical radius
    pub symmetry: Vec<SymmetryCandidate>,// scored, best first
    pub mesh_hash: u64,
}

pub struct HintOptions {
    pub categories: BTreeMap<HintCategory, CategorySetting>,
}

pub struct CategorySetting { pub enabled: bool, pub sensitivity: f64 /* 0..1 */ }
```

### The monotonicity contract (FR-006)

Sensitivity maps to a threshold that **decreases monotonically**, and each detector is a pure threshold filter over a precomputed signal. The hint sets are therefore nested: `s1 < s2` implies `hints(s1)` is a subset of `hints(s2)`.

This is the reason H-003 forbids percentile or adaptive thresholds. "Show the top 10%" is not nested — raising sensitivity can reshuffle which features qualify and remove a hint the maker was looking at.

**Two places the guarantee can leak, both handled:**

1. **Chaining.** Merging two crease chains changes their combined length, so a chain that failed the length filter at low sensitivity could fail differently at high. Fix: chain **once** at the most permissive threshold, then filter the resulting chains. Never re-chain per sensitivity level.
2. **Clustering.** Same problem for dart regions. Same fix: cluster once at the most permissive threshold, filter clusters afterward.

Property test (`tests/monotonicity.rs`) asserts the subset relation over random sensitivity pairs. Without it, this guarantee is a comment.

### Physical vs. scale-independent thresholds

| Quantity | Units | Moves with model size? |
|---|---|---|
| Dihedral angle | degrees | No — scale-independent [solid] |
| Minimum chain length | mm | **Yes** |
| Integration radius | mm | **Yes** |
| Minimum region extent | mm | **Yes** |
| Symmetry match score | relative to bbox diagonal | No |

FR-007 requires physical units so the same control means the same thing on a keychain and a large plush. FR-040's "recompute when size changes" exists precisely because three of these five move — if all thresholds were scale-independent, resizing would not change the hints at all.

## 4. Identity and re-matching (FR-008, FR-041)

```rust
pub struct OrphanedDismissal {
    pub category: HintCategory,
    pub stored_keys: Vec<ProvenanceKey>,
    pub dismissed_at_scale_mm: Option<f64>,
}
```

`HintId` hashes the category together with the **sorted set of provenance keys** (001 data-model section 12), never positional indices. `EdgeId` and `FaceId` move when repair settings change; provenance keys do not.

Re-match after any recomputation:

| Case | Behaviour |
|---|---|
| Exact provenance key set matches a stored decision | Same hint; decision carries over silently |
| A new hint shares more than half its keys with a dismissed one | Same hint; dismissal carries over, flagged as an approximate match |
| No overlap with any stored dismissal | The dismissal becomes an `OrphanedDismissal` and is surfaced for re-confirmation (FR-041) |

The overlap rule exists because a small edit near a crease changes a handful of edges without changing the maker's judgement about that crease. Exact-only matching would resurrect dismissed hints constantly; no matching would suppress genuinely new ones. The 50% figure is [likely] and configurable — spike H3 tunes it when there is real editing-session data.

**Accepted hints are not re-matched.** Once accepted, a hint has become an ordinary seam (section 6); the hint no longer exists as a hint, and recomputation will simply not propose a seam where one already runs.

## 5. Explanations (FR-022, FR-065-equivalent)

```rust
pub struct Explanation {
    pub why: &'static str,        // why this was flagged
    pub effect: &'static str,     // what a seam or dart here would accomplish
    pub detail: String,           // the measured specifics, e.g. "valley folds 112 degrees over 46 mm"
}
```

A struct rather than an `Option<String>`, for the same reason 001's `Remedy` is an enum: FR-022 says no hint may be shown without an explanation, and with an optional field "no explanation" stays representable and the requirement becomes a review checklist item. Here it cannot be constructed.

`why` and `effect` are static per category, written once, carefully. `detail` carries the numbers. Example for a concave crease:

> **Why**: This is an inward fold, and stuffing pushes inward folds open.
> **Effect**: A seam along it holds the crease, so the neck stays a neck instead of rounding out.
> **Detail**: Valley folds 112 degrees over 46 mm.

The `effect` text is the teaching payload of the whole feature. It is also where the honest limit of the tool lives: it says what a seam there *would do*, not that the maker *should* put one there.

## 6. Accepting a hint

```rust
pub fn to_proposal(hint: &Hint, mesh: &ManifoldMesh) -> Result<Proposal, HintError>;
```

Converts to 001's existing `Proposal` type. Preview-then-accept, undo, and "nothing changes until confirmed" all come from 001's machinery (Constitution VII), and the result is an ordinary seam with no record of having been a hint — which is what FR-030 requires and what keeps later editing uniform.

| Category | Becomes |
|---|---|
| `SharpEdge`, `ConcaveCrease` | A seam following the entire chain |
| `DartCandidate` | A dart request to 001's dart machinery, which chooses direction and size from the cluster's peak |
| `Symmetry` | Not accepted as a seam. Supplies the plane to 001's FR-010 mirroring mode |

Symmetry is the odd one out: it is a hint the maker *uses* rather than *applies*. The UI reflects that — no accept button, a "use for mirroring" toggle instead.

### When acceptance would break something

FR-036 requires a hint whose acceptance would be unsewable to say so before acceptance, with the choice still the maker's. The check runs 001's existing validation on the proposed `SeamSet`:

- Would the resulting piece fall below minimum sewable size, or below the pile multiple (001 FR-055)?
- Would the dart be too tight to sew?
- Would the chart stop being a disk?

The first two are warnings and acceptance proceeds. The last is 001's typed error — acceptance is refused, and the proposal carries the extra cuts needed, exactly as 001 FR-013 already does.

## 7. Persistence

No schema migration. `project.schema.json` v1 already defines `dismissedHints` and `hintSettings` (001 contract, H-009), so a file written by a 002-aware build opens in a 001-only build and vice versa.

| Stored | Not stored |
|---|---|
| Dismissals, as category plus provenance keys | Hints themselves — always recomputed |
| Per-category enabled flag and sensitivity | Signals — cached in memory only, keyed by mesh hash |
| `dismissedAtScaleMm`, for context when surfacing an orphan | Accepted hints — they are seams now, already in `seams` |

Accepted hints are deliberately absent from hint storage. Recording them would create two sources of truth about one seam, and the second would eventually disagree.

## 8. Edge cases

Each row is an edge case from the spec and the design decision that handles it.

| Edge case | Handling |
|---|---|
| Perfectly smooth model (sphere, egg) | Zero crease hints — reported via `absent`, not an empty panel (SC-004). Dart candidates still appear at tight regions, which is correct |
| Faceted or low-poly model (cube, icosphere) | Minimum chain length filter removes near-coplanar edge noise; whatever survives past 50 per category is reported as `flooded` rather than truncated |
| Dense scanned mesh with surface noise | Integration over a physical-radius geodesic disk, not per-vertex, so curvature describes shape rather than tessellation |
| Near-symmetric model (one ear tilted) | Plane reported with its match score so the maker judges; FR-005 requires the score, not just the plane |
| Several plausible mirror planes, or none | Report none when candidates score within a small margin of each other. Ambiguous beats arbitrary (H-006) |
| Multiple disconnected components | Creases and darts detected per component; symmetry evaluated on the whole model, since a plush is symmetric as an object, not limb by limb |
| Hint on a part already marked as a non-sewn detail | Suppressed. Detail parts produce placement marks, not pieces (001 FR-005, FR-042), so a seam there is meaningless |
| Hint coincides with or runs close to an existing seam | Suppressed when it overlaps an existing seam within a tolerance. A hint telling the maker to do what they already did is noise that makes the panel worth ignoring |
| Two hints cross each other | Both shown. Crossing creases are a real feature of real models (a limb joining a torso), and resolving the crossing is the maker's design decision, not the tool's |
| Dart candidate inside a region already covered by an accepted seam | Recomputed after the seam exists; concentrated curvature relieved by the new seam no longer exceeds threshold, so it disappears on its own |
| Acceptance would produce an unsewable piece | Section 6: warn and proceed, or refuse with proposals if the chart stops being a disk |
| Size change pushes a hint across a threshold | Expected and correct — thresholds are physical. FR-042 reports what changed so the maker understands why the overlay moved |
| Dismissed hint whose region was removed or reshaped | Overlap re-match, then `OrphanedDismissal` surfaced for re-confirmation (section 4) |
| Same model re-imported at different resolution | Provenance keys change entirely, so most dismissals orphan. Honest outcome: a re-import is a new model, and silently carrying decisions across would be the wrong guess |
| Model with unresolved health defects | Detection requires `ManifoldMesh`, which 001's typestate only produces after repair. So this case **cannot occur by construction** — the maker repairs first, exactly as 001 already requires |

The last row is the clearest payoff of 001's typestate gate: "inward fold" is not well defined on a non-manifold surface, and this feature never has to ask whether its input was repaired.

## 9. Determinism

- `hints` sorted by `(category, id)`; `counts` and settings in `BTreeMap`.
- Chaining picks continuations by smallest turning angle, with ties broken by lowest provenance key — never by iteration order.
- Clustering seeds from the lowest provenance key, not from a hash-map traversal.
- Symmetry ICP runs a fixed iteration count with a fixed initial candidate ordering from area-weighted PCA; no randomness anywhere.
- `f64::total_cmp` for every sort over floats, so NaN ordering cannot differ between platforms.

SC-008 asserts byte-identical hint reports across runs and platforms, which is 001's Constitution V applied unchanged.
