# OpenPlush — Agent Guidelines

Canonical rules for every AI coding agent; `CLAUDE.md` imports this file. Lane rules: `crates/AGENTS.md` (Rust), `apps/AGENTS.md` (TypeScript), `research/AGENTS.md` (Python).

## What this repo is
A free, open-source tool that turns a 3D model into a printable plush sewing pattern: a Rust geometry engine (WASM and native CLI) plus a TypeScript/Three.js web app, all on the user's device.

## Read before working
- `.specify/memory/constitution.md`: binding. A conflicting request → stop and name the principle.
- `.specify/feature.json` names the active feature (local, git-ignored); missing → ask which feature.
- `specs/<feature>/spec.md`, `plan.md`, `data-model.md`, and the task you are executing in `tasks.md`.

## Workflow
- Every non-trivial change maps to a task ID in a `tasks.md`. No task ID → stop and ask; never invent scope.
- Trivial-change lane (no task ID): docs or typo fixes, comment fixes, dependency patch bumps, a fixture with its license entry, CI changes that don't alter gates.
- Tests first: the failing test commit, then the implementation commit.
- Never hand-edit Spec Kit files (`.specify/templates/`, `.specify/scripts/`, `.specify/extensions/`, `.claude/skills/speckit-*`); use `specify`.
- Never update golden snapshots (`fixtures/golden/`, `*.snap`) to pass a test; say the output changed and leave it to a human.
- Never skip, weaken, or delete a test to get green.

## Non-negotiables
- **Boundary**: `crates/` never touches UI, browser APIs, filesystem, clock, or threads. `apps/` and `packages/` import only `@openplush/engine-client`, never engine crates or the wasm package.
- **No silent wrong output**: a violated geometric invariant returns a typed error. Never clamp, skip, or quietly fix up bad geometry.
- **Determinism**: same input → byte-identical output, native and WASM. No `HashMap`/`HashSet` iteration on output paths; `BTreeMap`, index order, stable sorts, `f64::total_cmp`.
- **Privacy**: no runtime network calls, telemetry, analytics, or remote fonts/scripts.
- **Units**: millimetres after scaling; model units and mm meet only through a named conversion.
- **Licenses**: dependencies are MIT, Apache-2.0, BSD-2/3, ISC, Zlib, Unicode, or MPL-2.0 and pass `cargo deny` / [`scripts/check-js-licenses.sh`](scripts/check-js-licenses.sh). A new dependency category needs an ADR in `docs/adr/`.

## Engineering rules
- **SOLID**: one responsibility per module and crate; the crate table in `crates/AGENTS.md` is the map. Depend downward only. Traits only at the plug-in points the constitution names (Principle X).
- **YAGNI**: add no type, variant, `pub` item, parameter, flag, dependency, or abstraction without a task that uses it now. Nothing is `pub` only so tests can reach it.
- **DRY**: one canonical definition per concept and one home per rule. The third copy of the same boilerplate becomes a macro or generic; a rule stated twice in the docs is a docs bug.
- **Fail loudly**: typed errors; never swallow an error into a default value.
- **Small units**, gated by `scripts/check.sh`: modules ≤ 400 lines, functions ≤ 60 lines, ≤ 5 parameters.

## Comments and prose
- Code says what. A comment says only a non-obvious why: an invariant, a numeric reason, a cited paper. Delete comments that restate the code or the name.
- Doc comments are one line by default; add more only for a unit, precondition, postcondition, or error the signature doesn't show.
- No task, FR, decision, or PR numbers, spec section references, or progress notes ("so far", "comes later") in code or config. Traceability lives in commits, PRs, and `tasks.md`.
- `// INVARIANT:` comments are required where the Rust rules say so.
- PR descriptions: the template's sections as terse bullets; don't restate the diff.

## Commands
`scripts/check.sh` is the single entry point; CI calls it too.

| Task | Command |
|---|---|
| Install the pinned tools | `scripts/check.sh --setup` |
| Every gate CI runs | `scripts/check.sh` |
| Engine: boundary, hygiene, fmt, clippy, tests | `scripts/check.sh engine` |
| One step | `scripts/check.sh <boundary\|hygiene\|fmt\|clippy\|test\|deny\|wasm\|shell>` |
| See the exact commands | `scripts/check.sh --dry-run <steps>` |
| One crate's tests | `cargo test -p openplush-<crate>` |

Not yet available: `openplush-cli` fixture runs, the release `openplush-wasm` build, the web dev server.

## Definition of done
All gates for the lane you touched pass locally, any public change has its as-built docs (see Specs and docs), and the PR states edge cases (degenerate geometry, empty input, seams and boundaries, scale extremes), failure modes with their typed errors, tests and what each proves, and what was skipped.

## Specs and docs
- Specs state what and why, never how; technology choices go in `plan.md` / `research.md`.
- Tag uncertain claims in research and plan docs: `[solid]`, `[likely]`, `[speculative]`, `[verify]`.
- Absolute dates (YYYY-MM-DD). Decisions go in the plan's Decision Ledger; principle changes need an ADR (MADR).
- `specs/` records intent and decisions; `docs/` and rustdoc describe what exists. Once code lands it is the source of truth: the spec section it implements points to the as-built docs and stops changing.
- A PR that changes public behaviour updates its docs in the same PR: rustdoc for `openplush-pipeline` items; `docs/reference/` for error and warning strings, CLI, worker protocol, and project schema; `docs/explanation/architecture.md` for a new stage, crate, or dependency edge. No public change, no docs.

## Commits and PRs
Reasoning and worked examples: [docs/how-to/pull-requests.md](docs/how-to/pull-requests.md).
- Conventional Commits (`feat(flatten): …`). Every PR uses `.github/pull_request_template.md`.
- One reviewable slice: ≤ 400 changed lines of hand-written source (fixtures, generated types, snapshots, lock files, and `docs/` prose excluded). Over → split, or justify in a `Size` section by naming what is indivisible.
- A PR is a test task plus the implementation that makes it pass, test commit first.
- Never in one PR: two user stories; two crates (a thin wiring commit excepted); spec and implementation; refactor and behaviour change; chores and feature work.
- Branches: `NNN/NN-slug` for feature slices (`NN` = position in the feature, never reused; implementation slugs include the task ID) or `chore/slug`. The slug names the content, not the activity. Rename Spec Kit's `NNN-feature-slug` working branch before pushing.
- Session branches (`claude/…`, `codex/…`, `copilot/…`) are not PR branches. If you cannot push the real name, put it and the stack position in the PR title and body.
- Work that depends on anything unmerged is stacked: branch from the parent, PR base = parent, fill the `Stack` section, link the child from the parent. A stack never spans features or mixes chores with feature slices.
- Merge commits only, bottom-up, and only a PR whose base is `main`. A fix to a lower slice is merged upward; never rebase or force-push a stack under review.
