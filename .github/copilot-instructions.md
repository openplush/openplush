# Copilot Instructions

Project rules live in `AGENTS.md` (root, `crates/`, `apps/`), which Copilot loads automatically. Don't restate them here. This file exists for Copilot code review, which is guaranteed to read it.

## When reviewing a pull request
Report findings in this order. Skip praise and style nits that `rustfmt`/`prettier` already enforce.

1. **Geometry correctness** — non-manifold assumptions, missing det J > 0 checks, per-vertex where per-corner attributes are required, topology work on unwelded glTF data.
2. **Boundary** — engine crates touching UI, browser, filesystem, clock, or threads; `apps/` importing engine crates or the wasm package.
3. **Determinism** — `HashMap`/`HashSet` on output paths, unstable sorts, float comparisons without a total order, unseeded randomness.
4. **Silent failure** — clamping or skipping bad geometry instead of returning a typed error; `unwrap` on user-derived data.
5. **Privacy** — any runtime network request, telemetry, or remote asset.
6. **Tests** — missing oracle tests, weakened assertions, golden snapshot changes without an explanation.
7. **Licensing** — a new dependency outside the allowlist or without a license check.
8. **PR completeness** — feature work missing its task ID; a fix missing its failing test; edge cases, failure modes, tests, or skipped items.
9. **Design** — duplicated definitions (DRY), speculative or test-only `pub` surface (YAGNI), functions or modules doing more than one job, comments that restate the code or carry task/spec numbers.
