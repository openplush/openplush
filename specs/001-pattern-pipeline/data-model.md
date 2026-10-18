# Data Model & Critical Edge Cases

**Feature**: `001-pattern-pipeline` | **Date**: 2026-10-12 | **Last updated**: 2026-10-13 | **Status**: Active — §6a (fabric profiles) and §14 (warnings) added after spec Q1–Q3 closed

Covers: representations per stage, the half-edge structure, non-manifold handling, chart topology, seam optimization, grainline/nap mapping, fabric profiles, allowance, notches, decals, persistence, warnings. Confidence tags as in [research.md](./research.md).

## 1. Representations by Stage

| Stage | Type | Accepts | Guarantees |
|---|---|---|---|
| Import | `TriangleSoup` | Anything parseable | Finite coordinates: a NaN/∞ anywhere is a located import error, never a dropped face (§3 step 0, D-021). Triangles only: polygons are triangulated at import. Provenance per corner |
| Validate | `ValidationReport` | `TriangleSoup` | Every defect located (face/edge/vertex IDs) |
| Repair | `ManifoldMesh` + `RepairReport` | `TriangleSoup` + `RepairPlan` | Constitution III post-repair invariants |
| Seams | `SeamSet` | `ManifoldMesh` edges | Typed edge flags; no geometry change |
| Chart | `ChartMesh` ×N + `SewingMap` | mesh + seams | Each chart a topological disk, or a typed error with proposed cuts |
| Flatten | `Parameterization` | `ChartMesh` | det J > 0 on every face; boundary edge lengths equal their 3D lengths (D-013) |
| Pattern | `PatternPiece` ×N | param + fabric + scale | Simple polygons in mm; seam parity within max(1.0 mm, 0.5%) — hard (D-013) |
| Layout | `Sheet` ×N | pieces + layout options | No overlaps; rotation sets honored |

**Typestate**: `ManifoldMesh` has no public constructor. The only way to get one is `repair::into_manifold(soup, &plan) -> Result<(ManifoldMesh, RepairReport), RepairError>`. Downstream stages take `&ManifoldMesh`, so unrepaired data cannot reach the flattener by construction.

## 2. Half-Edge Structure

Index-based arenas with newtype IDs — no pointers, no `Rc<RefCell<…>>`. Borrow-checker friendly, cache friendly, trivially serializable, and transferable to the worker as flat buffers.

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HalfEdgeId(u32);
pub struct EdgeId(u32);   // = HalfEdgeId >> 1
pub struct VertexId(u32);
pub struct FaceId(u32);

impl HalfEdgeId {
    #[inline] pub fn twin(self) -> Self { Self(self.0 ^ 1) }   // twins allocated as (2k, 2k+1)
    #[inline] pub fn edge(self) -> EdgeId { EdgeId(self.0 >> 1) }
}

/// Struct-of-arrays layout.
pub struct ManifoldMesh {
    // per vertex
    pos: Vec<DVec3>,
    v_out: Vec<HalfEdgeId>,          // boundary vertices: MUST point to the outgoing boundary half-edge
    // per half-edge
    he_next: Vec<HalfEdgeId>,
    he_prev: Vec<HalfEdgeId>,
    he_origin: Vec<VertexId>,
    he_face: Vec<Option<FaceId>>,    // None ⇒ boundary half-edge
    // per face
    f_he: Vec<HalfEdgeId>,
    // per corner (= per interior half-edge): source attributes
    corner_uv: Vec<Option<DVec2>>,   // original texture coords, preserved through repair
    corner_src: Vec<SourceCorner>,   // provenance: (source primitive, source vertex index)
                                     // OBJ primitive: a run of `f` lines between `o`/`g`/`usemtl` lines (D-034)
}
```

**Conventions**
- **Explicit boundary half-edges** (`face = None`): `twin` is always valid, boundary loops are traversable via `next`, no special-casing in iterators.
- **Paired-index twins** remove the `twin` array and give `EdgeId` for free.
- **Boundary vertex rule**: `v_out[v]` is the boundary half-edge if one exists → O(1) `is_boundary(v)` and a deterministic start for one-ring iteration.
- **Per-corner attributes on half-edges**, not vertices. A vertex on a UV seam in the source model has several UVs; one per corner is the only lossless representation.
- `prev` is stored (not recomputed) because cutting and boundary traversal hit it constantly; it costs 4 bytes per half-edge.

**Invariants** (checked by `debug_validate()` in tests and debug builds)
1. `next(prev(h)) == h`, `twin(twin(h)) == h`, `origin(next(h)) == origin(twin(h))`.
2. Interior face loops have length 3.
3. Each vertex's one-ring is a single fan (disk or half-disk) — the manifold condition.
4. Adjacent faces have opposite half-edge directions on their shared edge — consistent orientation.
5. No face has zero area below `ε_area` (see §11).

## 3. Non-Manifold Handling (repair pipeline)

Order matters; each step records entries in `RepairReport`.

| # | Step | Detects | Resolution | Seam consequence |
|---|---|---|---|---|
| 0 | Normalize (**at import**, not in repair; D-021) | n-gons, NaN/∞ | Quads split on shorter diagonal; n-gons ear-clipped on best-fit plane (lossless: no position changes). A NaN/∞ coordinate is **not** repaired: import fails with `EngineError::NonFiniteInput`, naming every non-finite source vertex by provenance, so the maker can fix the file. Dropping faces would be fixed-up geometry without confirmation (Constitution III, FR-004). Repair starts at step 1 | — |
| 1 | **Weld** | Coincident vertices | Merge within `ε_weld = 1e-6 × bbox diagonal` via sorted spatial hash (deterministic tie-break by source index). Corner UVs untouched | **Critical for glTF**: exporters split vertices at UV/normal discontinuities; unwelded, the model is topologically shredded into UV islands [solid] |
| 2 | Degenerates & duplicates | Zero-area faces, repeated indices, duplicate faces | Drop repeated-index faces (needles, already collapsed by the weld). A zero-area cap (area within `ε_area`, §11) puts its middle vertex on its longest edge; each face given such vertices is re-triangulated with them, and the caps are dropped, so no crack opens and no vertex moves (D-036). Keep the lowest-ID copy of same-orientation duplicates; keep both opposite-orientation duplicates and flag them "double-sided sheet"; step 3 pairs a sheet's interior edges within each layer (D-036) | — |
| 3 | **Non-manifold edges** (> 2 incident faces) | Edge fans | Sort incident faces by dihedral angle around the edge axis; pair angular neighbors, preferring opposite traversal; the pairings are returned as data and the half-edge build gives each pair its own edge copy (Guéziec et al. 2001, D-037) | New boundary edges become **mandatory seams**. Often *intentional*: an ear or fin sandwiched into a body seam → see §4 inserted pieces |
| 4 | **Non-manifold vertices** (bowtie) | Vertex whose faces form > 1 fan | Split into one vertex per fan | Usually separates components (e.g., two cones touching at a tip); reported, no seam |
| 5 | Orientation | Inconsistent winding | BFS per component from the largest face, flipping to agree; component orientation chosen so signed volume > 0 when closed | Non-orientable (Möbius-like) conflict edge ⇒ forced seam. Only edges with exactly two faces are walked; faces joined across a paired copy (D-037) are judged by the half-edge build, which refuses a same-way pair |
| 6 | Components | Multiple connected components | Each processed independently. Components with area < 1% of total (or user choice) ⇒ candidate **detail parts** (eyes, noses, buttons) | Detail parts produce placement marks, not pieces (FR-005, FR-042) |
| 7 | Holes | Boundary loops | Classify: user marks as *opening* (keep; hem allowance) or *accidental* (fill: Liepa 2003 minimal-area triangulation + fairing) | Openings are unsewn edges, not seams |
| 8 | Self-intersections | BVH triangle-triangle tests | **Warn only.** Disk charts flatten regardless; intersecting limbs in 3D usually need separate pieces anyway | Suggest seams along intersection curves |
| — | Last resort | Hopeless input | Not in v0.1 (D-024: `baby_shark` fails the dependency gates). If it returns: opt-in voxel remesh, which destroys UVs and sharp features → explicit warning | All seams/decals must be redrawn |

### 3a. RepairPlan and RepairReport (D-039)

`RepairPlan` is what the maker confirmed; steps 0-6 are lossless or mandatory and need no choice, so the plan carries only the two decisions the maker owns. Both are keyed by an ID that is stable across re-application (`RepairPlan` is stored and re-applied, §12).

```rust
pub struct RepairPlan {
    /// A boundary loop, named by its lowest vertex ID in the repaired mesh's pre-fill numbering,
    /// → what to do with it. A loop not listed stays an opening: filling is opt-in.
    pub holes: BTreeMap<VertexId, HoleDisposition>,   // Fill | Opening
    /// A component, named by its lowest face ID, → whether it is a detail part (FR-005).
    /// A component not listed follows the 1%-of-area default (step 6).
    pub components: BTreeMap<FaceId, ComponentRole>,  // Detail | Body
}
```

`RepairReport` lists everything repair did or found, so nothing is silent (Constitution VII, FR-004): the welded vertex groups; faces dropped as repeated-index or duplicate; the double-sided sheets; stitched caps and the faces re-triangulated with them; every paired non-manifold edge (pairs and unpaired face); every added vertex copy (bowtie); every flipped face; every orientation-conflict edge; every component with its detail flag and the role that decided it; every boundary loop with its disposition and the faces a fill added; and the mandatory seams the above create. Removals, flips and pairings name faces and vertices of the soup given to repair; additions (vertex copies, hole faces) name those of the repaired mesh.

A loop's fill is never silent: it appears in the report whether the plan named it or not, and a `Fill` of a loop that no longer exists (the mesh changed) is a typed error naming the loop, not a skipped entry.

## 4. Seams, Charts, and Sewing Correspondence

### SeamSet (non-destructive)
```rust
pub enum SeamKind { Cut, Dart, Opening, FabricBoundary, Mandatory /* from repair */ }
pub struct SeamSet { kind: BTreeMap<EdgeId, SeamKind> }
```
The mesh never changes when seams change. Undo/redo is a `SeamSet` diff.

### Seam graph nodes
Seam edges form a graph on mesh vertices. Node classes:
- **Junction**: seam-valence ≥ 3 (three or more seam edges meet).
- **Endpoint on boundary**: seam meets a hole/opening.
- **Free endpoint** (seam-valence 1, interior): **dart tip**.
- **Chain vertex**: seam-valence 2.

A **seam chain** is a maximal path between non-chain nodes. Chains are the unit of notching, labeling ("A3"), and length parity.

### ChartMesh (virtual cut)
Charts = flood fill of faces across non-seam edges. For each chart, build a local half-edge mesh by **duplicating vertices along seam edges inside the chart** — except dart tips, which stay single (a slit, not a separation). Provenance maps:
```rust
pub struct ChartMesh {
    mesh: LocalHalfEdgeMesh,
    vtx_to_global: Vec<VertexId>,
    he_to_global: Vec<HalfEdgeId>,
}
```

### SewingMap — generalized to N sides
Each global seam edge has `k ≥ 2` sides. Standard seams: `k = 2`. **Inserted pieces** (ear sandwiched into a head seam, from step 3 non-manifold edges or user construction): `k = 4` (head-left, ear-front, ear-back, head-right).
```rust
pub struct SeamBundle {
    chain: SeamChainId,
    sides: SmallVec<[ChainSide; 2]>,   // (chart, ordered boundary half-edges, direction)
}
```
MVP UI exposes `k = 2`; the model supports `k > 2` from day one so FR-016 isn't a migration later.

### Chart topology check (pre-flatten)
χ = V − E + F; for a connected surface with genus g and b boundary loops, χ = 2 − 2g − b. Flattenable ⇔ **g = 0, b = 1** (disk, χ = 1). Darts don't change this — a slit extends the single boundary loop.

| Failure | Cause | Auto-proposal (requires user approval — Constitution VII) |
|---|---|---|
| b = 0, g = 0 | Closed sphere-like region, no seam | One cut along the max-distortion geodesic, or a dart pair at highest angle defect |
| b > 1 | Annulus (e.g., a sleeve with both ends cut) | Shortest path connecting boundary loops (Dijkstra; edge cost = length × visibility × concavity weight) |
| g > 0 | Handle (e.g., a mug handle, arm touching body) | Tree-cotree generators shortened greedily (Eppstein 2003; Erickson & Whittlesey 2005) |

## 5. Seam Optimization

Two different problems — don't conflate them:
1. **Improve given seams** (MVP, FR-022): heatmap + suggestions.
2. **Generate seams from scratch** (FR-015) — v0.2, not v0.1 (D-015). Feature 002's hints are the intermediate step: they surface the same geometric signals to the maker before any code tries to chain them into a full layout.

### Objective (shared)
`E = w_d·D + w_l·L + w_v·V + w_c·C + w_s·S + w_n·N`

| Term | Meaning | Note |
|---|---|---|
| D | Distortion vs. fabric tolerance (Σ area-weighted excess σ) | Fabric-relative, not absolute |
| L | Total seam length | Sewing effort ∝ seam length (Pillow, 2008) |
| V | Visibility | Prefer underside, behind ears, inside limbs (Seamster) |
| C | **Unsupported concavity** | Stuffing pushes fabric outward; a concave crease (neck, between legs) holds only if a seam runs along it. Seams *reward* concave creases |
| S | Symmetry mismatch | Mirror seams across the detected/declared symmetry plane |
| N | Piece count | Fewer pieces = less assembly |

**Hard constraints**: every chart a disk; min piece size (provisional: 20 mm in each principal direction at target scale); min seam curvature radius ≥ seam allowance (else machine sewing fails); dart length ≤ fraction of chart diameter.

### MVP: dart suggestion
1. Find faces whose σ exceeds fabric tolerance; cluster.
2. Within each cluster, find the vertex of max angle defect (Gaussian curvature) — a cone point.
3. Propose a dart from the nearest chart boundary along the shortest path toward that cone point.
4. Re-flatten only that chart; accept the suggestion for display if D drops by ≥ threshold.

### v0.2: generation
- Seed with **D-Charts** (quasi-developable segmentation) — plush panels are near-developable patches, so this matches the domain.
- Refine with **OptCuts**-style alternation: flatten ↔ topology moves (extend/retract cuts, merge), accept moves that lower E.
- Symmetrize: detect bilateral plane (PCA → reflection-ICP refinement) or take user's; mirror seams; dedupe mirrored pieces (FR-034).

## 6. Grainline and Nap Mapping

Fabric is a rigid plane: within one piece, grain and nap are **constant in 2D**. The job is to choose each piece's rotation so that the constant fabric direction best matches a direction field on the 3D surface.

### Direction field on the surface
- **Nap (pile with head/tail)**: a **vector field** (1-RoSy). 180° matters.
- **Grain without nap**: a **line field** (2-RoSy). 180° equivalent.
- Input: user strokes ("fur flows this way") as sparse face constraints.
- Solve: smoothest field with soft alignment to constraints — Knöppel et al. 2013; a sparse linear system in complex form, same solver path as flattening.
- Default with no strokes: project a global "down/back" vector onto each tangent plane. Singular where normal ∥ vector (top of head) — interpolate across.

### Per-piece grain angle
For each face f in the chart, express the 3D direction in the face's tangent basis, push forward through the face Jacobian: `d₂(f) = normalize(J_f · d₃(f))`, angle θ_f.

- **Vector (nap)**: θ* = atan2(Σ a_f sin θ_f, Σ a_f cos θ_f).
- **Line (grain)**: average doubled angles 2θ_f, then halve.
- **Coherence** R = |Σ a_f e^{iθ_f}| / Σ a_f ∈ [0, 1] (use 2θ for line fields). Low R ⇒ nap direction varies visibly across the piece ⇒ warn and suggest a split along the field's high-gradient curve (FR-052). Provisional thresholds: warn R < 0.85, suggest split R < 0.7.
- Rotate the piece by −θ* so the fabric's warp/nap axis maps to pattern +Y (printed arrows).

### Cross-seam continuity
At every seam bundle, compare the mapped nap directions on each side at matched arc-length samples. A large jump means fur visibly changes direction across the seam → report.

### Constraints passed to layout
| Fabric | Allowed rotations | Mirrored copies |
|---|---|---|
| Napped (faux fur, velvet, minky with pile) | {0°} (± small tolerance, provisional 3°) | Reflect across the axis **parallel** to the nap — preserves nap direction |
| Non-napped, no directional print | {0°, 180°} | Either axis |
| Bias-cut woven | {45°, 225°} | Parallel to grain |

`jagua-rs` supports reflection about specified local axes, which maps directly to mirrored pieces (S4 confirms discrete rotation sets).

### Anisotropic stretch — out of scope (D-014)
Were it in scope: once θ* is fixed, the fabric's stretch tensor `M` is known in the piece frame → anisotropic re-flatten (research R3). Example: knit-backed faux fur stretches mostly crosswise, so the solver could place compression along that axis but not along the nap. FR-026 excludes this from 001; per-direction stretch feeds warnings only. Noted so the extension point is obvious and nobody designs it out.

## 6a. Fabric Profiles (FR-050, D-018)

```rust
pub enum FabricCategory { Woven, Knit, Pile, Felt }

pub struct Fabric {
    pub id: FabricId,
    pub name: String,
    pub category: FabricCategory,
    /// Percent elongation under normal hand tension. Recorded; drives warnings only (D-014).
    pub stretch_grain: f64,
    pub stretch_cross: f64,
    pub stretch_bias: f64,
    pub nap: bool,
    pub pile_len_mm: f64,
    pub thickness_mm: f64,
    pub frays: bool,
    pub default_allowance_mm: f64,
    pub roll_width_mm: f64,
}
```

- **Presets** (research R7) ship as data, not code: seven `Fabric` values the user can edit or delete. An edited preset is just a `Fabric` in the project; there is no "preset" type, so a reopened project never depends on the app's preset table having stayed the same. This is what makes FR-050's "profiles are saved in the project" true rather than aspirational.
- **Derived, never stored**: allowed rotation set (from `category` + `nap`, §6 table), whether mirrored printing applies (FR-054: `category == Pile`), and the pile-scaled default allowance (FR-055). Storing a derived value is how a project file ends up self-contradictory after the rules change.
- **Invariants**: `pile_len_mm > 0 ⇒ nap` for `Pile`; all stretch percentages ≥ 0; `roll_width_mm > 0`; `default_allowance_mm ≥ 0` (zero is legal only when `!frays`, FR-057).
- **Per-piece assignment** comes from painted fabric regions (FR-041). A piece spanning two fabrics is impossible by construction, because region boundaries are seams.

### Fabric-driven warnings

| Warning | Condition | Remedy kind |
|---|---|---|
| `PieceSmallerThanPile` (FR-055) | piece or detail extent < `k × pile_len_mm`, provisional `k = 3` | `Advisory` — shorter-pile fabric, or scale the toy up |
| `CurveTighterThanPile` (FR-055) | seam-curve radius < `k × pile_len_mm` | `Advisory` — same |
| `StretchMismatch` (FR-058) | \|stretch of fabric A − fabric B\| > threshold on a shared seam, provisional 20 percentage points, compared per-direction after each piece's grain angle is known | `Advisory` — change one fabric, or accept and stabilize the seam |
| `LowNapCoherence` (FR-052) | R < 0.85 (warn), R < 0.7 (suggest split) | `Actionable` — proposed split curve |
| `UnsupportedConcavity` (FR-025) | concave crease with no seam along it | `Actionable` — proposed seam along the crease |

`StretchMismatch` is the one that needs grain angle first, so it is evaluated at the pattern stage, after `orient` — not at fabric-assignment time. A seam between two pieces of the *same* knit cut at 90° to each other mismatches just as badly as two different fabrics, and only the post-orient comparison catches that.

## 7. Seam Allowance

- **Per-edge allowance** (seam default, per-seam override, opening allowance, hem for unsewn edges) breaks single-distance polygon offset. Construct instead: union of the sewing polygon with one strip per boundary segment (segment ⊕ rectangle of that segment's allowance, plus round/miter caps) via `i_overlay`. Result is the cut line.
- **Pile scaling** (FR-055): the effective default allowance is `max(fabric.default_allowance_mm, c × fabric.pile_len_mm)`. Long faux fur needs a wider allowance because the pile itself occupies the seam. Per-seam overrides still win — this only moves the default.
- **Zero allowance** (FR-057): legal only when `!fabric.frays`, and only on edges the user marked edge-stitched or appliqué. The cut line then coincides with the sewing line, which means the piece has no offset polygon at all — a case the offset code must handle rather than divide by zero on. Pieces with zero allowance are excluded from clip/notch-out marking (§7 tight curves), since there is no allowance to clip into.
- **Corners**: MVP uses miter joins with a miter limit (prevents spikes at acute corners). v0.2: drafting-standard **reflected corners** — allowance ends shaped as the mirror of the adjacent seam's allowance so edges align when pressed [likely].
- **Tight curves**: where sewing-line curvature radius < allowance, the offset self-intersects; the union resolves it, and the piece gets **clip marks** (concave) or **notch-out marks** (convex) at a spacing proportional to curvature (FR-035).
- **Robustness**: simplify input polygons first; catch `i_overlay` panics (known issue #74) at the stage boundary and convert to a typed error with the offending piece ID; fuzz in CI (S3). *Superseded 2026-10-15 by [ADR 0009](../../docs/adr/0009-wasm-panic-is-a-worker-crash.md): a panic cannot be caught on wasm32, so it is a crash on both targets and nothing catches it.*

## 8. Notches and Labels

- Parametrize each chain side by arc length. Notches at both chain ends (junctions) plus every `Δ` mm (provisional 50 mm) along the chain.
- Place side B's notches at the **same fractional arc length** as side A. If lengths differ within tolerance, label the chain "ease" and distribute proportionally.
- **Pairing codes** must be unambiguous: chain label (A3) printed at the midpoint on both sides; notch multiplicity (single/double/triple) differs between adjacent chains on the same piece, so a sewist can't rotate a piece and match the wrong edge.
- Piece label block: ID, name, cut quantity, mirror flag, fabric, grain arrow, nap arrow, scale reference.

## 9. Markings, Detail Parts, Texture

- **Surface-attached storage**: markings are stored as barycentric coordinates on faces of the `ManifoldMesh` — not in pattern space. They survive re-seaming and re-flattening, and map through each chart's provenance.
- **Crossing seams**: a marking is clipped per chart; matching alignment ticks are added on both sides of the seam.
- **Detail parts**: component centroid and contact footprint are closest-point projected onto the main surface → placement mark (center cross + outline) on the piece beneath. Safety-eye holes get a distinct mark.
- **Texture transfer (v0.2)**: per piece, rasterize triangles in pattern space at print DPI, sampling the source texture with the preserved corner UVs. Bleed into the allowance by dilation so print-on-fabric has no white edge. Embedded in SVG as an `<image>` under a `<clipPath>` of the cut line.

## 10. Units and Scale
- `ImportOptions.units` declared or inferred (heuristic on bbox size) and confirmed by the user.
- `Scale` is a single uniform factor applied at the pattern stage. Flattening output is in model units × scale; physical constraints (allowance, min piece size, min curvature radius) are evaluated in mm after scaling.
- **Supported range** (FR-006): keychain to roughly 1 m in the longest finished dimension. Outside it, a warning — the physical thresholds (min sewable size, pile multiples, notch spacing) were chosen for that range and stop being meaningful beyond it.
- **Shrink compensation** (FR-056, D-012): an optional uniform percentage `shrink_pct`, applied as a second uniform factor at the pattern stage, so a stretch fabric sewn and stuffed lands nearer the model's size. It is **uniform only** — one number, every direction, every piece. Direction-dependent compensation is out of scope (D-014), and so is any attempt to predict the stuffed shape (D-012). The maker sets this from experience with their fabric; the tool does not infer it.
- Because both `Scale` and `shrink_pct` are uniform factors at the same stage, flattening never re-runs when either changes — only `pattern` onward. That is the practical payoff of keeping compensation uniform.

## 11. Determinism and Tolerances
- Arenas iterate in index order; maps are `BTreeMap`; sorts are stable with total-order tie-breaks.
- All epsilons are **relative to bbox diagonal** unless physical: `ε_weld = 1e-6`, `ε_area = 1e-12` (relative², area), flip threshold det J > 1e-10 (relative). Physical tolerances (seam parity, allowance) are absolute mm.
- Solver: fixed ordering for the symbolic factorization; fixed iteration caps and convergence tests.

## 12. Persistence (project file)
- JSON document + embedded binary mesh (glb-style chunk). `schemaVersion` with forward migrations in `openplush-project`.
- **Stable references**: seams and markings reference edges/faces by keys derived from **source provenance** (sorted pair of canonical source-vertex IDs; face = sorted source-corner triple) — not by `EdgeId`, which changes if repair settings change.
- Stored: source mesh, `RepairPlan` (re-applied deterministically), `SeamSet`, fabrics and region assignments, direction strokes, markings, scale, options.
- Not stored: anything recomputable (charts, parameterizations, pieces). Optional cache block keyed by content hash for fast reopen.

## 13. Warnings and Remedies (FR-064, FR-065 — D-016, D-017)

```rust
pub struct WarningSet { items: Vec<Warning> }   // total order, see §13a "Warning codes and order"

pub struct Warning {
    pub code: WarningCode,          // stable, documented, user-visible
    pub target: WarningTarget,      // Face/Edge/Chain/Piece/Component/Project
    pub explanation: String,        // plain language: why it was flagged (FR-022, FR-065)
    pub remedy: Remedy,             // never absent — that is the point
    pub acknowledged: bool,         // persisted; FR-064's "acknowledged once"
}

pub enum Remedy {
    /// The engine can perform it. UI offers preview-then-accept (Constitution VII).
    Actionable { proposal: Proposal },
    /// The engine will not perform it; it names the material or sizing change.
    Advisory { what_to_change: String },
}
```

**Why `Remedy` is an enum and not an `Option<Proposal>`**: FR-065 says no warning may describe only the problem. With `Option`, "no proposal" is representable and the requirement becomes a review checklist item. With this enum, a warning that explains nothing cannot be constructed — the compiler enforces the spec. `Advisory` carries real text precisely so "we couldn't think of a fix" is not expressible either.

**Errors are the other channel entirely.** `WarningSet` rides along with `Ok(...)`; a violated invariant is `Err(EngineError)` and produces no output (plan "Warnings vs. errors"). The seam-parity tolerance is an error now (D-013), not a warning — which is the single most important consequence of Q2 for this file.

**Persistence**: acknowledged warnings and their codes+targets are stored in the project (§12), keyed by **source provenance** like seams are, so an acknowledgement survives re-repair and reopening. Unacknowledged warnings are recomputed, never stored.

**Export** (FR-064): `export` takes the `WarningSet`. Unresolved entries render as a summary block on the pattern's first page — code, location, explanation, remedy. Export is never refused. The summary is part of the golden snapshot, so a warning silently vanishing from the printed page is a test failure.

## 13a. Proposals, Locations, and Warning Codes (D-022)

The error and warning channels (§13, engine-api §5) refer to a proposal type and to IDs that no stage had defined. They are defined here so T028–T029 implement a decision instead of making one.

### Element IDs and who owns them

Every ID is owned by the crate that creates it. A crate can only name types from itself or crates below it (plan's crate DAG), so this table follows the DAG.

| ID | Names | Owned by | Stable across |
|---|---|---|---|
| `VertexId`, `EdgeId`, `HalfEdgeId`, `FaceId` | `ManifoldMesh` elements (§2) | `openplush-mesh` | One repaired mesh. Persisted state never stores them; it stores provenance (§12) |
| `ComponentId` | A connected component after repair, numbered by its lowest `FaceId` | `openplush-mesh`, created by repair step 6 (T065) | One repaired mesh |
| `ChartId` | A chart, numbered by its lowest `FaceId` | `openplush-seams` | One `SeamSet` |
| `SeamChainId` | A seam chain (§4), numbered by its lowest `EdgeId` | `openplush-seams` | One `SeamSet` |
| `PieceId` | A pattern piece; one per chart, numbered as its chart. When mirror-image pieces collapse into one template (FR-034, T116), the template lists its member `PieceId`s and is named by the lowest | `openplush-pattern` | One `SeamSet` |
| `ProposalId` | A proposal, derived from its content (below) | `openplush-seams` | One repaired mesh |

All are `u32` newtypes except `ProposalId`, which is a `u64`. Numbering by lowest member makes them deterministic (Constitution V) without a hash map.

### Proposal: a seam-set delta

Every remedy the engine can carry out is a seam edit: extra seams to make a chart a disk (FR-013), darts (FR-022; a dart *is* a seam ending inside a piece, §4), piece splits (FR-052), and seams along creases (FR-025). The mesh never changes when seams change (§4). So one type covers all of them, and accepting one is the same operation every time:

```rust
pub struct Proposal {                      // fields private; read through accessors
    id: ProposalId,                        // derived from kind + add, never set directly
    kind: ProposalKind,
    add: BTreeMap<EdgeId, SeamKind>,       // never empty; never an edge already in the SeamSet
    explanation: String,                   // plain language: what it fixes and what it costs
}
// The only constructor, Proposal::new(kind, add, explanation), refuses an empty
// `add` and computes `id`, so the ID can never drift from the edit.

#[non_exhaustive]
pub enum ProposalKind {
    CloseChart { chart: ChartId, failure: ChartFailure },   // FR-013: make a chart a disk
    Dart { chart: ChartId },                                // FR-022: relieve distortion
    SplitPiece { chart: ChartId },                          // FR-052: nap coherence (the piece is its chart's)
    Seam { reason: SeamReason },                            // any other seam the engine suggests
}

#[non_exhaustive]
pub enum SeamReason {
    Crease,      // FR-025, T106: a concave region with no seam through it
    SeamHint,    // feature 002: an accepted placement hint (its `to_proposal`)
}
```

- **Ownership**: `Proposal`, `ProposalKind`, `SeamReason` and `ProposalId` live in `openplush-seams`, which produces proposals (T076, T105, T106); `openplush-pipeline` re-exports them. `SplitPiece` names the chart, not the `PieceId`, because seams sits below pattern.
- **Edges only**: a suggested seam that does not follow mesh edges (a self-intersection curve, §3 step 8) is snapped to the shortest edge path along it (T071) or, failing that, offered as an `Advisory` remedy, never as a proposal.
- **Accepting** a proposal inserts `add` into the `SeamSet` and re-runs the affected charts (FR-014). Preview is the same insertion on a copy (Constitution VII).
- **Proposals only add seams.** Removing a seam is never an automatic remedy in v0.1.

### ProposalId

> **As built**: [`docs/reference/engine-api.md`](../../docs/reference/engine-api.md#wire-forms) and rustdoc. Frozen here as the original intent ([ADR 0003](../../docs/adr/0003-as-built-docs.md)).

A 64-bit FNV-1a hash (offset basis `0xcbf29ce484222325`, prime `0x100000001b3`) of the proposal's edit, not a counter, so the same fix proposed twice has the same ID even after unrelated seam changes renumber charts:
- **Bytes hashed**, in order: one byte for the `ProposalKind` variant (0 `CloseChart`, 1 `Dart`, 2 `SplitPiece`, 3 `Seam`); then, for each entry of `add` in key order, the `EdgeId` as 4 bytes little-endian followed by one byte for the `SeamKind` (§4 declaration order). Chart IDs and text are deliberately left out: they can change while the edit stays the same.
- **Stable** for as long as the repaired mesh is, since `EdgeId`s are.
- **On the wire** (worker protocol) it is a 16-character lowercase hex string: a `u64` as a JSON number loses precision above 2^53.
- **Applying an ID no longer offered** (the edit is no longer proposed, or the mesh was re-repaired) is the typed error `EngineError::ProposalNotOffered { id }`, never a silent no-op.

### Locations

```rust
#[non_exhaustive]
pub enum WarningTarget { Face(FaceId), Edge(EdgeId), Chain(SeamChainId), Piece(PieceId), Component(ComponentId), Project }
```

### Warning codes and order

> **As built**: [`docs/reference/engine-api.md`](../../docs/reference/engine-api.md#warnings) and rustdoc. Frozen here as the original intent ([ADR 0003](../../docs/adr/0003-as-built-docs.md)).

`WarningCode` is a `#[non_exhaustive]` enum. Each variant has a stable string form (`"stretch-exceeds-fabric"`, kebab-case) that never changes once released, because acknowledgements are persisted by code (§12, §13). The variants are added by the tasks that first raise them. Each code is documented in `docs/reference/` by the PR that adds it (ADR 0003).

`WarningSet` is sorted by the code's **string** (not its declaration order, so adding a variant never reorders a printed summary, T157), then by target (variant order as declared above, then ID), then by the remedy (an `Actionable` proposal's `ProposalId`, then an `Advisory`'s text), then by explanation. Two warnings equal in all of these are one warning. The order is therefore total and independent of insertion order (T030).

### Import format

```rust
#[non_exhaustive]
pub enum Format { Obj, Gltf, Glb, Stl, Ply }
```

## 14. Contracts to Write Next (`contracts/`)
1. `engine-api.md` — `openplush-pipeline` functions, inputs/outputs, error enum, `WarningSet`.
2. `worker-protocol.md` — message types, handle lifetime, transferables, cancellation.
3. `project.schema.json` — JSON Schema for the project document, v1.
