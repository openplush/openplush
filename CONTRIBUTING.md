# Contributing to OpenPlush

Thanks for helping. OpenPlush is pre-alpha with no code yet, but both features are fully specified and planned, so there is concrete work to pick up:

- **The most useful contribution right now is test models.** Nothing in `fixtures/` exists, and several success criteria cannot be measured until it does. A model that arrives with a **known-good hand-made sewing pattern** is worth more than any single code change — it becomes a regression oracle and it is the only way to verify that feature 002's hints land where experienced makers actually cut. No code needed.
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
2. Pick a task, open one small PR for it, and reference it: `Task: T014 (specs/001-pattern-pipeline/tasks.md)`.
3. **Trivial changes skip the task requirement:** docs or typo fixes, comment fixes, dependency patch bumps, new fixtures with a license entry, and CI changes that don't alter gates.

## Pull requests are small here
`main` is the trunk and everything arrives by PR. The size rule is not a preference:

- **400 changed lines of hand-written source.** Fixtures, generated types, snapshots, and lock files don't count. Over budget means split it, or justify it in a `**Size**` section.
- **A PR is one test task plus the implementation that makes it pass**, committed in that order. Test-first stays visible in history and CI is green at the tip.
- **Never in one PR**: two user stories, two crates, a spec change and an implementation change, a refactor and a behaviour change, or chores mixed with features.
- Branch names: `NNN/NN-slug` for feature work (`001/42-T050-obj-import`: feature, position, content; numbers here are illustrative), `chore/slug` for repo work. Work that depends on an unmerged PR is **stacked** on it. See [`AGENTS.md`](AGENTS.md#commits-and-prs).

Why so strict: this project's worst outcome is a pattern that looks right and is wrong, discovered after the fabric is cut. A sign error in a dihedral angle or an epsilon against the wrong scale is a one-line defect that produces plausible output. It does not survive a focused review of 200 lines, and it sails through 2,000.

Details and worked examples: [docs/how-to/pull-requests.md](docs/how-to/pull-requests.md).

Read the [constitution](.specify/memory/constitution.md) before your first PR. Reviewers enforce it.

## Pull requests
Use Conventional Commits (`feat(flatten): …`, `fix(mesh): …`, `docs(spec): …`). The PR description covers:
- **What** and **why**
- **Task** ID (unless trivial)
- **Edge cases** you considered
- **Failure modes** and the error each produces
- **Tests** added and what each proves
- **Skipped** — anything deliberately left out

## AI coding agents
Agent rules live in [`AGENTS.md`](AGENTS.md) (plus `crates/AGENTS.md` and `apps/AGENTS.md`). They apply to you too. You're responsible for everything you submit, whoever or whatever wrote it.

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

`scripts/check.sh engine` runs the fast subset (boundary, hygiene, fmt, clippy, tests). Name individual steps to run just those (`boundary hygiene fmt clippy test deny wasm shell`), and add `--dry-run` to see the exact commands. Every CI workflow calls the same script (`engine`, `deny`, `wasm`, `shell`), so a green local run of a step means a green CI run of it. `rust-toolchain.toml` pins the exact Rust toolchain, which is also the MSRV.

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
| `unwrap_used`, `expect_used`, `panic` | No panics on user-derived data. Panics never cross into JS. Allowed in `#[test]` functions and `#[cfg(test)]` modules (`clippy.toml`). Code outside those, such as helpers in `tests/*.rs` and everything in `openplush-testkit`, returns `Result` and uses `?`. |
| `disallowed_types`: `HashMap`, `HashSet` | Iteration order is unspecified, which breaks byte-identical output (Constitution V). |
| `todo`, `unimplemented`, `dbg_macro` | Placeholders and debug output don't merge. |

If a lint is wrong for one specific line, put `#[allow(clippy::…)]` on that item and a comment saying why. For `unwrap`/`expect`, the comment is `// INVARIANT: …` naming the invariant that makes it safe. Never turn a lint off workspace-wide to get one PR green; that needs its own PR and a reason.

## Tests come first
Not a style preference — [constitution](.specify/memory/constitution.md) principle IV. Add or update the failing test, then implement. Two rules that have no exceptions:

- **Never update a golden snapshot** (`fixtures/golden/`, `*.snap`) to make a test pass. If the output legitimately changed, say so in the PR and leave the snapshot update to a reviewer.
- **Never skip, weaken, or delete a test** to get to green.

Both exist because the thing this project most needs to avoid is a pattern that looks right and is wrong — you find out after cutting the fabric.

## License
Contributions are dual-licensed under MIT OR Apache-2.0, as stated in the [README](README.md#license). There is no CLA.
