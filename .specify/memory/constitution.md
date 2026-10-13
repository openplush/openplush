# OpenPlush Constitution

**Status**: Draft — pending maintainer ratification

A free, open-source tool that turns a 3D model into a sewable plush pattern. These principles are binding on every spec, plan, task, and PR. Where another document conflicts, this one wins.

## Core Principles

### I. Engine / Shell Separation (NON-NEGOTIABLE)
All geometry lives in pure Rust library crates with **zero** dependency on UI, rendering, browser APIs, the filesystem, wall-clock time, or native threads. The web app, CLI, and future bindings are thin shells over **one** versioned engine API (`openplush-pipeline`).
- A geometry change MUST NOT require editing `apps/` or `packages/`.
- A UI change MUST NOT require editing `crates/`.
- Shells depend on `openplush-pipeline` only — never on internal engine crates.

*Enforcement:* CI boundary check over `cargo metadata` (no `wasm-bindgen` / `web-sys` / `js-sys` / `std::fs` use in `crates/`); `cargo deny` bans.

### II. CLI Parity
Every engine capability is reachable from `openplush-cli` as file-in / file-out. If it can't run headless, it isn't done. Bug reports are reproduced as CLI invocations; golden tests run through the CLI path.

### III. Geometric Invariants Are Contracts
Each pipeline stage declares pre- and postconditions — in the type system where possible (typestate), debug assertions otherwise — and in its contract doc:

| Stage boundary | Invariant |
|---|---|
| Post-repair | Manifold-with-boundary, consistently oriented, no degenerate or duplicate faces |
| Pre-flatten | Every chart is a topological disk (connected, one boundary loop, χ = 1) |
| Post-flatten | Injective: every triangle has det J > 0; no boundary self-overlap |
| Post-pattern | Paired seam sides match in length within tolerance; cut outline is a simple polygon |

Violations fail with a typed error. A silently wrong pattern is the worst possible outcome — worse than a crash — because the user discovers it after cutting fabric.

### IV. Test-First With Geometric Oracles
Algorithms land with tests first, using oracles independent of the implementation:
- **Developable oracles:** cylinder strips, cones, and folded flat sheets must flatten with distortion ≈ 0.
- **Invariant oracles:** injectivity, seam-length parity, area conservation within stated tolerances.
- **Property tests** (`proptest`) on mesh construction, repair, and cutting.
- **Golden fixtures** (snapshot SVG + metrics JSON) for regression; changes require visual review in the PR.

### V. Determinism
Same input + same settings ⇒ byte-identical output, native and WASM. No iteration over unordered hash containers on output-affecting paths (`BTreeMap` / index order instead); seeded PRNG only; fixed solver orderings; explicit epsilon policy documented in `data-model.md`.

### VI. Physical Units at the Boundary
Mesh units are declared (or inferred and confirmed by the user) at import. After scaling, everything is millimetres internally. Seam allowance, notch spacing, and page geometry are physical quantities; distortion metrics are dimensionless.

### VII. Explicit Repair, Never Silent Mutation
The engine never alters user geometry without producing a `RepairReport` that lists every change and where it occurred (renderable in the UI). Destructive repairs (e.g., voxel remeshing) are opt-in. Engine-proposed seams are proposals until the user accepts them.

### VIII. Local-First, Private by Default
All processing runs on the user's device. No account, no server dependency, no telemetry, no analytics. The hosted web build is static files. A user's model never leaves their machine.

### IX. Free, Open, Contributor-First
- Project code: **MIT OR Apache-2.0** (dual, Rust-ecosystem convention). No CLA.
- Dependencies must pass the license allowlist: MIT, Apache-2.0, BSD-2/3, ISC, Zlib, Unicode, MPL-2.0. Nothing GPL/AGPL/LGPL in the shipped artifact.
- Two contribution lanes with separate onboarding: **engine** (Rust/geometry) and **shell** (TypeScript/Three.js). Plus low-barrier lanes: **fixtures** (test models) and **docs**.
- A new contributor builds and runs all tests with ≤ 3 documented commands.

### X. Baseline Before Cleverness
Heuristics and ML (ONNX) plug in behind traits only after a deterministic geometric baseline exists and is measured against the fixture corpus. ML is never required to produce a valid pattern, and no model weights ship in the core artifact.

## Technical Constraints
- Engine: Rust stable, MSRV pinned in `rust-toolchain.toml`; `wasm32-unknown-unknown` builds in CI on every PR.
- Numerics: `f64` throughout geometry; `f32` only at the render boundary.
- Shell: TypeScript `strict`; the engine runs in a Web Worker — the UI thread never blocks on geometry.
- Hosting: static (GitHub Pages or equivalent); installable PWA for offline use.
- Accessibility: color-blind-safe palettes for all heatmaps (e.g., viridis/cividis); keyboard-operable non-3D UI.

## Development Workflow
- SDD flow: `specify → clarify → plan → tasks → implement`. No implementation PR without a linked spec ID and task ID, except the **trivial-change lane**: docs or typo fixes, comment fixes, dependency patch bumps, new fixtures with license entries, and CI changes that don't alter gates.
- AI agent rules live in `AGENTS.md` (root plus lane files). Tool-specific files (`CLAUDE.md`, `.github/copilot-instructions.md`) only import those rules or add tool-specific deltas — never a second copy.
- PRs touching `crates/`: `fmt`, `clippy -D warnings`, native tests, WASM tests, golden diff, determinism check (native vs. WASM output hash), boundary check.
- PRs touching `apps/` or `packages/`: typecheck, unit tests, Playwright smoke on fixture models.
- Any decision that changes a principle or a public contract gets an ADR (MADR format) in `docs/adr/`.
- Docs follow Diátaxis (`tutorials/`, `how-to/`, `reference/`, `explanation/`).

## Governance
Amendments require a PR containing the change, an ADR, and a version bump: MAJOR = principle removed or redefined; MINOR = principle added or materially expanded; PATCH = clarification. Maintainers ratify by approving review.

**Version**: 0.1.0 | **Ratified**: TODO | **Last Amended**: 2026-10-13
