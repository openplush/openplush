# Spike S7: boundary-pinned flattening (D-013)

**Status (2026-10-15): T007 answered on the fixture corpus.** Every chart of every reference seam set in `fixtures/` closes and flattens injectively with its boundary pinned, as do seeded asymmetric variants and two refinement levels: 36 fixture runs (10 charts, the same 10 jittered, 16 refined), zero closure failures, zero injectivity failures. The verdict and what it means for T079–T082 are in [ADR 0005](../../docs/adr/0005-boundary-pinned-flattening.md). The corpus is three coarse, smooth or piecewise-flat shapes (see Limits), so the verdict is [likely], not [solid]; the synthetic results below are the stress evidence the fixtures cannot give yet.

Non-shipping research code (plan.md: `research/` holds reference implementations, not product code). It answers the structural questions now so the Rust flattener (T079 boundary, T080 interior, T082 fallback) is designed around the right failure mode.

> **Correction (2026-10-14, same PR).** The first version of this spike reported that three stress charts had "no injective start" and that failures grew with resolution. An adversarial review found those charts were **folded 3D input**: the cap's polar angle passed 180°, so the surface wrapped past the south pole. A second, smaller defect was the radial lobe warp, which inverted triangles whenever lobe depth × lobe count ≳ 1, at every resolution. Both are fixed: the generators now refuse folded caps, lobes come from a provably one-to-one conformal warp, and an independent validity oracle (`test_s7.Validity`) checks every chart before it is flattened. The findings below are from the corrected corpus.

## The question

D-013 pins every chart's boundary edges to their 3D lengths, so the two sides of a seam always match, and puts all distortion in the interior. Research R3 lists two risks:

1. The prescribed lengths may not **close** into a flat outline.
2. The map may not be **injective** (some triangle folds over, det J ≤ 0), where free-boundary SLIM had a local guarantee.

## Fixture corpus

### What the fixtures contain

| Fixture | Charts | Triangles | Interior curvature ΣK | What it tests |
|---|---|---|---|---|
| `sphere` (six gores) | 6 identical gores, pole to pole, 60° wide | 28 each | 59° each; no interior cone above 12° | A curved panel with sharp (≈ 60°) corners at the poles, where six seams meet |
| `capsule` (two caps, slit tube) | top cap, tube, bottom cap | 60, 72, 60 | 268° per cap; tube 0 | Caps: a hemisphere, the worst case a plush maker would cut in one piece. Tube: exactly developable (planar quads) |
| `cube` (cross net) | 1 chart, the whole cube | 108 | **0**: all 8 cube corners (π/2 defect each) lie on the cut tree | Exactly developable. Its 4 tree leaves are free seam endpoints, so they stay single vertices (slits), as dart tips do |

So the corpus has **no interior cone singularity** worth the name: the cube's corners are all on the cut, and the smooth shapes spread their curvature over many small cones (≤ 13.4° each). Every committed chart is also mirror- or rotation-symmetric or developable, so its BFF outline closes **by symmetry alone** (natural gap ≤ 3e-12 of the perimeter). To exercise closure anyway, `run.py` adds **asymmetric variants**: every mesh vertex moved by a seeded random vector of at most 0.1 × its shortest edge, at the committed resolution and before cutting, so seam sides still match. `test_corpus.Jitter` checks they are reproducible and do not crease (no fold opens by more than 30°, none passes 120°).

### Method additions

- `corpus.py` reads `fixtures/meshes/*.obj` and `fixtures/seams/*.json` (typed `object` until each field is checked; enforces the fixtures/README.md format: version 1, base 0, smaller index first, a sorted list, a schema seam kind; raises on anything unexpected) and cuts virtually per data-model §4: faces flood-filled across non-seam edges, each vertex split once per fan of corners joined across non-seam edges. A free seam endpoint has one fan, so it stays single (a slit). Each chart must be a disk (connected, χ = 1, one boundary loop) or the cut raises `NotADisk`.
- **Refinement sweep**: 1–2 levels of midpoint subdivision; seam edges split with their faces, so parity is preserved. Unjittered meshes are projected back onto the analytic sphere (r 50), capsule (r 30), or the cube's planes. Jittered meshes are jittered once at the committed resolution and then refined **flat**, so every level is the same polyhedron and the levels form a convergence sequence (its cones stay put; a re-jitter per level would be a new surface each time).
- **Area-integrated measures** alongside the per-triangle extremes, because the synthetic sweep showed extremes don't converge: the compression at the 5% area quantile, the mean compression deficit `Σ A·max(0, 1 − σ₂) / ΣA`, and the mean stretch excess `Σ A·max(0, σ₁ − 1) / ΣA`. Singular values now come from an SVD; the old closed form cancelled to 1e-8 noise near isometry (caught by `test_s7.AreaMeasures`).
- **Free-boundary outline check**: the baseline's final outline is tested for self-overlap too, since SLIM's barrier is only local.

Oracle tests (`test_corpus.py`, 16 tests): hand-derived chart sizes (faces, vertices, boundary edges) for every fixture; an independent χ and loop count; faces partitioned and positions kept through the cut; every seam edge on exactly two chart boundaries and no other edge on any; a slit tip stays single; an annulus, an uncut sphere, a non-edge seam, floats or booleans as indices, an unknown seam kind, an unsorted or repeated edge list, a reversed pair, version ≠ 1, base ≠ 0, and malformed OBJ lines all raise; subdivision quadruples faces and stays on the sphere; jitter is seeded, bounded, seam-consistent and fold-free.

### Fixture results

Full tables in [`results.md`](./results.md) (fixture section first).

| Chart | Tris | Max angle fix | Start | Pinned SD | Pinned min σ₂ | Mean compress deficit | Free SD | Free min σ₂ | Free seam-length drift (max edge) |
|---|---|---|---|---|---|---|---|---|---|
| sphere gore (each of 6) | 28 | 0.000° | cotan | 4.026 | 0.901 | 7.0% | 4.011 | 0.944 | 5.2% |
| capsule cap (each of 2) | 60 | 0.000° | cotan | 5.256 | 0.612 | 37% | 4.220 | 0.874 | 41% |
| capsule tube | 72 | 0.000° | cotan | 4.000 | 1.000 | 0 | 4.000 | 1.000 | 0 |
| cube net | 108 | 0.000° | cotan | 4.000 | 1.000 | 0 | 4.000 | 1.000 | 0.0007% |
| jittered, worst of 10 | 28–108 | 0.026° (cap) | cotan | 5.276 (cap) | 0.602 (cap) | 37.5% (cap) | 4.225 | 0.853 | 45% (cap) |
| sweep, worst of 16 | 112–1,728 | 0.004° | cotan | 5.216 (cap L1) | 0.623 (jittered cap L1) | 36.5% (cap L1) | 4.161 | 0.848 | 47% (jittered cap L2) |

1. **Closure never failed** [likely]. On the committed charts it is trivial (symmetry). On the asymmetric variants the natural gap was at most 0.04% of the perimeter (jittered caps), fixed with at most **0.026°** of turning-angle change at any corner, and the gap shrinks with refinement of the same jittered polyhedron (cap: 4e-4, 1e-4, 3e-5). Every outline was simple and counter-clockwise; no all-boundary triangle was inverted.
2. **Injectivity never failed** [likely]. All 36 runs started from an injective **cotan** fill (the uniform fallback was never needed) and converged (`gtol` or `stalled`) with zero flips. The free baseline's outlines never overlapped either.
3. **Developable charts pay nothing** [solid]. The tube and the cube net flatten to SD 4.000 and σ ∈ [1.000, 1.000] at every level: the pinned pipeline reproduces the exact rectangle and the exact cross net, including the cube's slit tips. These are the constitution IV developable oracles, on real fixtures.
4. **The penalty is concentrated where curvature is** [likely]. A 60° sphere gore: pinned SD 4.026 vs free 4.011, min compression 0.90 vs 0.94, and the free solve drifts seam lengths by 5–8%. A hemisphere cap: pinned SD 5.26 vs 4.22, compression 0.61 vs 0.87, and free drift 41–47%, i.e. no free solve preserves parity on it. Same numbers as the synthetic cap-90 (5.20, 0.63, 42%), so the synthetic findings transfer.
5. **Area measures converge; the single worst triangle need not** [likely]. Convergence evidence is the unjittered sweeps, the jittered polyhedra refined flat, and the synthetic stress caps. On the smooth unjittered charts both converge: mean compression deficit gore 7.03%, 6.89%, 6.86% and cap 37.3%, 36.5%, 36.3%; min σ₂ gore 0.901, 0.906, 0.907. On the jittered polyhedra, whose cones (up to 20°) stay fixed under refinement, the deficit's steps shrink each level (cap 36.6%, 35.3%, 34.9%; cube 0.53%, 0.44%, 0.40%), and so do the 5%-area quantile's (cube 0.978, 0.984, 0.985). Min σ₂ falls by a near-constant step per level instead (cube 0.963, 0.957, 0.951; gore 0.897, 0.890, 0.884), consistent with a scale factor that is singular at a cone [likely]. On the synthetic stress caps it fell from 0.145 to 0.051. The dart signal should use the area measures.

## Synthetic corpus (first pass)

The rest of this README is the synthetic prototype that preceded the fixtures. It is the stress evidence: higher resolution, deep lobes, near-full spheres.

### Method

| Step | What | Why |
|---|---|---|
| Boundary curvature | BFF (Sawhney & Crane 2017) with boundary scale factor u = 0: solve A_II u_I = −K_I, then κ̃ = κ + (A u)_B | u = 0 keeps every boundary length at its 3D value |
| Closure | Keep lengths; least-squares change to the turning angles subject to closure (SLSQP, then Gauss-Newton polish to 1e-14) | BFF closes by changing lengths, which would break parity |
| Outline check | Counter-clockwise, and no two non-adjacent edges cross **or touch** | A self-touching outline can't be cut |
| Rim check | Triangles with all three corners on the boundary must not be inverted | The boundary alone fixes them, so no interior solve can repair them |
| Interior start | Harmonic fill with the boundary fixed: cotan weights, then uniform (Tutte) weights | The two cheap linear starts the Rust code would try first |
| Interior solve | L-BFGS on the symmetric Dirichlet energy (SLIM's), boundary fixed, step capped at the first triangle flip (Smith & Schaefer 2015, cancellation-free roots); reports why it stopped | Cannot create a flip, and never reports a non-converged run as `ok` |
| Baseline | Free boundary: Tutte onto a circle, then the same energy with every vertex free | How much distortion pinning costs, and how far boundary lengths drift without it |

The corpus is made of analytic surfaces. Lobed and asymmetric outlines come from a conformal warp `w = z(1 + Σ εₖ zᵏ)` of the disk with `Σ(k+1)|εₖ| < 1`. That bound makes the planar warp one-to-one and orientation-preserving (Noshiro–Warschawski), so a deep lobe never folds the chart. It does not make every triangle well-shaped: the map onto a near-full sphere squeezes the rim rings, and the 175° stress caps have rim slivers with a minimum corner angle of about 1.8° (the `Min angle` column). That is realistic input for a scanned or sculpted model, and the solver handled it. The `Reflex` column counts inward-turning outline corners, to show that the non-convex cases really are non-convex.

Oracle tests (`test_s7.py`, 25 tests):
- **Input validity** on every chart: sphere charts face outward, and no two adjacent faces fold over each other.
- Euler characteristic and Gauss–Bonnet, both for the 3D chart and for the flattened boundary.
- A finite-difference gradient check.
- Exact closure with exact lengths.
- Touching and clockwise outlines are rejected.
- Quadratic roots match `numpy.roots`, including the near-linear case.
- A flat disk must map to itself, and must converge rather than run out of iterations.
- A bent plane (developable) must flatten almost isometrically.
- The area measures match a known linear map (singular values 1.2 and 0.9) and a piecewise map on unequal triangles (an unweighted mean, a 50% quantile, or the maximum would each fail), and a mirrored outline is rejected.

Run it: `pip install -r requirements.txt`, then `python3 -m unittest -v` and `python3 run.py > results.md` (about 95 s, most of it the synthetic resolution sweep; the fixture runs take about 3 s). The output is deterministic, and `results.md` records the Python/numpy/scipy versions it was generated with.

### Results

Full table in [`results.md`](./results.md). Read the columns as:
- **SD mean**: 4.0 means isometric (no distortion); higher is worse.
- **Stretch / compress**: the largest and smallest singular values. Compression 0.63 means the fabric there must gather into 63% of its 3D extent.
- **Stop**: why the optimizer stopped, with the final gradient inf-norm. `gtol` means the gradient fell below 1e-8 of its starting value. `stalled` means the energy stopped changing at machine precision. Both count as converged; `maxiter` or `linesearch` would be reported as a failure, and none occurred.

#### Corpus (14 charts) and stress set (5 charts)

- **Closure never failed.** The BFF outline's natural closure gap was at most **0.6%** of the perimeter (bump-h1.2). Closing it with lengths held exact needed at most **0.05°** of turning-angle change at any corner on the 640–1,536-triangle charts, and up to **0.10°** on the coarsest sweep mesh (asym-cap-p175, 384 triangles). Every outline was simple and counter-clockwise, and no all-boundary triangle was inverted.
- **Injectivity never failed.** All 19 charts had an injective linear start: cotan on 18, and uniform on bump-h1.2, where cotan folded. Every interior solve converged with zero flips. That includes the stress caps, which cover 175° of the sphere in one piece with 18–25 reflex corners.
- **Resolution sweep** on the three 175° stress caps, at 384, 1,536, 3,456 and 6,144 triangles: zero folds at every size (up to 55 reflex corners). The natural closure gap shrinks as the mesh refines (lobed-cap-p175: 0.156%, 0.055%, 0.026%, 0.015%), so it is a discretisation effect that converges, not a structural failure. **Minimum compression does not converge** on these caps: it keeps falling as the rim slivers refine (star-cap-p175: 0.145, 0.081, 0.063, 0.051). Per-triangle extremes are mesh-dependent there, so the dart signal should use area-integrated or converged measures, not the single worst triangle.
- **The cost is interior distortion, and it grows fast with curvature:**

  | Chart | Pinned SD | Pinned min compress | Free SD | Free boundary-length drift (max edge) |
  |---|---|---|---|---|
  | cap-45 | 4.06 | 0.89 | 4.01 | 8.3% |
  | cap-90 (hemisphere) | 5.20 | 0.63 | 4.14 | 42% |
  | gore-90 (¼ of a ball) | 4.13 | 0.84 | 4.02 | 16% |
  | ellipsoid-cap-90 | 8.12 | 0.40 | 4.60 | 83% |
  | ear | 12.4 | 0.27 | 5.03 | 420% |
  | star-cap-p175 (stress) | 18.2 | 0.08 | 5.06 | 590% |

### Findings (synthetic)

1. **Closure is not the failure mode** [likely; synthetic]. With u_B = 0 and a harmonic interior factor, the flattened metric is flat on a disk, so in the smooth limit the boundary closes (Sawhney & Crane 2017). The discrete gap is what remains, and it shrank with refinement in every sweep here. Pinning only lengths leaves every turning angle free, and the angle fix needed was tiny everywhere. R3's "fails to close" risk can be downgraded. The closure step still belongs in T079, as a least-squares angle correction rather than BFF's length correction.
2. **On valid input, injectivity held everywhere** [likely; synthetic]. On every geometrically valid chart tried (up to 6,144 triangles and 55 reflex corners) a linear start was injective, and the optimizer kept it that way. **The injectivity failures seen in this spike's first version came from folded input**, not from the method. An untangling rung in T080 remains a [speculative] safeguard until real fixtures say otherwise.
3. **Input validity is the real gate** [likely]. A chart that is folded in 3D is garbage in. It flattens to garbage, and the flattener is the wrong place to discover that. It should be a typed input-validation error *upstream* of T080, with its own task (a follow-up, not this PR).
4. **Pinned distortion is the dart signal** [likely]. On a hemisphere, the free solve hides distortion by stretching the boundary 42%. The pinned solve keeps the boundary exact and compresses the interior to 0.63. A plush maker would never cut a hemisphere as one piece; they'd use gores (gore-90: compression 0.84). The pinned interior's compression is exactly the FR-021 heatmap and FR-022 dart-suggestion input. This supports D-013's design: distortion goes where the sewist can see it and act on it.
5. **The free baseline is not Option B** [solid]. It is unweighted, so its boundary drift (8.3%–990% over the non-oracle charts) is an upper bound, not what a heavy-boundary-weight solve would produce. It shows parity cannot be left to a free solve. It doesn't measure how small Option B's residual could get, and R3 already rejected Option B on contract grounds.

## Recommendations for T079–T082

Unchanged by the fixture run, which confirmed each one; ADR 0005 records them as the S7 verdict.

- **Before T079 (new, upstream)**: validate the 3D chart (consistently oriented, no folded-over faces) and return a typed input error. This spike's own first mistake is the reason.
- **T079 (boundary)**: BFF curvature with u_B = 0, then a least-squares *angle* closure, weighted by dual length: minimise Σ Δκᵢ² / Lᵢ with Lᵢ = (ℓᵢ₋₁ + ℓᵢ)/2, and run the Gauss-Newton polish in the same metric. The prototype's unweighted version is fine on these near-uniform meshes but would over-penalise short edges on real ones. Never adjust lengths. Return a typed error if closure fails, the outline self-touches, or an all-boundary triangle comes out inverted.
- **T080 (interior)**: try cotan first, then uniform (bump-h1.2 needed the fallback). Accept only a start with zero folds, because the flip-capped line search can't repair one. Add an untangling rung only if fixture runs show it's needed.
- **T082 (fallback)**: when no start is injective, return the typed error with a dart or extra-seam proposal, as D-013 says. Never relax parity.

## Limits

- **The fixture corpus is small and kind.** Three procedural shapes, ≤ 192 triangles each before refinement, smooth or piecewise flat, with symmetric seam layouts and no interior cone sharper than 13.4° (20° jittered). No plush scan, no sculpted model, no sharp interior feature, no human-drawn seams. That is why the verdict is [likely]: the fixtures confirm the synthetic findings, and the synthetic stress set is still the only evidence near the failure edge.
- The asymmetric variants are random jitter, not a shape anyone would sew; they exercise closure and give fixed cones to refine around, not realism.
- Symmetric Dirichlet is a geometric proxy, not a fabric model. "Compression 0.63" means "needs a dart", not a fabric strain measurement.
- Fold counts are for two linear starts. The prototype makes no claim about the best possible start, and it found no case where a better one was needed.
- 28–6,144 triangles, not the ~30k proxy.
- The angle closure is unweighted (see T079 above).
