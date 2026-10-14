# Spike S7: boundary-pinned flattening (D-013), synthetic prototype

**Status (2026-10-14): preliminary.** This is a reference prototype run on a **synthetic** corpus. T007 asks for results "on every fixture", and `fixtures/` does not exist yet, so T007 stays open. Running `run.py` on real charts is the remaining step.

Non-shipping research code (plan.md: `research/` holds reference implementations, not product code). It answers the structural questions now so the Rust flattener (T079 boundary, T080 interior, T082 fallback) is designed around the right failure mode.

> **Correction (2026-10-14, same PR).** The first version of this spike reported that three stress charts had "no injective start" and that failures grew with resolution. An adversarial review found those charts were **folded 3D input**: the cap's polar angle passed 180°, so the surface wrapped past the south pole. A second, smaller defect was the radial lobe warp, which inverted triangles whenever lobe depth × lobe count ≳ 1, at every resolution. Both are fixed: the generators now refuse folded caps, lobes come from a provably one-to-one conformal warp, and an independent validity oracle (`test_s7.Validity`) checks every chart before it is flattened. The findings below are from the corrected corpus.

## The question

D-013 pins every chart's boundary edges to their 3D lengths, so the two sides of a seam always match, and puts all distortion in the interior. Research R3 lists two risks:

1. The prescribed lengths may not **close** into a flat outline.
2. The map may not be **injective** (some triangle folds over, det J ≤ 0), where free-boundary SLIM had a local guarantee.

## Method

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

Oracle tests (`test_s7.py`, 19 tests):
- **Input validity** on every chart: sphere charts face outward, and no two adjacent faces fold over each other.
- Euler characteristic and Gauss–Bonnet, both for the 3D chart and for the flattened boundary.
- A finite-difference gradient check.
- Exact closure with exact lengths.
- Touching and clockwise outlines are rejected.
- Quadratic roots match `numpy.roots`, including the near-linear case.
- A flat disk must map to itself, and must converge rather than run out of iterations.
- A bent plane (developable) must flatten almost isometrically.

Run it: `pip install -r requirements.txt`, then `python3 -m unittest -v` and `python3 run.py` (about 90 s, most of it the resolution sweep). The output is deterministic, and `results.md` records the Python/numpy/scipy versions it was generated with.

## Results

Full table in [`results.md`](./results.md). Read the columns as:
- **SD mean**: 4.0 means isometric (no distortion); higher is worse.
- **Stretch / compress**: the largest and smallest singular values. Compression 0.63 means the fabric there must gather into 63% of its 3D extent.
- **Stop**: why the optimizer stopped, with the final gradient inf-norm. `gtol` means the gradient fell below 1e-8 of its starting value. `stalled` means the energy stopped changing at machine precision. Both count as converged; `maxiter` or `linesearch` would be reported as a failure, and none occurred.

### Corpus (14 charts) and stress set (5 charts)

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

## Findings

1. **Closure is not the failure mode** [likely; synthetic]. With u_B = 0 and a harmonic interior factor, the flattened metric is flat on a disk, so in the smooth limit the boundary closes (Sawhney & Crane 2017). The discrete gap is what remains, and it shrank with refinement in every sweep here. Pinning only lengths leaves every turning angle free, and the angle fix needed was tiny everywhere. R3's "fails to close" risk can be downgraded. The closure step still belongs in T079, as a least-squares angle correction rather than BFF's length correction.
2. **On valid input, injectivity held everywhere** [likely; synthetic]. On every geometrically valid chart tried (up to 6,144 triangles and 55 reflex corners) a linear start was injective, and the optimizer kept it that way. **The injectivity failures seen in this spike's first version came from folded input**, not from the method. An untangling rung in T080 remains a [speculative] safeguard until real fixtures say otherwise.
3. **Input validity is the real gate** [likely]. A chart that is folded in 3D is garbage in. It flattens to garbage, and the flattener is the wrong place to discover that. It should be a typed input-validation error *upstream* of T080, with its own task (a follow-up, not this PR).
4. **Pinned distortion is the dart signal** [likely]. On a hemisphere, the free solve hides distortion by stretching the boundary 42%. The pinned solve keeps the boundary exact and compresses the interior to 0.63. A plush maker would never cut a hemisphere as one piece; they'd use gores (gore-90: compression 0.84). The pinned interior's compression is exactly the FR-021 heatmap and FR-022 dart-suggestion input. This supports D-013's design: distortion goes where the sewist can see it and act on it.
5. **The free baseline is not Option B** [solid]. It is unweighted, so its boundary drift (8.3%–990% over the non-oracle charts) is an upper bound, not what a heavy-boundary-weight solve would produce. It shows parity cannot be left to a free solve. It doesn't measure how small Option B's residual could get, and R3 already rejected Option B on contract grounds.

## Recommendations for T079–T082

- **Before T079 (new, upstream)**: validate the 3D chart (consistently oriented, no folded-over faces) and return a typed input error. This spike's own first mistake is the reason.
- **T079 (boundary)**: BFF curvature with u_B = 0, then a least-squares *angle* closure, weighted by dual length: minimise Σ Δκᵢ² / Lᵢ with Lᵢ = (ℓᵢ₋₁ + ℓᵢ)/2, and run the Gauss-Newton polish in the same metric. The prototype's unweighted version is fine on these near-uniform meshes but would over-penalise short edges on real ones. Never adjust lengths. Return a typed error if closure fails, the outline self-touches, or an all-boundary triangle comes out inverted.
- **T080 (interior)**: try cotan first, then uniform (bump-h1.2 needed the fallback). Accept only a start with zero folds, because the flip-capped line search can't repair one. Add an untangling rung only if fixture runs show it's needed.
- **T082 (fallback)**: when no start is injective, return the typed error with a dart or extra-seam proposal, as D-013 says. Never relax parity.

## Limits

- **Synthetic surfaces only.** Real models have sharp features, anisotropic triangles, and seam layouts drawn by people. The fixture run is what closes T007.
- Symmetric Dirichlet is a geometric proxy, not a fabric model. "Compression 0.63" means "needs a dart", not a fabric strain measurement.
- Fold counts are for two linear starts. The prototype makes no claim about the best possible start, and it found no case where a better one was needed.
- 384–6,144 triangles, not the ~30k proxy.
- The angle closure is unweighted (see T079 above).
