---
status: Proposed
date: 2026-10-14
deciders: maintainer (proposed by the implementing agent from spike S7, decided in review of the T007 PR)
---

# Boundary-pinned flattening holds; the fallback stays a rare typed error and the dart signal is area-integrated compression

## Context and problem statement

D-013 (plan Decision Ledger) made seam parity exact and hard: every chart's boundary edges are pinned to their 3D lengths, and the interior absorbs all distortion. Research R3 named two risks: the pinned outline may not close, and the pinned map may not be injective, where free-boundary SLIM had a local guarantee. Constitution III makes det J > 0 and a non-overlapping outline a contract, so each failure must be a typed error with a dart or extra-seam proposal (T082), never relaxed parity. T007 (spike S7) asked how often those failures happen on the fixture corpus, and what pinning costs in interior distortion. The answer sets how aggressive T082 must be, and which distortion measure should drive dart suggestions (T105). Evidence: [`research/s7_boundary_pinned/`](../../research/s7_boundary_pinned/README.md).

## Decision drivers

* Constitution III: injectivity and a simple outline are post-flatten contracts; a violation is a typed error, never a fix-up.
* FR-023 / D-013: parity is a contract, so no fallback may trade it away.
* Constitution VII: a dart or seam proposal changes the user's seams, so it needs their approval; proposing one is not free.
* Constitution IV and V: the measure that drives a proposal must be stable under remeshing, or the same model suggests different darts at different resolutions.
* Contributor burden: a fallback that never fires is untested code.

## Considered options

* **Option 1**: keep D-013; T082 is a rare-path typed error that fires only on an actual geometric failure; dart suggestions (T105) are driven by area-integrated compression measured on the pinned result.
* **Option 2**: keep D-013, but make T082 pre-emptive: split or dart any chart whose curvature or predicted distortion passes a threshold, before flattening fails.
* **Option 3**: relax D-013 to a heavy boundary-length weight (R3 Option B) wherever pinning is costly.

## Decision outcome

Chosen option: **Option 1**, because on every input S7 tried, the failures T082 guards against did not occur, while the cost of pinning is real but shows up as interior compression, which is exactly what the heatmap and dart suggestions are for.

**S7 verdict on D-013, fixture corpus** [likely]: 34 fixture runs (the 9 charts of the three reference seam sets, the same 9 with seeded asymmetric jitter, and 16 runs at one and two levels of refinement, up to 1,728 triangles): **0 closure failures, 0 injectivity failures**. Every outline was simple and counter-clockwise, no all-boundary triangle was inverted, every run started from an injective cotan fill (the uniform fallback was never needed) and converged with zero flips. Closing the outline with lengths held exact needed at most 0.026° of turning-angle change at any corner. The synthetic stress set (19 charts plus a 12-run resolution sweep, up to 6,144 triangles, 55 reflex corners, caps covering 175° of the sphere) gave the same: no failure on any geometrically valid chart.

**Distortion penalty** (pinned vs free-boundary SLIM, symmetric Dirichlet, 4.0 is isometric) [likely]:

| Chart | Pinned SD | Free SD | Pinned min σ₂ | Free min σ₂ | Mean compression deficit, pinned | Free seam-length drift (max edge) |
|---|---|---|---|---|---|---|
| Developable (capsule tube, cube net) | 4.000 | 4.000 | 1.000 | 1.000 | 0 | 0 |
| Sphere gore, 60° wide | 4.022–4.032 | 4.004–4.013 | 0.88–0.91 | 0.92–0.96 | 6.5–7.7% | 5–9% |
| Hemisphere cap | 5.20–5.28 | 4.14–4.23 | 0.60–0.63 | 0.85–0.87 | 36–38% | 41–46% |

Developable charts pay nothing, and the pinned pipeline reproduces the exact net [solid: closed-form oracle]. On curved charts the penalty grows with curvature, and the free solve hides it by drifting seam lengths far past anything a sewist could ease in. Pinning moves that error into interior compression, where the user can see it and add a dart.

**Implications**:

* **T082 is a rare path, not a routine one** [likely]. It fires only on an actual failure: closure did not converge, the outline self-touches, an all-boundary triangle is inverted, or neither linear start is injective. Each returns its typed error with a dart or extra-seam proposal, and parity is never relaxed. No pre-emptive splitting. It must still exist and be tested on constructed failures, because the corpus never triggers it.
* **Distortion is a fabric-tolerance question, handled by T103–T105, not T082** [likely]. A hemisphere flattens validly, but its fabric must gather: a 37% mean compression deficit, and σ₂ down to 0.61. That is a warning and a dart suggestion against the fabric profile, not a flattening failure.
* **The dart signal uses area-integrated compression** [likely]: the mean compression deficit `Σ A·max(0, 1 − σ₂) / ΣA` and the compression at the 5% area quantile, per chart and per cluster. These were stable under refinement on every fixture (gore 7.03%, 6.89%, 6.86%; cap 37.3%, 36.5%, 36.3%). The single worst triangle's σ₂ is not: it kept falling with refinement on the synthetic stress caps (0.145 to 0.051) and on the jittered cube (0.963 to 0.945). Per-triangle σ stays the heatmap's input (FR-021); it must not be the threshold in T105 steps 1 and 4. This matches data-model §5, whose distortion term D is already area-weighted.
* **Input validation stays upstream** [likely]: folded 3D input was the only source of injectivity failures S7 ever saw (its first synthetic draft). That is T174's typed error, before T079.

### Consequences

* Good, because parity stays exact with no cross-chart coupling, and the measured cost lands where FR-021 and FR-022 can show and relieve it.
* Good, because T082 stays small: a typed error plus a proposal on failure, no heuristics about when a chart is "too curved".
* Good, because the dart signal is resolution-independent, so remeshing (T001) does not change which darts are suggested.
* Bad, because T082's fallback is never exercised by the corpus; its tests must construct failing charts (a non-closing length sequence, a self-touching outline, a rim triangle inverted at a reflex corner).
* Bad, because the evidence is thin where it matters most: see the corpus limits below. The decision is [likely], not [solid].

### Confirmation

* T079–T081 tests run on every fixture chart and assert closure, a simple outline, and det J > 0, so a regression shows up as a typed error in CI.
* The tube and cube net are exact developable oracles for T079–T080: SD 4.000, σ = 1 within tolerance.
* T082's tests construct each failure and assert the typed error, the proposal, and unchanged boundary lengths.
* Re-run `research/s7_boundary_pinned/run.py` when the corpus gains a scanned or sculpted model (contributed CC0/CC-BY fixture); any closure or injectivity failure reopens this ADR.

## Pros and cons of the options

### Option 1: rare-path fallback, area-integrated dart signal

* Good, because it matches the measured failure rate (zero on valid input) and keeps every proposal user-approved and evidence-driven.
* Good, because developable and gently curved panels flatten untouched.
* Bad, because a failure mode outside the corpus (sharp interior features, human-drawn asymmetric seams on dense scans) would surface only as an error, with a proposal, at flatten time.

### Option 2: pre-emptive darts or splits

* Good, because a user would rarely see a flattening error.
* Bad, because it changes seams without a failure to justify it, which Constitution VII forbids without approval, and the threshold would be a guess: nothing in S7 predicts failure from curvature.
* Bad, because it duplicates T105, which already proposes darts from measured distortion.

### Option 3: heavy boundary weight where pinning is costly

* Good, because interior distortion would drop (free SD 4.15 vs pinned 5.21 on a hemisphere).
* Bad, because FR-023 forbids handing the sewist a length residual, and the residual is large: the unweighted free baseline drifts 41–46% on a hemisphere and 5–9% on a gore. A heavy weight shrinks it, but a residual remains by construction [solid].

## More information

* **Corpus limits** [solid]: three procedural shapes (sphere, capsule, cube), ≤ 192 triangles before refinement, smooth or piecewise flat, with symmetric seam layouts. The committed charts close by symmetry alone; seeded jitter (≤ 0.1 × the shortest edge) supplies the asymmetry. No plush scan, no sculpted model, no interior cone sharper than 20°, no human-drawn seams. The synthetic stress set is the only evidence near the failure edge.
* **Open** [verify]: behaviour at the ~30k-triangle proxy resolution, and on charts with a sharp interior cone (an ear tip inside a panel), which is where a dart-free chart is most likely to fail injectivity. Revisit when such a fixture exists.
* **Open** [speculative]: an untangling rung in T080. Not needed on any input so far; add it only if a fixture needs it.
* Symmetric Dirichlet is a geometric proxy, not a fabric model; compression 0.61 means "needs a dart or gathers", not a measured fabric strain.
* Sawhney & Crane, "Boundary First Flattening" (2017). Rabinovich et al., "Scalable Locally Injective Mappings" (2017). Smith & Schaefer, "Bijective Parameterization with Free Boundaries" (2015).
* Tasks: T007 (this spike), T079 boundary, T080 interior, T081 injectivity check, T082 fallback, T103–T105 metrics and dart suggestion, T174 folded-chart validation.
