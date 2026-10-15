# Phase 0 Research — 001-pattern-pipeline

**Feature**: `001-pattern-pipeline` | **Date**: 2026-10-12 | **Last updated**: 2026-10-13 | **Status**: Active — R3 resolved by spec Q2; R7 added for the fabric profile presets

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
| [rajgandhi1/threecrate](https://github.com/rajgandhi1/threecrate) | Rust point-cloud + mesh library with Python bindings: registration, Poisson/BPA reconstruction, simplification, IO (PLY/OBJ/PCD/LAS/E57), wgpu GPU path. | MIT OR Apache-2.0 [solid — S6] | 46 stars, 9 forks, v0.8 | **Roadmap only** | **Not MVP.** Post-MVP native-only "scan an object → mesh" path via reconstruction. Overlaps baby_shark for simplification. |
| [microsoft/onnxruntime](https://github.com/microsoft/onnxruntime) | ML inference runtime; web build (WASM/WebGPU) [solid]; Rust bindings via `ort` [likely]. | MIT [solid] | Very mature | Yes (onnxruntime-web) | **No MVP role.** No off-the-shelf model for plush seam prediction exists that I know of [speculative] — would need a dataset. Post-MVP behind `SeamProposer` / `FeatureDetector` traits (Constitution X). |
| `faer` | Pure-Rust dense + sparse linear algebra: sparse Cholesky (supernodal/simplicial), sparse QR. | MIT [solid — S6] | Mature, active | Yes (no `rayon`) | **Core solver.** Symbolic factorization reused across iterations. |
| `i_overlay` | Pure-Rust polygon booleans, slicing, polygon outline offset; powers `geo`'s booleans. | MIT OR Apache-2.0 [solid — S6] | Active (v9.0.1 audited; `jagua-rs` → `geo` also brings v4.5.2) | Pure Rust [likely] | **Seam allowance offset + union.** Offset requires valid input polygons (simplify first); an outline-offset panic is open as iOverlay #74 → fuzz our inputs, catch and report. |
| [JeroenGar/jagua-rs](https://github.com/JeroenGar/jagua-rs) | Collision-detection engine for 2D irregular nesting: bin + strip packing, continuous rotation, reflections about chosen axes, container holes/quality zones, minimum separation; WASM target. Paper in INFORMS JoC. | MPL-2.0 | 205 stars, v1.0 | **Yes** | **Layout engine.** Strip packing = fabric roll; quality zones = fabric flaws; min separation = cutting gap; reflection axes = mirrored pieces. Optimizer: `sparrow` (same author) — MIT, but **not on crates.io**; see S6 result. MPL-2.0 is file-level copyleft — fine as a dependency of an MIT/Apache project [solid]. |
| `gltf`, `tobj` | glTF 2.0 and OBJ parsers. | `gltf` MIT OR Apache-2.0, `tobj` MIT [solid — S6] | Mature | Yes | Import. |
| `three`, `three-mesh-bvh` | Rendering; BVH raycasting for edge picking while drawing seams. | MIT [likely] | Mature | n/a (TS) | Shell. |
| `svg2pdf` (typst) | SVG → PDF conversion in Rust. | MIT OR Apache-2.0 [solid — S6] | Mature | [verify] | Tiled PDF export. |
| `insta`, `proptest` | Snapshot and property testing. | MIT/Apache [solid] | Mature | Native tests | Golden + property tests. |

## R3. Flattening Algorithm

**Decision (Q2 closed 2026-10-13 — D-013)**: **boundary-pinned isometric flattening**. Each chart's boundary edge lengths are prescribed to their 3D lengths, then the interior is minimized under symmetric Dirichlet (SLIM) with the boundary held fixed and a flip-preventing line search. ARAP remains available as an alternative interior energy. Distortion metrics from Jacobian singular values.

This is R3 Option A below, chosen because spec FR-023 made seam parity a **hard** constraint rather than a reported residual. The consequence is deliberate: all unavoidable distortion moves into the piece interior, where FR-021's heatmap shows it and FR-022's dart suggestions can relieve it — instead of into a length mismatch the sewist discovers after cutting.

| Method | Preserves | Injective? | Fit for fabric |
|---|---|---|---|
| Tutte / mean-value embedding (Tutte 1963; Floater 2003) | Nothing (convex boundary) | **Guaranteed** for disk + convex boundary [solid] | Initializer only |
| LSCM (Lévy et al. 2002) | Angles (least squares) | No guarantee [solid] | Fast preview / alternative init |
| ABF++ (Sheffer et al. 2005) | Angles | Usually | Blender-like baseline |
| BFF (Sawhney & Crane 2017) | Angles; **prescribable boundary lengths** | Usually | **Chosen for the boundary solve (D-013)** — exact seam parity |
| ARAP local/global (Liu et al. 2008) | Lengths (rigidity) | No guarantee | Good fabric proxy; needs flip guard |
| **SLIM** (Rabinovich et al. 2017) | Isometry via symmetric Dirichlet | **Locally injective** (barrier) [solid] | **Chosen for the interior solve (D-013)**, boundary held fixed |
| Bijective w/ free boundary (Smith & Schaefer 2015) | Isometry + no boundary overlap | **Globally injective** [solid] | v0.2 boundary-overlap barrier |

**Why isometric, not conformal**: fabric resists stretch far more than shear relative to how conformal maps distribute error; conformal maps preserve angles but let area drift — area drift is puckering or strain in fabric [likely].

**Seam-length parity — resolved**: both sides of a seam edge come from the *same* 3D edge, so the natural target for each side is the 3D length. That is the whole reason parity can be exact without cross-chart coupling: pin every chart boundary to its 3D lengths independently and the sides agree automatically.

- **Option A — chosen (D-013)**: boundary lengths fixed to 3D lengths (BFF prescribes exactly this; Sawhney & Crane 2017) → exact parity, interior absorbs distortion.
- **Option B — rejected**: SLIM/ARAP with a heavy boundary-length weight → small residual mismatch, reported as ease. Rejected because FR-023 now forbids handing the sewist a residual.

**Risk this introduces** [likely — S7 found no failure, see S7 result]: prescribing boundary lengths can fail two ways. The boundary polygon may not close (the prescribed lengths are not realizable for a flat disk), and the resulting map is "usually" but not provably injective, where SLIM's barrier gave a local guarantee. Constitution III requires det J > 0 on every face, so a failure must surface as a typed error with a proposal — the fallback is a dart or extra-seam proposal that makes the chart flattenable, **never** relaxing parity. Parity is a contract now, not a preference.

**Anisotropic fabric — out of scope (D-014)**: the obvious extension is to replace the identity target metric with a per-fabric tensor `M` (allowed stretch along warp vs. weft), alternating isotropic flatten → grain angle → anisotropic re-flatten. Spec FR-026 excludes it from this feature: per-direction stretch is recorded in the fabric profile and drives warnings (FR-058) only. Recorded here so a future feature does not have to rediscover the approach.

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
| **S7** | **Boundary-pinned flattening (D-013): on the fixture corpus, how often do prescribed boundary lengths fail to close, and does the result stay injective (det J > 0 everywhere)? Measure the interior distortion penalty vs. free-boundary SLIM.** Outcome decides how aggressive the dart-proposal fallback must be. Parity is not negotiable, so a bad result changes the fallback, not the contract | 2 days |
| S8 | Fabric preset values (R7) need review by someone who sews with these fabrics, or supplier spec sheets. Every number below is currently an educated guess | 0.5 day + outside review |

### S1 result: baby_shark on wasm32 (2026-10-14) ✅ closed

Spike and full results: [`research/s1_baby_shark_wasm/`](../../research/s1_baby_shark_wasm/README.md). Decision: [ADR 0001](../../docs/adr/0001-preprocessing.md) (Proposed).

**It builds and runs on `wasm32-unknown-unknown`, byte-identical to native, but it can't be adopted as published** [solid]. `baby_shark` 0.3.12's isotropic remesher, quadric decimator and both voxel meshers ran under Node, and each output hash matched the native release build. Rayon does not panic on wasm32: `rayon-core` 1.13 falls back to the current thread.

- **It fails two gates and has no features to opt out.**
  - `rayon` is unconditional (direct, and through `faer` default features), so the `cargo deny` bans fail.
  - On wasm32 it always pulls in `wasm-bindgen` and `getrandom` (`js`), and it compiles its own `#[wasm_bindgen]` exports, so boundary rule 1 fails.
  - Licenses pass.
  - Making it native-only fails the same gates (they resolve every target) and would break native/WASM byte-identity (Constitution V).
- **A small upstream change would fix it** [solid for the harness]: make those three dependencies optional, turn off `faer`'s defaults, and make three call sites sequential. In the harness that passes `cargo deny` and keeps every hash unchanged.
- **Library defects** [solid]:
  - dual-contouring output is nondeterministic with more than one thread, and contains `todo!()`;
  - decimation ignores its face floor;
  - the decimation threshold is an absolute model-units² value;
  - voxel output is an unwelded soup;
  - there is no step or yield API.
- **Implication**:
  - both targets flatten the repaired mesh, so T080 cannot assume isotropic triangles;
  - S2 must size charts at raw-input size;
  - the §3 voxel last resort is out of v0.1;
  - re-adopt only from a crates.io release that has opt-out features.

### S6 result — licenses (2026-10-14) ✅ closed

**Every candidate dependency, and every crate in their combined dependency graph, passes the Constitution IX allowlist.** No ADR or replacement needed.

Method: a throwaway crate depending on each candidate at its current version, with the feature settings the plan names (`faer` with `default-features = false`), checked with `cargo deny check licenses` (v0.20.2) against `deny.toml` as proposed in #26 (commit `217d4e0`; not yet on `main`). That covers the **full transitive graph (409 crates)**, not just the top-level licenses. `deny.toml` sets `all-features = true`, so the checked graph is a superset of what the plan's feature settings would ship. Licenses only; advisories are noted below but are not part of T006.

| Crate | Version | License | Note |
|---|---|---|---|
| `faer` | 0.24.4 | MIT | |
| `i_overlay` | 9.0.1 | MIT OR Apache-2.0 | |
| `svg2pdf` | 0.13.0 | MIT OR Apache-2.0 | |
| `threecrate` | 0.9.0 | MIT OR Apache-2.0 | Badge and manifest agree on dual |
| `jagua-rs` | 1.1.0 | MPL-2.0 | On the allowlist; file-level copyleft |
| `sparrow` (JeroenGar) | git `1fe3e31` | MIT (LICENSE file only) | **See below** |
| `baby_shark` | 0.3.12 | MIT | |
| `gltf` | 1.4.1 | MIT OR Apache-2.0 | |
| `tobj` | 4.0.5 | MIT | |

Transitive crates with a non-allowlist license (LGPL-2.1/3.0, BSL-1.0, Unlicense, 0BSD, `Apache-2.0 WITH LLVM-exception`) **all offer it as one alternative of an `OR` expression that also contains an allowlisted license**. For example, `priority-queue` (via `threecrate`) is `LGPL-3.0-or-later OR MPL-2.0`, and `r-efi` is `MIT OR Apache-2.0 OR LGPL-2.1-or-later`. cargo-deny takes the allowlisted branch, so nothing is waived.

**`sparrow` is the one finding that needs action before v0.2 adopts it** [solid]:

1. **The crates.io crate named `sparrow` is a different project**, an unrelated in-memory database (Apache-2.0, `DonaCrio/sparrow`). `sparrow = "…"` in a manifest would silently pull the wrong crate.
2. **JeroenGar's `sparrow` is not published** and must be a git dependency. `deny.toml` denies unknown git sources, so adopting it means an `allow-git` entry pinned to a commit.
3. **Its `Cargo.toml` has no `license` field.** The MIT license exists only as a `LICENSE` file. That needs a `[[licenses.clarify]]` entry in `deny.toml` keyed to the file's hash, so a future relicense fails the gate instead of passing silently.
4. **Its library is not WASM-ready as a dependency**: it is a lib + bin crate, and the library's non-optional dependencies include `clap`, `fern`, `num_cpus`, `rayon`, and `jiff`. Its manifest already gates `ctrlc` off wasm32 and enables `getrandom`'s `wasm_js`, so the author targets WASM, but `rayon` conflicts with `crates/AGENTS.md`. Upstream feature-gating, or our own optimizer over `jagua-rs`, is a question for **S4**, recorded here because the audit surfaced it.

**Advisories (not part of T006)** [solid — `cargo deny check advisories` on the same graph, 2026-10-14]: four crates are flagged **unmaintained**, and there are no vulnerability advisories. `paste` (RUSTSEC-2024-0436, a proc-macro) via **`faer` 0.24 itself** (→ `gemm` 0.19), and again via `baby_shark` → `faer` 0.22 → `gemm` 0.18. Because `faer` is the v0.1 core solver, the PR that adopts it must carry a reasoned `ignore` for this advisory (a build-time proc-macro, no runtime code) or wait for an upstream fix. `rustybuzz` (RUSTSEC-2026-0206) and `ttf-parser` (RUSTSEC-2026-0192) via `svg2pdf` → `usvg`. `bincode` 1 (RUSTSEC-2025-0141) via `threecrate`. `deny.toml` will fail on each, so the PR that adopts the dependency decides: a replacement, or a targeted `[advisories] ignore = [{ id, reason }]`. The gate stays strict until then.

**Also surfaced (not license issues; for S1/S4)** [verify; the `baby_shark` part is confirmed by S1 above]: `rayon` arrives via `baby_shark` 0.3.12 (which also brings its own `faer` 0.22 beside 0.24), `jagua-rs` 1.1.0 (default features), `sparrow`, and `threecrate`. `faer` 0.24 itself does not pull `rayon` with these features. Since `deny.toml` bans `rayon` (#26), each of these fails `cargo deny` until `rayon` is feature-gated off or the crate is replaced. That is a hard gate now, not a review item. `crates/AGENTS.md` forbids `rayon` on the WASM path, so S1 and S4 need to check whether those dependencies are optional.

### S7 result — boundary-pinned flattening (2026-10-15) ✅ closed

Prototype and full results: [`research/s7_boundary_pinned/`](../../research/s7_boundary_pinned/README.md). Verdict: [ADR 0005](../../docs/adr/0005-boundary-pinned-flattening.md).

**Corpus.** 36 runs of 28–1,728 triangles:
- every chart of the three reference seam sets in `fixtures/`: 6 sphere gores, 2 capsule caps, a slit capsule tube and a cube net, so 10 charts;
- the same 10 charts with seeded asymmetric jitter;
- 16 runs at one and two refinement levels.

The earlier synthetic set (19 charts and a 12-run sweep, up to 6,144 triangles) is still the stress evidence.

- **Closure never failed** [likely].
  - The committed charts close by symmetry alone.
  - On the jittered variants the natural gap was at most 0.04% of the perimeter. Closing it with lengths held exact took at most 0.026° of turning-angle change, and the gap shrinks when the same polyhedron is refined.
- **Injectivity never failed** [likely].
  - Every run started from an injective cotan fill; the uniform fallback was never needed.
  - Every run converged with zero flips.
  - Every outline was simple, and no all-boundary triangle was inverted.
- **The penalty follows curvature** [likely].

  | Chart | SD pinned | SD free | Min compression pinned | Min compression free | Free solve's seam-length drift |
  |---|---|---|---|---|---|
  | Developable (tube, cube net) | 4.000 | 4.000 | — | — | — |
  | 60° sphere gore | 4.02–4.03 | 4.00–4.01 | 0.88–0.91 | 0.92–0.96 | 5–8% |
  | Hemisphere cap | 5.07–5.28 | 4.15–4.23 | 0.60–0.63 | 0.85–0.87 | 41–47% |

  Developable charts have no penalty: they flatten to the exact net [solid].
- **The dart signal must be area-integrated** [likely].
  - Mean compression deficit settles under refinement. Unjittered cap: 37.3%, 36.5%, 36.3%. On a jittered polyhedron its steps shrink (cube: 0.53%, 0.44%, 0.40%).
  - The single worst triangle's compression does not settle. It falls by a near-constant step per level near fixed cones (cube: 0.963, 0.957, 0.951), and from 0.145 to 0.051 on the synthetic stress caps.
- **Implications**:
  - T079: least-squares *angle* closure, never length closure.
  - T080: a cotan start, falling back to uniform.
  - T174: folded input is rejected upstream.
  - T082: fires only on an actual failure, as a typed error plus a proposal; no pre-emptive splitting.
  - T105: thresholds on area-integrated compression; per-triangle σ stays the heatmap's input.
- **Limits** [solid]:
  - three coarse procedural shapes with symmetric seams;
  - no interior cone sharper than 20°;
  - no scan or sculpt, and no human-drawn seams.

  Re-run S7 when such a fixture lands. Any failure reopens ADR 0005.

## R7. Fabric Profiles (FR-050, D-018)

Seven shipped presets, all editable, all persisted in the project. The schema is in [data-model.md §6a](./data-model.md#6a-fabric-profiles).

> **Every number in this table is `[verify]`** — plausible craft-fabric values, not measured ones, and not sourced from supplier data. Spike S8 exists to replace them with reviewed figures. They are good enough to build and test against (the feature's correctness does not depend on their accuracy, only on the plumbing that carries them), and wrong enough that shipping them unreviewed would mislead a maker.

| Preset | Category | Stretch grain / cross / bias | Nap | Pile (mm) | Thick (mm) | Frays | Allowance (mm) | Roll (mm) |
|---|---|---|---|---|---|---|---|---|
| Quilting cotton | woven | 0 / 0 / 5% | no | 0 | 0.3 | **yes** | 6 | 1120 |
| Craft felt | felt | 0 / 0 / 0 | no | 0 | 1.2 | no | 6 | 1800 |
| Polar fleece | knit | 5 / 25 / 30% | no | 1 | 2.0 | no | 6 | 1500 |
| Minky / cuddle | knit (pile) | 10 / 40 / 45% | **yes** | 3 | 2.5 | no | 10 | 1500 |
| Velboa | knit (pile) | 5 / 30 / 35% | **yes** | 1.5 | 1.5 | no | 6 | 1500 |
| Short faux fur | knit (pile) | 5 / 30 / 35% | **yes** | 8 | 3.0 | no | 10 | 1500 |
| Long faux fur | knit (pile) | 5 / 30 / 35% | **yes** | 30 | 4.0 | no | 12 | 1500 |

### Why these fields and not others

- **Category** drives the rotation set (FR-053) and which warnings even apply. `pile` implies nap, mirrored printing (FR-054), and pile-scaled allowance (FR-055).
- **Per-direction stretch** is recorded but does **not** steer flattening (D-014). Its only job in 001 is FR-058's cross-seam mismatch warning. Knit-backed fur stretching ~30% crosswise against a 0% woven is exactly the seam that fights back while sewing.
- **Pile length** scales the default allowance and triggers the small-part warning (FR-055). The provisional 3× rule means long faux fur (30 mm) warns on any piece or detail under 90 mm — which is most plush faces. That is informative, not a bug, but it is the strongest argument for making the multiple configurable.
- **Thickness** is recorded for the "thick backing on a small plush" edge case and for future thickness compensation, which is explicitly out of scope here. It drives no geometry in 001.
- **Frays** gates FR-057's zero-allowance option. Felt is the case that motivates it: edge-stitched or appliqué felt pieces need no allowance at all, and adding one makes the piece wrong.
- **Roll width** bounds FR-062's layout. Note that pile fabrics are commonly sold at 1500 mm while quilting cotton is ~1120 mm, so the same pattern can fit one and not the other.

### Deliberately absent

Weight (gsm), fibre content, wash behaviour, and price. None of them change a pattern, and each would imply the tool knows more about the fabric than it does.
