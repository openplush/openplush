# Contributing to OpenPlush

Thanks for helping. OpenPlush is pre-alpha with no code yet, but both features are fully specified and planned, so there is concrete work to pick up:

- **The most useful contribution right now is test models.** `fixtures/` holds only generated shapes (a cube, a sphere, a capsule, and the defect corpus), so several success criteria cannot be measured until real plush models arrive. A model that arrives with a **known-good hand-made sewing pattern** is worth more than any single code change — it becomes a regression oracle and it is the only way to verify that feature 002's hints land where experienced makers actually cut. No code needed.
- **Engine work** starts at [001 tasks](specs/001-pattern-pipeline/tasks.md) `T001`–`T008` (research spikes) and `T009`–`T021` (workspace setup). `T007` is the highest-value spike: the exact-seam-parity decision rests the whole pattern stage on it and nobody has verified it yet.
- **Spec and plan review** is still welcome. Five minor defaults remain open in 001, and every fabric preset value is currently a guess.

## Lanes
| Lane | Folder | You need |
|---|---|---|
| Engine | `crates/` | Rust; geometry helps |
| Web app | `apps/`, `packages/` | TypeScript, Three.js — no Rust |
| Fixtures | `fixtures/` | 3D modeling or sewing. No code. A model plus a known-good hand-made pattern is gold. |
| Docs | `docs/` | Writing. Sewing expertise is especially welcome. |

## How work flows
OpenPlush uses spec-driven development ([Spec Kit](https://github.com/github/spec-kit)).

1. Features are specified in `specs/NNN-name/spec.md`, planned in `plan.md`, and broken into tasks in `tasks.md`.
2. Pick a task, or a few that finish one outcome together, open one PR for them, and reference them: `Tasks: T014 (specs/001-pattern-pipeline/tasks.md)`.
3. **Trivial changes skip the task requirement:** docs or typo fixes, comment fixes, dependency patch bumps, new fixtures with a license entry, and CI changes that don't alter gates.

## Pull requests carry one intent
`main` is the trunk and everything arrives by PR ([ADR 0013](docs/adr/0013-intent-sized-pull-requests.md)):

- **One intent per PR**: what it brings in toward an epic (a feature or a bigger issue) and why, in one sentence and in its title, so tracing a line back to its PR explains it. Every task it touches is finished, and ticked, in that PR. There is no line budget.
- **Each task is a test commit, then the implementation that makes it pass.** A large PR is reviewed commit by commit, so name the reading order in the description.
- **Never in one PR**: two unrelated intents, or chores mixed with features. A decision the maintainer accepts lands before the code that implements it. Interdependent intents are **stacked** (X, then a fix, then the feature on both), and may be folded into one PR at the merge gate.
- Branch names: `NNN/NN-slug` for feature work (`001/42-obj-import`: feature, position, intent; numbers here are illustrative), `chore/slug` for repo work. Work that depends on an unmerged PR is **stacked** on it. See [`AGENTS.md`](AGENTS.md#commits-and-prs).

Why intent and not size: this project's worst outcome is a pattern that looks right and is wrong, discovered after the fabric is cut. A reviewer catches that by checking a stated outcome against its tests, which needs the whole capability in view. Test-first commits keep a large PR readable one step at a time.

Details and worked examples: [docs/how-to/pull-requests.md](docs/how-to/pull-requests.md).

Read the [constitution](.specify/memory/constitution.md) before your first PR. Reviewers enforce it.

## Pull requests
Use Conventional Commits (`feat(flatten): …`, `fix(mesh): …`, `docs(spec): …`). The PR description covers:
- **Intent**, and the **Task** IDs it finishes (unless trivial)
- **Reading order** and **why**
- **Edge cases** you considered
- **Failure modes** and the error each produces
- **Tests** added and what each proves
- **Skipped** — anything deliberately left out

## AI coding agents
Agent rules live in [`AGENTS.md`](AGENTS.md) (plus `crates/AGENTS.md` and `apps/AGENTS.md`). They apply to you too. You're responsible for everything you submit, whoever or whatever wrote it.

### Automated review
Every non-draft pull request gets one adversarial review from the agents in [`.claude/agents/`](.claude/agents/), run by `.github/workflows/review.yml` when the PR is opened or marked ready for review:
- `pr-reviewer` on every PR: bugs, rule violations, and claims the code doesn't back.
- `spec-reviewer` when `specs/` changes: contradictions between documents, missing ledger entries.
- `mutation-auditor` on request (it is the expensive one): breaks each new check in turn and reports the ones no test catches.

It runs once per PR, not on every push. A rerun is the maintainer's call: the `rereview` label runs it again, and `mutation-audit` alone runs it again with the mutation auditor (add only that one; two labels added one after the other can start two runs). AI agents never add either label; they prove their own fixes with tests. Both labels are removed when the run starts. A rerun reviews only what changed since the last summary, and skips findings already answered on the PR.

Findings arrive as review comments marked 🔴 blocking, 🟡 optional, or 🟣 pre-existing. Answer every 🔴: fix it or reply with why not. The review is advisory and never approves. Who merges depends on what the PR touches ([ADR 0014](docs/adr/0014-auto-merge-tiers.md)): engine code, specs, decisions, CI and dependencies wait for a maintainer. To run the same agents before you push, ask Claude Code in this repository to use them by name, for example "run the mutation-auditor agent on my branch against main".

## Authoring specs (maintainers)
Needs Python 3.11+, [uv](https://docs.astral.sh/uv/), and Spec Kit:

```sh
uv tool install specify-cli
specify version
```

This repo uses Spec Kit's Claude Code integration with `sh` scripts. Skills are invoked as `/speckit-<command>`.

Spec Kit tracks the active feature in `.specify/feature.json`, which is local and git-ignored. After cloning, point it at the feature you're working on:

```json
{"feature_directory":"specs/001-pattern-pipeline"}
```

On Windows, Claude Code runs the `sh` scripts through Git Bash. Verified working as of 2026-10-13 for `create-new-feature-branch.sh` and `check-prerequisites.sh`. If a script does fail, switch to PowerShell with `specify integration upgrade claude --script ps --force`.

`/speckit-specify` creates a local `NNN-feature-slug` branch from `main`, which is the trunk. Rename it to the slice name before pushing: `git branch -m 003/01-spec`.

## Development environment
Prerequisites: [rustup](https://rustup.rs), Node 22+ with pnpm 10 (`corepack enable`), bash ≥ 4.3, jq, and python3 ≥ 3.11. Then, three commands:

```sh
git clone https://github.com/openplush/openplush && cd openplush
scripts/check.sh --setup   # the pinned Rust toolchain, cargo-deny, and the wasm test runner matching Cargo.lock
scripts/check.sh           # every gate CI runs
```

`scripts/check.sh engine` runs the fast subset (boundary, hygiene, fmt, clippy, tests, and `types`, which fails if `packages/engine-client/src/generated/` differs from a fresh `scripts/gen-ts-types.sh` export). Name individual steps to run just those (`boundary hygiene fmt clippy test types deny wasm shell`), and add `--dry-run` to see the exact commands. Every CI workflow calls the same script (`engine`, `deny`, `wasm`, `shell`), so a green local run of a step means a green CI run of it. `rust-toolchain.toml` pins the exact Rust toolchain, which is also the MSRV.

### Dependency licenses
Every dependency, dev dependencies included, must carry a license on the allowlist in `deny.toml` ([constitution](.specify/memory/constitution.md) principle IX). `scripts/check.sh deny` checks crates with `cargo deny`. `scripts/check.sh shell` runs `scripts/check-js-licenses.sh` right after `pnpm install`, which checks every installed npm package against the same allowlist. It reads the license each package declares in its own `package.json`, never pnpm's license column, because pnpm guesses a license from the LICENSE text when the field is missing. pnpm installs only the platform packages that match your machine, so for every other package in `pnpm-lock.yaml` the gate asks the registry (`pnpm view`) for its declared license. Like `pnpm install`, it needs registry access. `OR` passes if any alternative is allowed and `AND` only if every part is. A `WITH` exception is never allowed on its own, so it passes only as an `OR` alternative to an allowed license. IDs are case-sensitive. A missing license, `SEE LICENSE IN …`, `UNLICENSED`, a `licenses` array of more than one entry, and any expression that mixes `AND` with `OR` all fail. The last is stricter than `cargo deny` on purpose, until a real dependency needs it. A license that isn't on the list needs an ADR; there is no per-package exception.

### Tests that must also run on wasm32
`cargo test --target wasm32-unknown-unknown` only runs tests marked `#[wasm_bindgen_test]`. Plain `#[test]` functions are skipped there, with "no tests to run!" and exit 0. And `#[wasm_bindgen_test]` on its own compiles to nothing natively, so a test using only that attribute silently never runs off wasm. For a test that must run on both targets, use both attributes, as `crates/openplush-geom/tests/wasm_determinism.rs` does:

```rust
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn my_test() { /* … */ }
```

The crate needs `[target.'cfg(target_arch = "wasm32")'.dev-dependencies] wasm-bindgen-test.workspace = true`.

Name such a file `tests/wasm_<topic>.rs`. `scripts/check.sh wasm` treats every file with that name as a canary and fails if it runs no test on wasm32, which is what happens silently if the attribute goes missing.

The engine crates in `crates/` are scaffolds until their tasks land. [001's quickstart](specs/001-pattern-pipeline/quickstart.md) is the end-to-end walkthrough once the pipeline exists.

### Warnings are errors
CI runs clippy with `-D warnings`, so every warning blocks a merge. That is deliberate. The workspace lints (`[workspace.lints]` in `Cargo.toml` and `clippy.toml`) only flag things this project has decided are bugs:

| Lint | Why |
|---|---|
| `unsafe_code = "forbid"` | Constitution, engine lane. Exceptions need an ADR. |
| `missing_docs` (`openplush-pipeline` only) | The public engine API documents every item. Elsewhere a doc is one line saying what the signature doesn't (`AGENTS.md`, Comments and prose). |
| `unreachable_pub` | Visibility is `pub(crate)` unless another crate calls the item (YAGNI). |
| `too_many_lines` (60), `too_many_arguments` (5) | Small units. A function that predates the limit carries `#[expect(clippy::too_many_lines, reason = …)]` until it is split. |
| `unwrap_used`, `expect_used`, `panic` | No panics on user-derived data. A panic cannot be caught on wasm32 ([`crates/AGENTS.md`](crates/AGENTS.md#wasm-constraints), WASM constraints). Allowed in `#[test]` functions and `#[cfg(test)]` modules (`clippy.toml`). Code outside those, such as helpers in `tests/*.rs` and everything in `openplush-testkit`, returns `Result` and uses `?`. |
| `disallowed_types`: `HashMap`, `HashSet` | Iteration order is unspecified, which breaks byte-identical output (Constitution V). |
| `disallowed_methods`: `sort_unstable*`, `select_nth_unstable*` | Equal elements end in an unspecified order, which has differed between x86_64 and wasm32 and between toolchains (Constitution V). Sort stably with a total order (`total_cmp`). |
| `todo`, `unimplemented`, `dbg_macro` | Placeholders and debug output don't merge. |

If a lint is wrong for one specific line, put `#[allow(clippy::…)]` on that item and a comment saying why. For `unwrap`/`expect`, the comment is `// INVARIANT: …` naming the invariant that makes it safe. An opt-out of `disallowed_types` or `disallowed_methods` also needs a line in `scripts/disallowed-opt-outs.txt` with its reason, so a reviewer sees every one in one place. `scripts/check.sh hygiene` runs `scripts/check-opt-outs.sh`, which fails on an unlisted opt-out, on an entry whose opt-out is gone, on any `clippy.toml` or `.clippy.toml` besides the three known ones, and on `CLIPPY_CONF_DIR` in a Cargo.toml or `.cargo/config*`. It scans the files git tracks or would add (so ignored trees such as build output or a nested worktree are skipped) and counts every textual mention of either lint name in a `.rs` file under `crates/` or `bindings/`, a Cargo.toml, or a `.cargo/config*`, plus `clippy::style`, `clippy::all`, and `warnings` lint levels; its header lists the exact patterns and what they miss. It is a tripwire for honest mistakes, not a sandbox. Never turn a lint off workspace-wide to get one PR green; that needs its own PR and a reason.

## Tests come first
Not a style preference — [constitution](.specify/memory/constitution.md) principle IV. Add or update the failing test, then implement. Two rules that have no exceptions:

- **Never update a golden snapshot** (`fixtures/golden/`, `*.snap`) to make a test pass. If the output legitimately changed, say so in the PR and leave the snapshot update to a reviewer.
- **Never skip, weaken, or delete a test** to get to green.

Both exist because the thing this project most needs to avoid is a pattern that looks right and is wrong — you find out after cutting the fabric.

## License
Contributions are dual-licensed under MIT OR Apache-2.0, as stated in the [README](README.md#license). There is no CLA.
