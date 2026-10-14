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
- Branch names: `001/T050-obj-import` for implementation, `chore/slug` for repo work, `NNN-feature-slug` for specs.

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

One known wrinkle: the Spec Kit git extension creates feature branches from `main`, which is **not** this repo's trunk — the trunk is `001-pattern-pipeline`. After `/speckit-specify` creates a branch, check it with `git log --oneline -3` and fast-forward it onto the trunk if it is missing commits.

## Development environment
You need [rustup](https://rustup.rs), Node 22+, and pnpm 10. `rust-toolchain.toml` pins the exact Rust toolchain (it is also the MSRV), with `rustfmt`, `clippy`, and the `wasm32-unknown-unknown` target. rustup installs it on first use.

```sh
cargo test --workspace
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --target wasm32-unknown-unknown
cargo deny check          # cargo install cargo-deny --locked
scripts/tests/check-boundary.test.sh && scripts/check-boundary.sh   # bash >= 4.3, jq, python3 >= 3.11
pnpm install
```

The engine crates in `crates/` are scaffolds until their tasks land. [001's quickstart](specs/001-pattern-pipeline/quickstart.md) is the end-to-end walkthrough once the pipeline exists.

### Warnings are errors
CI runs clippy with `-D warnings`, so every warning blocks a merge. That is deliberate. The workspace lints (`[workspace.lints]` in `Cargo.toml` and `clippy.toml`) only flag things this project has decided are bugs:

| Lint | Why |
|---|---|
| `unsafe_code = "forbid"` | Constitution, engine lane. Exceptions need an ADR. |
| `missing_docs` | Public items document their pre- and postconditions (`crates/AGENTS.md`). |
| `unwrap_used`, `expect_used`, `panic` | No panics on user-derived data. Panics never cross into JS. |
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
