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
- Every PR uses `.github/pull_request_template.md`: Stack · What · Why · Task · Edge cases · Failure modes · Tests · Skipped · Size.
- Full detail and worked examples: [docs/how-to/pull-requests.md](docs/how-to/pull-requests.md).

### PR size is a hard rule

**One reviewable slice per PR.** Never bundle a whole feature, a whole phase, or a whole spec.

- **Budget**: 400 changed lines of hand-written source. Excluded from the count: fixture models, generated TS types, snapshot files, lock files.
- Over budget → split it, or justify it in a `**Size**` section of the PR description. "It was easier" is not a justification; "this is one indivisible algorithm" is.
- A PR nobody can review in one sitting does not get reviewed — it gets approved.

### What a PR contains

A PR is **the test task plus the implementation task(s) that make it pass**, committed in that order so test-first stays visible in history while CI is green at the tip.

This is the one place "one task per PR" bends, and it has to: a PR landing only a failing test cannot merge, and marking it `#[ignore]` to get green would violate "never weaken a test".

### Never share a PR

- Two different user stories
- Two different crates (the thin wiring commit is the exception)
- A spec or plan change and an implementation change
- A refactor and a behaviour change
- Repo chores and feature work

### Branch naming

Every PR branch says **which feature, which position in the stack, and what it contains** — readable in `git branch -r` with no other context.

| Kind | Pattern | Example |
|---|---|---|
| Feature slice (spec, plan, or code) | `NNN/NN-slug` | `002/03-plan`, `001/42-T050-obj-import` (illustrative position) |
| Repo chore | `chore/slug` | `chore/pr-policy` |

- `NNN` is the feature number; `NN` is the slice's position in that feature, starting at `01` and never reused. After `001/06-tasks` merges, the next 001 slice is `001/07-…`, whether it stacks or starts from `main`. Sorting branches lists each feature's slices in order.
- Include the task ID in the slug for implementation slices: `001/57-T060a-weld-spatial-hash` (positions in examples are illustrative).
- The slug names the content, not the activity. `001/04-contracts-project-schema`, never `001/design-docs`, `001/updates`, `001/fixes`.
- Spec Kit's `/speckit.specify` creates a local `NNN-feature-slug` branch. That is a working branch: rename it before pushing (`git branch -m 003/01-spec`).
- Session-assigned names (`claude/<slug>-<id>`, `codex/…`, `copilot/…`) are not PR branches. Push the slice under its `NNN/NN-slug` name. If your session cannot, put the intended name and stack position in the PR title and body so a maintainer can rename the branch in GitHub (renaming keeps the PR).

### Stacking is the default for dependent work

If a slice reads, links to, or builds on anything not yet on `main`, it is **stacked**:

1. Cut the branch from the parent slice's branch, not from `main`.
2. Open the PR with **base = the parent branch**, so the diff shows only this slice.
3. Fill in the `Stack` section of the PR template: position, parent PR, child PR. Edit the parent's body to link the new child.
4. Only slices with no unmerged dependency start from `main`. Siblings cut from `main` that depend on each other are the failure this rule exists to prevent: each PR shows dead links and nothing records merge order.

A stack never spans two features and never mixes chores with feature slices. Chores that depend on an unmerged chore stack the same way: base = the parent chore's branch. There is no depth cap: because each PR targets its parent, every diff stays one slice no matter how deep the stack is.

### Merging

- **Merge commits only.** No squash, no rebase-merge. Squash would erase the test-before-implementation commit order, and both squash and rebase-merge rewrite commit SHAs, so every PR above the merged one would need a manual rebase. A merge commit keeps the SHAs, so the next PR's diff against `main` is already just its own slice.
- **Merge bottom-up, and only a PR whose base is `main`.** Merging a PR whose base is another slice merges it into that slice's branch, not into `main`.
- After merging, the head branch is deleted and GitHub retargets the child PR to `main` automatically. If it doesn't, retarget it by hand. Never rebase or force-push a stack someone else is reviewing just to tidy history.
- A change requested on a lower slice goes on that slice's branch. Then merge it upward (`git checkout 002/04-… && git merge 002/03-…`), not rebase.
