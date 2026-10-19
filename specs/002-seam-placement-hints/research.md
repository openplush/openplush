# Phase 0 Research — 002-seam-placement-hints

**Feature**: `002-seam-placement-hints` | **Date**: 2026-10-13 | **Status**: Draft

Confidence tags as in [001's research](../001-pattern-pipeline/research.md): **[solid]** consensus / verified · **[likely]** reasonable inference · **[speculative]** guess · **[verify]** check before committing.

This feature adds no new dependencies and no new architectural layer. Everything it needs — curvature, direction fields, symmetry detection, the proposal mechanism — was already specified for 001 and is reused. The research question is therefore not "which library" but **"which geometric signal corresponds to each thing a sewist would actually do"**.

## R1. The four detectors

### Sharp edges and creases

**Decision**: dihedral angle thresholding on edges, chained into polylines, filtered by physical length.

For each edge, take the dihedral angle between its two incident faces. Above a threshold the edge is a crease. The signed version distinguishes convex (ridge) from concave (valley), which is the entire basis for separating this detector from the next one.

- **Why edges and not vertices**: a crease is a curve. Reporting vertices gives the maker dots to interpret; reporting chained polylines gives them something a seam can follow directly, which FR-030 requires.
- **Chaining**: walk from each crease edge to adjacent crease edges, preferring the continuation with the smallest turning angle. Stop at junctions, and emit junctions as their own short hints so a T-junction is not arbitrarily resolved into one arm.
- **Filtering**: drop chains shorter than a physical length (provisional 10 mm at target scale). This is what prevents the faceted-model hint flood that SC-010 caps — a cube's 12 edges survive, an icosphere's hundreds of near-coplanar edges do not.
- **Note on units**: dihedral angle is scale-independent [solid], so FR-007's physical-units rule applies to the *length* filter, not to the angle. Both still move when the model is resized, because the length filter is in millimetres.

**Rejected**: per-face normal clustering (segments the surface but loses the curve), and curvature-derivative ridge extraction (Ohtake et al. 2004 — sharper on smooth organic surfaces, far more sensitive to mesh noise, and overkill when the target is "where would a sewist cut").

### Concave creases

**Decision**: the same dihedral computation, filtered to the **concave** sign, with a separate and lower threshold.

A concave crease earns its own category because it carries a different consequence. 001's research R4 established the finding [solid]: stuffing pushes every panel outward, so a concave crease survives in the finished toy **only if a seam runs along it**. A convex ridge is a style choice; a concave valley is load-bearing — the neck, the gap between legs, the join where a limb meets a body.

- **Lower threshold than sharp edges** [likely]: a shallow valley still vanishes when stuffed, so it is worth flagging earlier than a shallow ridge is.
- This is the same signal 001's FR-025 warning uses. **The two must share an implementation.** A region that warns "unsupported concavity" after flattening but showed no hint beforehand would be the tool contradicting itself, and two separately tuned thresholds would drift into exactly that.

### Dart candidates

**Decision**: integrated absolute Gaussian curvature over a geodesic neighbourhood, thresholded, clustered, one hint per cluster.

Gaussian curvature is the right signal because it *is* the obstruction to flattening. A region with concentrated curvature cannot be flattened without stretching or a dart, and the angle defect at a vertex quantifies exactly how much material has to go somewhere [solid].

- Discrete Gaussian curvature as angle defect: `K_v = 2*pi − (sum of interior angles at v)`.
- Integrate over a geodesic disk of physical radius (provisional 15 mm) so the result describes the *shape* rather than the mesh resolution. Without this, a dense mesh and a coarse mesh of the same object produce different hints, which breaks the identity stability FR-008 expects.
- Cluster adjacent over-threshold vertices and emit one hint per cluster, located at the maximum-defect vertex — which is also precisely the cone point that 001's dart suggestion (data-model section 5) walks toward.

**The honest limit** [likely]: this detects *where a dart is needed*, not *which dart*. Direction, length, and width come from 001's existing dart machinery when the maker accepts. So for this category, FR-030's "follows the whole flagged feature" means the cluster region, not a specific dart polygon — and the spec should be read that way.

### Symmetry

**Decision**: PCA for candidate planes, reflection-ICP refinement, explicit match score with a rejection threshold.

1. Candidate planes from the principal axes of the vertex cloud, **weighted by face area** so tessellation density does not bias the result.
2. For each candidate: reflect the mesh and measure mean closest-point distance back to the original, iterating a few times to refine the plane.
3. Score = mean reflected distance divided by bbox diagonal. Below a threshold the plane is accepted; otherwise **no plane is reported**, because FR-005 requires reporting absence rather than guessing.
4. Report the score alongside the plane, so FR-005's "how closely the model matches it" is a number the maker can judge rather than a bare claim.

This is the approach 001's data-model section 5 already names for mirroring seams, so this feature *supplies* that plane rather than inventing a second mechanism.

**Multiple planes**: a sphere or any radially symmetric shape has infinitely many. FR-005 caps the output at one dominant plane. Picking the best-scoring plane on a sphere is arbitrary but harmless; picking one on a *nearly* radially symmetric animal would be actively misleading. Mitigation: when a second distinct candidate plane also scores within the acceptance threshold, report none and say why; the gap between two fitting planes ranks how exactly each mirrors the shape, not which one the maker sews by (H-010, spike H4). Ambiguous is closer to the truth than arbitrary.

## R2. Sensitivity, and the monotonicity requirement

FR-006 requires that raising sensitivity only ever **adds** hints. This is a real design constraint, not a nicety — a control that removes the hint you were looking at as you turn it up is worse than no control at all.

Monotonicity is free if and only if sensitivity maps to a single threshold per detector, and the detector is a pure threshold filter over a precomputed ranked signal:

- Compute the signal once (dihedral angle per edge, integrated curvature per vertex).
- Sensitivity `s` in `[0,1]` maps to a threshold that decreases monotonically in `s`.
- The hint set is every feature whose signal exceeds that threshold, which is nested by construction as `s` rises.

**This forbids an obvious temptation**: adaptive or percentile-based thresholds, the "show me the top 10 percent" approach. Those are not nested — raising sensitivity can reshuffle which features qualify, and the guarantee dies. Fixed absolute thresholds keep it, and the guarantee is then testable with a subset assertion.

The length and cluster filters must be monotone too. Chaining is the step that needs care, since merging two chains changes their length: chain **once** at the most permissive threshold and filter the resulting chains afterwards, rather than re-chaining per sensitivity level.

## R3. Hint identity across recomputation

FR-008 requires identity stable enough to survive dismissal and re-matching; FR-041 requires an unmatched dismissal to be surfaced rather than dropped.

**Decision**: identity by **provenance key plus category**, reusing 001's existing mechanism (data-model section 12). A sharp-edge hint is keyed by the sorted set of provenance edge keys in its chain; a dart candidate by the provenance face key at its peak; symmetry by its category alone, since there is at most one.

Re-matching after a model change:

| Case | Behaviour |
|---|---|
| Exact key match | Same hint; the decision carries over silently |
| Partial match — a new hint shares more than half its provenance keys with a dismissed one | Treated as the same hint, dismissal carries over, and the approximate match is recorded |
| No overlap | The dismissal is orphaned, and FR-041 surfaces it for re-confirmation |

The overlap rule exists because a small edit near a crease changes a few edges without changing the maker's judgement about that crease. Requiring exact matches would resurrect dismissed hints constantly; requiring none would suppress genuinely new ones. [likely — the 50% figure needs tuning against real editing sessions, spike H3]

## R4. Why there are no new dependencies

| Need | Already available from 001 |
|---|---|
| Dihedral angles, curvature | `openplush-fields` (001 T144) |
| Mesh traversal, provenance keys | `openplush-mesh`, `openplush-project` |
| Symmetry plane | Specified in 001 data-model section 5; implemented here |
| Proposal preview-then-accept | `openplush-pipeline` proposals (001 T107) |
| Seam and dart creation from a path | `openplush-seams` (001 T069–T077) |
| Flat-pattern preview | `openplush-flatten` plus `openplush-pattern` (001 T078–T087) |
| Overlay rendering, inspection panel | `apps/web` patterns from 001's health report and quality view |

Two consequences. No ADR is required (no new dependency category, per `AGENTS.md`). And the hard dependency on 001's flattening is not incidental — it is four of the seven rows above, which is why Q3 resolved the way it did.

## R5. Prior art

- **Blender 3D Print Toolbox** — the interaction model the maker asked for: a panel of named checks, each listing located results, each clickable to jump to it. Its checks concern *printability* (non-manifold, thickness, overhangs). This feature borrows the shape of the interface and substitutes sewing checks; 001's FR-003 and FR-004 already cover the overlapping mesh-defect half.
- **Seamster** (Sheffer & Hart 2002) — seam placement guided by visibility and curvature. Origin of the idea that high-curvature regions are where seams belong.
- **D-Charts** (Julius, Kraevoy, Sheffer 2005) — quasi-developable segmentation, the likely basis for v0.2's automatic layout. This feature surfaces the same signals to a human first, which is Constitution X's "baseline before cleverness" in practice.
- **Variational Surface Cutting** (Sharp & Crane 2018) and **OptCuts** (Li et al. 2018) — full cut optimisation. Deliberately *not* used: they produce a complete cut set, which is exactly the automatic layout this feature is scoped to avoid.
- **Suggestive interfaces** (Igarashi & Hughes 2001) — the general principle that a tool offering options a user picks from beats a tool that decides. FR-032's "never auto-applied" follows from this as much as from Constitution VII.

## R6. Open spikes

| Spike | Question | Box |
|---|---|---|
| H1 | Default thresholds: tune the dihedral, concave, and curvature defaults against the fixture corpus so a typical plush yields useful hints with no adjustment. Needs the hand-made-pattern fixtures first | 1 day, after fixtures |
| H2 | Does the monotonicity guarantee (R2) survive chaining and clustering in practice? Property test over random sensitivity pairs asserting subset containment | 0.5 day |
| H3 | Re-match overlap threshold (R3): is 50% right? Needs recorded editing sessions, so it ships configurable and gets tuned when there is data | deferred |
| H4 | Symmetry on near-symmetric models: how close must two candidate planes score before reporting none beats reporting the best? | 0.5 day |
| H5 | SC-001 measurement harness: compare hint sets against hand-made patterns. **Blocked** on `fixtures/` gaining models with known-good hand-made patterns — the criterion cannot be evaluated at all until then | 1 day, blocked |
