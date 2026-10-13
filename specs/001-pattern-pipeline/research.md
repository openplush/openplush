# Phase 0 Research — 001-pattern-pipeline

**Feature**: `001-pattern-pipeline` | **Date**: 2026-10-12 | **Status**: Draft

Confidence tags: **[solid]** consensus / verified · **[likely]** reasonable inference · **[speculative]** guess · **[verify]** check before committing.

## R1. Stack Decision

**Decision**: Rust engine (Cargo workspace) compiled to **WASM** for the browser and **native** for the CLI; **TypeScript + Three.js** web shell; engine hosted in a **Web Worker**. Python bindings (PyO3/maturin) deferred post-MVP for a Blender add-on and research use.

**The brief's two options are a false binary.** Three of the four libraries named for this project are Rust crates; the natural architecture is a Rust core with a TS shell, not TS-vs-Python.

### Evaluation

| Criterion | TS + Three.js only | Python (Trimesh/NumPy/SciPy) | Blender add-on | **Rust → WASM + TS shell** | C++ → Emscripten + TS |
|---|---|---|---|---|---|
| Sparse linear algebra (the flattening bottleneck) | Weak; no mature sparse Cholesky in JS [likely] | Excellent (SciPy, libigl bindings) [solid] | Good (NumPy/SciPy bundled) | Good: `faer` sparse Cholesky + QR, pure Rust [solid] | Excellent (Eigen, libigl) [solid] |
| End-user distribution (crafters, zero install) | Excellent | Poor: installer or Pyodide (heavy) [likely] | Poor: requires learning Blender | Excellent (static site / PWA) | Excellent |
| Contributor pool | Largest | Large, research-friendly | Niche | Shell: large (TS). Engine: smaller (Rust) | Small; build pain deters |
| Dev velocity for geometry | Low (tooling gaps) | Highest | High | Medium | Low |
| Runtime perf (50k-tri iterative solve) | Marginal | Good (vectorized) | Good | Good, ~native in WASM minus SIMD/threads [likely] | Good |
| Determinism across platforms | Medium | Medium | Low (Blender version churn) | High (controlled f64 path) | High |
| Reuse of named libs | None | threecrate (Py bindings) | None | baby_shark, threecrate, `ort` | None |
| Seam-marking UI for free | No | No | **Yes (biggest pro)** | No | No |

**Rationale**
1. The product's users are sewists, not developers. Zero-install web is non-negotiable → Python and Blender lose as the *primary* surface.
2. Flattening is iterative sparse solves. Pure TS lacks a credible sparse direct solver → TS-only loses on correctness and speed.
3. Rust gives one engine for web, CLI, and (later) Python, while the TS shell keeps the large web contributor pool away from Rust entirely (Constitution I).
4. Maintainer already knows Rust; the Rust learning cost lands on engine contributors only.

**Rejected**
- *C++/Emscripten*: best geometry ecosystem, worst contributor ergonomics and memory safety. Not worth it when `faer` covers the solver need.
- *Blender add-on as primary*: keep as a post-MVP shell over `openplush-py`. Note that add-ons using Blender's Python API are generally treated as GPL-bound [likely] — the add-on would be its own GPL-compatible package; the MIT/Apache engine stays compatible.

**Velocity mitigation**: `research/` holds non-shipping Python notebooks (libigl, SciPy) for prototyping energies. Their outputs become oracle fixtures the Rust port must match. Prototype in Python, ship in Rust.

**WASM constraints to plan around** [likely]
- Threads need `SharedArrayBuffer` → cross-origin isolation headers, which GitHub Pages can't set natively. MVP is single-threaded WASM.
- Long operations expose step-wise sessions (`step()` returns progress) so the worker can report progress and honor cancellation between iterations — no threads required.
- `faer` default features pull in `rayon`, which breaks `wasm32-unknown-unknown`; build with `default-features = false` [solid — documented in downstream crates].

## R2. Library Audit

| Library | What it actually is | License | Maturity (2026-10-12) | WASM | Verdict |
|---|---|---|---|---|---|
| [foundlings/qew](https://github.com/foundlings/qew) | **Not a geometry library.** Offline-first, git-backed work ledger: items, dependency graph, readiness queue, mirrored to GitHub/Forgejo/GitLab; agent hooks for Claude/Codex/Cursor. Python 3.15+, no runtime deps. | Apache-2.0 | 2 stars, 0 forks, ~700 commits; single maintainer | n/a | **Maintainer tooling, not engine.** Fits SDD gating (drafts become approved work only on a human's quoted approval). Use for `tasks.md` → work items; keep GitHub Issues as the contributor-facing view. Never required for contributors. Bus-factor risk: keep `tasks.md` canonical in the repo. |
| [dima634/baby_shark](https://github.com/dima634/baby_shark) | Pure-Rust geometry processing: voxel remeshing, volume booleans, offsetting, isotropic remeshing, decimation, deformation. IO: STL, OBJ, PLY. Corner-table mesh. | MIT | 97 stars, 9 forks | Unverified — **spike** [verify] | **Preprocessing only.** Isotropic remeshing before flattening (triangle quality drives solver conditioning); decimation for preview; voxel remesh as opt-in last-resort repair. Not our core mesh type (no per-corner attributes, no seam flags, no glTF) → adapter at the boundary. |
| [rajgandhi1/threecrate](https://github.com/rajgandhi1/threecrate) | Rust point-cloud + mesh library with Python bindings: registration, Poisson/BPA reconstruction, simplification, IO (PLY/OBJ/PCD/LAS/E57), wgpu GPU path. | MIT (badge says MIT/Apache-2.0) [verify] | 46 stars, 9 forks, v0.8 | **Roadmap only** | **Not MVP.** Post-MVP native-only "scan an object → mesh" path via reconstruction. Overlaps baby_shark for simplification. |
| [microsoft/onnxruntime](https://github.com/microsoft/onnxruntime) | ML inference runtime; web build (WASM/WebGPU) [solid]; Rust bindings via `ort` [likely]. | MIT [solid] | Very mature | Yes (onnxruntime-web) | **No MVP role.** No off-the-shelf model for plush seam prediction exists that I know of [speculative] — would need a dataset. Post-MVP behind `SeamProposer` / `FeatureDetector` traits (Constitution X). |
| `faer` | Pure-Rust dense + sparse linear algebra: sparse Cholesky (supernodal/simplicial), sparse QR. | MIT [likely] | Mature, active | Yes (no `rayon`) | **Core solver.** Symbolic factorization reused across iterations. |
| `i_overlay` | Pure-Rust polygon booleans, slicing, polygon outline offset; powers `geo`'s booleans. | MIT [likely] | Active (v6) | Pure Rust [likely] | **Seam allowance offset + union.** Offset requires valid input polygons (simplify first); an outline-offset panic is open as iOverlay #74 → fuzz our inputs, catch and report. |
| [JeroenGar/jagua-rs](https://github.com/JeroenGar/jagua-rs) | Collision-detection engine for 2D irregular nesting: bin + strip packing, continuous rotation, reflections about chosen axes, container holes/quality zones, minimum separation; WASM target. Paper in INFORMS JoC. | MPL-2.0 | 205 stars, v1.0 | **Yes** | **Layout engine.** Strip packing = fabric roll; quality zones = fabric flaws; min separation = cutting gap; reflection axes = mirrored pieces. Optimizer: `sparrow` (same author) [verify license]. MPL-2.0 is file-level copyleft — fine as a dependency of an MIT/Apache project [solid]. |
| `gltf`, `tobj` | glTF 2.0 and OBJ parsers. | MIT/Apache [likely] | Mature | Yes | Import. |
| `three`, `three-mesh-bvh` | Rendering; BVH raycasting for edge picking while drawing seams. | MIT [likely] | Mature | n/a (TS) | Shell. |
| `svg2pdf` (typst) | SVG → PDF conversion in Rust. | MIT/Apache [verify] | Mature | [verify] | Tiled PDF export. |
| `insta`, `proptest` | Snapshot and property testing. | MIT/Apache [solid] | Mature | Native tests | Golden + property tests. |

## R3. Flattening Algorithm

**Decision (pending Q2)**: injective initialization → symmetric-Dirichlet minimization (SLIM) with flip-preventing line search; ARAP available as an alternative energy. Distortion metrics from Jacobian singular values.

| Method | Preserves | Injective? | Fit for fabric |
|---|---|---|---|
| Tutte / mean-value embedding (Tutte 1963; Floater 2003) | Nothing (convex boundary) | **Guaranteed** for disk + convex boundary [solid] | Initializer only |
| LSCM (Lévy et al. 2002) | Angles (least squares) | No guarantee [solid] | Fast preview / alternative init |
| ABF++ (Sheffer et al. 2005) | Angles | Usually | Blender-like baseline |
| BFF (Sawhney & Crane 2017) | Angles; **prescribable boundary lengths** | Usually | Exact seam parity option (Q2) |
| ARAP local/global (Liu et al. 2008) | Lengths (rigidity) | No guarantee | Good fabric proxy; needs flip guard |
| **SLIM** (Rabinovich et al. 2017) | Isometry via symmetric Dirichlet | **Locally injective** (barrier) [solid] | **Primary** |
| Bijective w/ free boundary (Smith & Schaefer 2015) | Isometry + no boundary overlap | **Globally injective** [solid] | v0.2 boundary-overlap barrier |

**Why isometric, not conformal**: fabric resists stretch far more than shear relative to how conformal maps distribute error; conformal maps preserve angles but let area drift — area drift is puckering or strain in fabric [likely].

**Seam-length parity (Q2)**: Both sides of a seam edge come from the *same* 3D edge, so the natural target for each side is the 3D length. Option A: BFF with boundary lengths fixed to 3D lengths → exact parity, interior absorbs distortion. Option B: SLIM/ARAP with heavy boundary-edge length weight per chart → small residual mismatch, reported and marked as ease. No cross-chart coupling needed in either case.

**Anisotropic fabric (v0.2)**: replace the identity target metric with a per-fabric tensor `M` (allowed stretch along warp vs. weft). Because `M` depends on piece rotation, alternate: isotropic flatten → derive grain angle → anisotropic re-flatten (1–2 rounds) [likely sufficient].

**Distortion reported** (per triangle, singular values σ₁ ≥ σ₂ of J): max stretch σ₁, max compression σ₂, area ratio σ₁σ₂, conformal ratio σ₁/σ₂. Interpreted against the active fabric's tolerances, not fixed constants.

## R4. Plush-Specific Geometry

- **Stuffing inflates pieces.** Pressure bulges every panel between seams; a pattern flattened from the target surface sews up rounder and puffier than the model, and concave regions only survive where a seam runs through them [solid — the core finding of the plush/inflatable design literature].
  - Mori & Igarashi 2007, *Plushie: An Interactive Design System for Plush Toys* (SIGGRAPH) — sketch-based design with inflation simulation.
  - Igarashi et al. 2008, *Pillow: Interactive Flattening of a 3D Model for Plush Toy Design* (Smart Graphics) — user-drawn seams, flattening, simple mass-spring stuffing model, sewing-cost estimate from seam length. Closest academic analogue to this project.
  - Skouras et al. 2014, *Designing Inflatable Structures* (SIGGRAPH) — inverse design: compute flat panels whose *inflated* shape matches the target.
- **Seam optimization**: OptCuts (Li, Kaufman, Kim, Solomon, Sheffer — SIGGRAPH Asia 2018, joint cuts + distortion); Variational Surface Cutting (Sharp & Crane 2018); Seamster (Sheffer & Hart 2002, low-visibility seams); D-Charts (Julius, Kraevoy, Sheffer 2005, quasi-developable segmentation — plush panels are near-developable, strong fit for auto-seams).
- **Cut graphs for non-disk charts**: tree-cotree (Eppstein 2003); greedy shortest homotopy/homology generators (Erickson & Whittlesey 2005); optimal cutting to a disk is NP-hard in general (Erickson & Har-Peled 2004) → heuristics are expected.
- **Non-manifold repair**: Guéziec, Taubin, Lazarus, Horn 2001, *Converting Sets of Polygons to Manifold Surfaces by Cutting and Stitching* (IEEE TVCG).
- **Direction fields (nap/grain)**: Knöppel, Crane, Pinkall, Schröder 2013, *Globally Optimal Direction Fields* — sparse linear solve, handles user alignment constraints.
- **Hole filling**: Liepa 2003, *Filling Holes in Meshes*.

## R5. Parity Sources

- AlternativeTo listing: Plushify is free but proprietary, web-based; user draws seams in 3D, gets a PDF pattern; built-in quality visualization indicates where darts or more seams help; user images can be placed on the model as markings; any size with a human-scale preview; print auto-scaled — <https://alternativeto.net/software/plushify/about/>
- FileInfo: `.plush` project = JSON describing the model plus the user's edits; OBJ in, PDF out — <https://fileinfo.com/extension/plush>
- Pillow paper — <https://www-ui.is.s.u-tokyo.ac.jp/~takeo/papers/yuki_sg08_pillow.pdf>

## R6. Open Research Spikes (time-boxed, before `/tasks`)

| Spike | Question | Box |
|---|---|---|
| S1 | Does `baby_shark` (isotropic remesh, decimate) build and run on `wasm32-unknown-unknown`? | 0.5 day |
| S2 | SLIM in Rust + `faer` on a 50k-tri chart in WASM: wall time per iteration vs. native | 2 days |
| S3 | `i_overlay` offset robustness: fuzz 10k random valid piece outlines with mixed per-edge allowances | 1 day |
| S4 | `jagua-rs` WASM bundle size + discrete rotation sets {0°} / {0°,180°} support | 0.5 day |
| S5 | `svg2pdf` WASM viability vs. generating PDF directly | 0.5 day |
| S6 | Licenses: confirm `faer`, `i_overlay`, `sparrow`, `svg2pdf`, threecrate (MIT vs. dual) | 0.5 day |
