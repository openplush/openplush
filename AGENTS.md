# OpenPlush — Agent Guidelines

Canonical instructions for every AI coding agent. Claude Code reads this through `CLAUDE.md` (`@AGENTS.md`); Copilot, Codex, and Cursor read it directly. Lane rules live in `crates/AGENTS.md` and `apps/AGENTS.md`.

## What this repo is
A free, open-source tool that turns a 3D model into a printable plush sewing pattern. Rust geometry engine (compiled to WASM and a native CLI) plus a TypeScript/Three.js web app. Everything runs on the user's device.

## Read before working
- `.specify/memory/constitution.md` — binding principles. If a request conflicts with it, stop and say which principle.
- `.specify/feature.json` — names the active feature directory (local, git-ignored). If it's missing, ask which feature.
- `specs/<feature>/spec.md`, `plan.md`, `data-model.md` — what is being built and how.
- `specs/<feature>/tasks.md` — find the task you are executing.

## Workflow
1. Every non-trivial change maps to a task ID in a `tasks.md`. No task ID → stop and ask. Do not invent scope.
2. **Trivial-change lane** (no task ID needed): docs or typo fixes, comment fixes, dependency patch bumps, a new fixture with its license entry, CI changes that don't alter gates.
3. Tests first: add or update the failing test, then implement.
4. Do not hand-edit Spec Kit-managed files (`.specify/templates/`, `.specify/scripts/`, `.specify/extensions/`, `.claude/skills/speckit-*`). Change them through `specify` commands.
5. Never update golden snapshots (`fixtures/golden/`, `*.snap`) to make a test pass. If output legitimately changed, say so and leave the snapshot update to a human reviewer.
6. Never skip, weaken, or delete a test to get green.

## Non-negotiables
- **Boundary**: `crates/` never touches UI, browser APIs, filesystem, clock, or threads. `apps/` and `packages/` never import engine crates or the wasm package — only `@openplush/engine-client`.
- **No silent wrong output**: a violated geometric invariant returns a typed error. Never clamp, skip, or quietly "fix up" bad geometry.
- **Determinism**: same input → byte-identical output, native and WASM. No `HashMap`/`HashSet` iteration on output paths; use `BTreeMap`, index order, stable sorts, `f64::total_cmp`.
- **Privacy**: no runtime network calls, telemetry, analytics, or remote fonts/scripts.
- **Units**: millimetres after scaling. Never mix model units and mm in one function without a named conversion.
- **Licenses**: new dependencies must be MIT, Apache-2.0, BSD-2/3, ISC, Zlib, Unicode, or MPL-2.0 and pass `cargo deny` / the JS license check. A new dependency category needs an ADR in `docs/adr/`.

## Commands
> Planned — update when the workspace is scaffolded.

| Task | Command |
|---|---|
| Engine tests | `cargo test --workspace` |
| Lint | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` |
| WASM build | `cargo build -p openplush-wasm --target wasm32-unknown-unknown --release` |
| Licenses/advisories | `cargo deny check` |
| Run one fixture | `cargo run -p openplush-cli -- pattern fixtures/meshes/capsule.glb -o out/` |
| Web dev server | `pnpm --filter web dev` |
| Web tests | `pnpm -r test && pnpm --filter web exec playwright test` |

## Definition of done
The PR description states:
- **Edge cases** considered (degenerate geometry, empty input, seam/boundary cases, scale extremes).
- **Failure modes** and the typed error each produces.
- **Tests** added (unit / property / golden) and what each proves.
- **Skipped** — anything deliberately deferred, such as error handling, named explicitly.

All CI gates for the lane you touched pass locally.

## Specs and docs
- Specs state what and why, never how. Technology choices go in `plan.md` / `research.md`.
- Tag uncertain claims in research and plan docs: `[solid]`, `[likely]`, `[speculative]`, `[verify]`.
- Absolute dates (YYYY-MM-DD). Decisions go in the plan's Decision Ledger; principle changes need an ADR (MADR).

## Commits and PRs
- Conventional Commits: `feat(flatten): …`, `fix(mesh): …`, `docs(spec): …`.
- One task per PR, referenced as `Task: T014 (specs/001-pattern-pipeline/tasks.md)`.
- PR sections: What · Why · Task · Edge cases · Failure modes · Tests · Skipped.
