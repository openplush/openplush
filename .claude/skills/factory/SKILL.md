---
name: "factory"
description: "One unattended firing of the task factory: pick the next ready tasks for a feature, land each as a stacked PR, record what was skipped and why, and stop at a named condition. Invoked as `/factory <feature>`; a scheduled task fires it into a fresh session."
---

# Factory firing

An unattended session that takes ready tasks from a feature's `tasks.md` to open PRs. The binding rules in `AGENTS.md` and the loop in `steward` still apply; this file says what a firing may decide on its own and when it stops. A firing never asks a question: anything that would need one is a stop condition or a skip.

## Arguments

`/factory <feature>` names the feature (`001`, `002`, `003`). Without one, `.specify/feature.json` names it; if that is missing too, the firing stops with reason `no feature`.

## Readiness

A task is **ready** when every line holds:

1. Unticked, and no open PR names its ID (search PR titles and bodies for the ID).
2. No `blocked:` or `human:` token.
3. Every ID in its `needs:` token is ticked.
4. Every task its phase depends on is ticked. The phase order is in the file's `## Dependencies` section (or, for 001, the phase checkpoints) and, for 002 and 003, the `## Blocking dependency on 001` table. The file's prose dependencies count too: the tokens add precision where the prose was ambiguous, they do not replace it.
5. Its test task, if the list names one, is ready or ticked. A test task and the implementation that makes it pass are one PR (`docs/how-to/pull-requests.md`).

Pick ready tasks in ID order within the earliest phase that has one. A spike whose output is a decision for the maintainer is `human:` by nature: skip it with that reason.

## One firing

1. Check `main`, the open PRs, and the next free slice number once (`steward`, Sessions).
2. Walk the open PRs first: drive any with a 🔴, red CI, or a conflict (`steward`, Waiting and Reviews). Events reach only the session that opened a PR, and that session is gone.
3. Pick up to four ready tasks that touch different crates and share no file. One task runs inline; two or more run through the Workflow tool, one agent per task, each in its own worktree, and nothing else runs while they do.
4. Per task: branch `NNN/NN-<ID>-slug` from its parent (`main`, or the open PR it depends on); the failing test commit; the implementation commit; `scripts/check.sh <lane>` green; `mutation-auditor` when the PR adds checks to Rust (`steward`, Local review); push; open the PR from the template with base = parent; remove the attribution footer the server adds; subscribe.
5. Scope each PR by intent (ADR 0013): one task is one PR unless it carries two intents, and the `Intent` section says what it brings in and why. Never split by size, and never a reason to stop.
6. Append one comment to the feature's ledger (below) and end the session. Do not wait for CI or review here; the next firing walks the open PRs.

## What a firing decides alone

Branching, committing, pushing to `NNN/NN-*` and `chore/*`, opening a PR and stacking it, resolving a merge conflict with `main` or its parent, answering 🟡 findings, and fixing 🔴 findings with the proof `steward` asks for.

## What it never does

Merge; force-push; edit a golden snapshot or `*.snap`; weaken a test; touch `.specify/`, `specs/*/spec.md`, or a contract except through a task that says so; accept a decision; add scope without a task ID; add `rereview` or `mutation-audit` (only the maintainer requests a rerun).

## Stop conditions

Each ends the firing after the ledger comment names it:

| Condition | Reason written |
|---|---|
| No ready task | `idle: next blocker <ID> (<token>)` |
| A gate fails twice on the same task after a fix | `gate: <ID> <step>` with the last error |
| A golden snapshot or `*.snap` would change | `golden: <ID>` with the diff summary |
| The task needs a decision, a spec change, or a constitution reading | `decision: <ID> <D-NNN or question>` |
| Usage limit or tool failure that a retry does not clear | `limit: <what>` |

A gate or golden stop also leaves the branch pushed at its last green commit, so the work is not lost.

## Ledger

One GitHub issue per feature, titled `Factory ledger: <feature>`, labelled `factory`, created on first use. Each firing appends one comment:

```
**Firing <date> <time UTC>**
Opened: #N (<ID>, stacked on #M), …
Skipped: <ID> (<reason>), …
Stopped: <reason>
Next ready: <ID> or none
```

The maintainer reads the ledger, accepts decisions and ticks `human:` work in `tasks.md`, and merges bottom-up. Ticks for landed tasks go in one `docs(tasks): tick …` PR per firing, as `steward` says.

## Scheduling

A scheduled task fires `/factory <feature>` into a fresh session (`create_new_session_on_fire`), hourly. The maintainer enables it for the hours they want the factory running and disables it after. A firing that finds nothing to do costs one ledger comment.
