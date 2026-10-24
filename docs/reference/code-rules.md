# Code rules for the engine

<!-- index: the rules for Rust code in `crates/` and `bindings/`, each with its reason. -->

The rules for Rust code in `crates/` and `bindings/`, each with the reason behind it. Agents read the same rules as one-line directives in [`crates/AGENTS.md`](https://github.com/openplush/openplush/blob/main/crates/AGENTS.md); the lints that enforce some of them, and why each lint exists, are in [Checks](checks.md).

## Entries

In the order `crates/AGENTS.md` lists them. "Constitution" and a numeral name a principle of the [project constitution](https://github.com/openplush/openplush/blob/main/CONSTITUTION.md).

### Where code goes

- **Put code in the crate that owns its concern**, from the [ownership table](crates/README.md#which-crate-owns-what). One responsibility per crate keeps each one small enough to test against its own oracle.
- **Depend downward only**, along the crate graph in the [plan](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/plan.md#crate-dependency-dag-enforced). A sideways edge needs an ADR, because two crates that reach into each other can no longer be changed or tested apart.
- **`bindings/*` depend on `openplush-pipeline` only.** The pipeline is the one public engine API, so the command line and the browser run the same operations with the same errors (Constitution I and II); `scripts/check-boundary.sh` enforces it.

### Rust rules

- **No panic on data that comes from the user**: no `unwrap`, `expect`, `panic!` or panicking index on it. A panic cannot be caught on wasm32 (see [WASM constraints](#wasm-constraints)), so a bad file would crash the maker's session instead of returning an error. An internal invariant may still unwrap, with an `// INVARIANT:` comment naming it, so a reviewer can see why it cannot fire.
- **One `thiserror` error enum per crate, carrying element IDs** (`FaceId`, `EdgeId`, `VertexId`). A violated invariant is a typed error, never a quiet fix (Constitution III), and the IDs let the app highlight the part of the model at fault.
- **Newtype IDs in every API, never a raw `usize` or `u32`.** Passing a face index where an edge index was meant is then a compile error, not a wrong answer.
- **`f64` for geometry, with tolerances from `openplush_geom::epsilon`.** Model-space tolerances scale with the model's bounding-box diagonal, so the same shape behaves the same at any unit; physical ones, such as seam parity, are in millimetres because fabric is. A literal `1e-9` in algorithm code hides a scale assumption.
- **`#![forbid(unsafe_code)]`** unless an ADR says otherwise; see [Checks](checks.md#workspace-lints-clippy).

### Rust design

- **Make invalid states unrepresentable**: private fields and one validating constructor, as `Proposal::new` does, and a type per stage boundary (Constitution III). A value that exists is then valid, so later code does not re-check it. `ManifoldMesh` is the example: only repair can build one, so unrepaired data cannot reach the flattener (Constitution VII).
- **`pub(crate)` by default**; `pub` only for what another crate calls. The public surface stays the API and nothing else, and `unreachable_pub` holds the line.
- **Borrow in, own out.** The caller decides whether it needs a copy; a `clone()` added only to satisfy the borrow checker hides a cost and usually a design problem.
- **Enums over `bool` flags and strings**, and `#[non_exhaustive]` on public enums that will grow. A call site then says what it means, and adding a variant does not break another crate's `match`.
- **Iterators over index loops**, unless the index is the point. There is no index to get wrong or to panic on.
- **Doc comments are one line**, saying what the signature doesn't; only `openplush-pipeline` documents every item, because it is the public API.
- **Small units**: modules ≤ 400 lines, functions ≤ 60 lines, ≤ 5 parameters, enforced by `scripts/check.sh`. Split by responsibility, not convenience. A function already over the limit carries `#[expect(clippy::too_many_lines, reason = "…")]` until it is split, and no new one may.

### WASM constraints

The app runs the engine in a browser worker, on the maker's own machine (Constitution VIII), so every crate in `crates/` builds for `wasm32-unknown-unknown`.

- **`usize` is 32 bits there**, in tests too: no `usize` value, literal or shift past `u32::MAX`; use `u64` where the value needs it. `scripts/check.sh wasm-check` builds every test binary for wasm32, because `cargo check` and clippy never reach the code generation that reports an overflowing shift.
- **No `std::fs`, `std::thread`, `std::time::Instant`, `rayon` or OS randomness**, and `faer` with `default-features = false`. The target has no file system, threads or clock, and a clock or random seed would also make output differ between runs (Constitution V).
- **Long operations are step-wise sessions** (`step() -> Progress`), and no loop runs longer than about 50 ms without a point where it can yield. The worker has one thread, so a loop that never yields freezes progress reports and cancelling.
- **A panic cannot be caught**: with `panic = "abort"` it traps the instance. No engine code relies on `catch_unwind`; a panic is a bug, which the app reports as a worker crash (`EngineCrashed`) and the command line as exit status 101 ([ADR 0009](../adr/0009-wasm-panic-is-a-worker-crash.md)). `openplush-wasm` installs a panic hook that forwards the message to the worker's console.

### Determinism

The same input gives byte-identical output, natively and on wasm32 (Constitution V).

- **No `HashMap`, `HashSet` or unstable sort in code that affects output**; the lints and their opt-out list are in [Checks](checks.md#workspace-lints-clippy).
- **Sort with a total order (`total_cmp`), breaking ties by ID.** Floating-point comparison has no order for NaN, and equal keys would otherwise land in whatever order the sort leaves them.
- **Property tests run with fixed seeds in CI**, and a failing seed becomes a committed regression case, so a failure found once stays found.

### Geometry gotchas

How a mesh is stored, and what that means for code that walks it, is in [Meshes](../explanation/meshes.md); why a chart must be a disk is in [Seams](../explanation/seams.md); what the flattener requires of a chart is in [Flattening](../explanation/flattening.md).

### Tests

- **Each algorithm has an oracle test independent of its implementation** (Constitution IV), such as a developable surface flattening with near-zero distortion. A test that reuses the algorithm's own maths passes when both are wrong.
- **Mesh data over about 20 triangles comes from `openplush-testkit` fixtures**, loaded by name, not inline literals a reviewer cannot read.
- **Golden output uses `insta`, and only a person accepts a changed snapshot.** A changed snapshot is changed output; accepting it to get green would turn a regression into the expected result.

### Solver notes

- **Reuse the symbolic factorization across iterations** and refactor numerically only: the sparsity pattern does not change between iterations, so the symbolic step is work that would be repeated for nothing.
- **Keep the solver's ordering fixed**, because a different ordering changes the rounding and so the output's bytes.
