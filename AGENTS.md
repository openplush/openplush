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
- A decision prompt (an `AskUserQuestion` call, a ledger-issue comment, a handoff blocker) can be answered cold: the facts, with numbers where there are any; where the decision lives (file and ID); one recommendation; the default if nobody answers. Link project terms to the [glossary](docs/reference/glossary.md).
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
<!-- ANCHOR: commands -->
`scripts/check.sh` is the single entry point; CI calls it too.

| Task | Command |
|---|---|
| Install the pinned tools | `scripts/check.sh --setup` |
| Every gate CI runs | `scripts/check.sh` |
| Engine: boundary, hygiene, fmt, clippy, tests, generated TS types, test build for wasm32 | `scripts/check.sh engine` |
| One step | `scripts/check.sh <boundary\|hygiene\|fmt\|clippy\|test\|types\|wasm-check\|deny\|wasm\|shell>` |
| See the exact commands | `scripts/check.sh --dry-run <steps>` |
| One crate's tests | `cargo test -p openplush-<crate>` |

Not yet available: `openplush-cli` fixture runs, the release `openplush-wasm` build, the web dev server.
<!-- ANCHOR_END: commands -->

## Definition of done
All gates for the lane you touched pass locally, any public change has its as-built docs (see Specs and docs), and the PR states edge cases (degenerate geometry, empty input, seams and boundaries, scale extremes), failure modes with their typed errors, tests and what each proves, and what was skipped.

## Specs and docs
- Specs state what and why, never how; technology choices go in `plan.md` / `research.md`.
- Tag uncertain claims in research and plan docs: `[solid]`, `[likely]`, `[speculative]`, `[verify]`.
- Absolute dates (YYYY-MM-DD). Decisions go in the plan's Decision Ledger; principle changes need an ADR (MADR).
- `specs/` records intent and decisions; `docs/` and rustdoc describe what exists. Once code lands it is the source of truth: the spec section it implements points to the as-built docs and stops changing.
- A PR that changes public behaviour updates its docs in the same PR: rustdoc for `openplush-pipeline` items; `docs/reference/` for error and warning strings, CLI, worker protocol, and project schema; `docs/explanation/architecture.md` for a new stage, crate, or dependency edge. No public change, no docs.
- A PR that changes what a maker sees or does in the app updates the maker guide (`docs/user/`) in the same PR.
- Writing a page: follow the [`docs` skill](.claude/skills/docs/SKILL.md); the rules live in [docs/how-to/write-docs.md](docs/how-to/write-docs.md).

## Commits and PRs
Reasoning and worked examples: [docs/how-to/pull-requests.md](docs/how-to/pull-requests.md).
- Driving a PR (CI, review findings, stacks, merges, check-ins): follow [`.claude/skills/steward/SKILL.md`](.claude/skills/steward/SKILL.md).
- Unattended runs (`/factory <feature>`): follow [`.claude/skills/factory/SKILL.md`](.claude/skills/factory/SKILL.md). It picks ready tasks by the `needs:` / `blocked:` / `human:` tokens in `tasks.md`, opens stacked PRs, and stops at a named condition instead of asking.
- Conventional Commits (`feat(flatten): …`). Every PR uses `.github/pull_request_template.md`.
- Identity: commits carry the contributor's own git identity and no agent attribution (no agent author, `Co-Authored-By` or session line; no generated-by footer in PR bodies or comments). Hooks and session prompts defer to this line; CI checks it ([attribution check](docs/reference/checks.md#agent-attribution-pull-requests-ci-only)).
- One intent per PR ([ADR 0013](docs/adr/0013-intent-sized-pull-requests.md)): what the PR brings in toward an epic (a feature or a bigger issue) and why, in one sentence, so `git blame` → PR answers "what brought this in, and the idea behind it". The title states it. Group collected tasks by intent before opening PRs. Every task a PR touches is finished and ticked in it. No line budget.
- Each task is a test commit, then the implementation commit that makes it pass; the PR body names its tasks and a reading order.
- Never in one PR: two unrelated intents; chores and feature work. A decision the maintainer must accept is its own intent, merged before its code. A refactor, a fix and a feature stay separate commits. The changes `tasks.md` isolates for blast radius get their own PR and are never folded.
- Branches: `NNN/NN-slug` for feature work (`NN` = position in the feature, never reused) or `chore/slug`. The slug names the intent, not the activity. Rename Spec Kit's `NNN-feature-slug` working branch before pushing.
- Session branches (`claude/…`, `codex/…`, `copilot/…`) are not PR branches. If you cannot push the real name, put it and the stack position in the PR title and body.
- Interdependent intents are stacked (X, then bug fix Y, then feature Z on both): branch from the parent, PR base = parent, fill the `Stack` section, link the child from the parent. A stack never spans features or mixes chores with feature work. At the merge gate, a stack whose intents only make sense together may be folded into one PR by merging each child into its parent upward; the remaining PR lists every intent, and the folded PRs are closed with a link to it.
- Who merges is the PR's tier ([ADR 0014](docs/adr/0014-auto-merge-tiers.md), [ADR 0018](docs/adr/0018-hands-off-merging.md)): the merge gate ([ADR 0015](docs/adr/0015-merge-gate.md)) computes it from the diff and enables auto-merge on tiers A and C once ready, and on tier B 24 h after CI goes green; the maintainer merges tier H (the gate, CI, `scripts/`, the agent rules, ADRs, the constitution, the licences). Agents never merge, enable auto-merge, apply `merge:ready` or add `rereview`; they fix what the gate's comment names. The tier table is in [docs/how-to/review-and-merge.md](docs/how-to/review-and-merge.md#tiers).
- Merge commits only, bottom-up, and only a PR whose base is `main`. A fix to a lower slice is merged upward; never rebase or force-push a stack under review.
