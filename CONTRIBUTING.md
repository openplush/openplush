# Contributing to OpenPlush

Thanks for helping. OpenPlush is pre-alpha: the first feature is still being specified, so the most useful contributions right now are spec review, test models, and research.

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
2. Pick a task, open one PR for it, and reference it: `Task: T014 (specs/001-pattern-pipeline/tasks.md)`.
3. **Trivial changes skip the task requirement:** docs or typo fixes, comment fixes, dependency patch bumps, new fixtures with a license entry, and CI changes that don't alter gates.

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

On Windows, Claude Code runs the `sh` scripts through Git Bash. If they fail, switch to PowerShell scripts with `specify integration upgrade claude --script ps --force`.

## Development environment
The build setup arrives with the first setup tasks in `specs/001-pattern-pipeline/tasks.md`. Planned: Rust stable with the `wasm32-unknown-unknown` target, Node LTS, and pnpm.

## License
Contributions are dual-licensed under MIT OR Apache-2.0, as stated in the [README](README.md#license). There is no CLA.
