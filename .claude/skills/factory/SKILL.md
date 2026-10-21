---
name: "factory"
description: "One unattended firing of the task factory: pick the next ready tasks for a feature, land each as a stacked PR, record what was skipped and why, and stop at a named condition. Invoked as `/factory <feature>`; a scheduled task fires it into a fresh session."
---

# Factory firing

An unattended session that takes ready tasks from a feature's `tasks.md` to open PRs. The binding rules in `AGENTS.md` and the loop in `steward` still apply; this file says what a firing may decide on its own and when it stops. A firing never asks a question: anything that would need one is a stop condition or a skip.

## Arguments

`/factory <feature>` names the feature (`001`, `002`, `003`). Without one, `.specify/feature.json` names it; if that is missing too, the firing stops with reason `no feature`.

### Placeholders

Before step 1 of One firing, scan the arguments this firing received: the text after `/factory` and any kickoff prompt it was started with. It never scans this file, whose `<feature>`, `<ID>` and `<reason>` are notation. A placeholder is either:

- an angle-bracketed word, `<[A-Za-z][A-Za-z0-9_-]*>`: `<window>`, `<feature>`;
- angle brackets containing a pipe, `<[^<>]*\|[^<>]*>`: `<accepted | change it to ...>`.

Stop at the first one, in reading order, with reason `kickoff: placeholder <text> (<field>)`: the matched text verbatim, and the field it stands for, which is the label before it on its line (`D-056 (4) overflow`), or the bracketed word when there is no label. Nothing is written: no branch, commit, push or comment, not even the ledger comment. A placeholder inside a code span or a quoted example is refused too; the maintainer rewrites the kickoff without it, because a false stop costs one message and a false start hours.

The scan covers the kickoff only. A blocker inside a task (`blocked:`, `human:`, an open decision) is not a placeholder: that task is skipped and recorded in the ledger as before.

## Readiness

A task is **ready** when every line holds:

1. Unticked, and no open PR names its ID (search PR titles and bodies for the ID).
2. No `blocked:` or `human:` token.
3. Every ID in its `needs:` token is ticked.
4. Every task its phase depends on is ticked. The phase order is in the file's `## Dependencies` section (or, for 001, the phase checkpoints) and, for 002 and 003, the `## Blocking dependency on 001` table. The file's prose dependencies count too: the tokens add precision where the prose was ambiguous, they do not replace it.
5. Its test task, if the list names one, is ready or ticked. A test task and the implementation that makes it pass are one PR (`docs/how-to/pull-requests.md`).

Pick ready tasks in ID order within the earliest phase that has one. A spike whose output is a decision for the maintainer is `human:` by nature: skip it with that reason.

## One firing

The placeholder scan (Arguments) runs first; then the steps below. A session runs one firing and never starts a second: the handoff's kickoff starts the next one in a fresh session.

1. Check `main`, the open PRs, and the next free slice number once (`steward`, Sessions).
2. Walk the open PRs first: drive any with a 🔴, red CI, or a conflict (`steward`, Waiting and Reviews), and fix whatever the merge gate's comment names on a tier A, B or C PR that is not `merge:ready` (`steward`, Merging). Events reach only the session that opened a PR, and that session is gone; the gate enables auto-merge on its own.
3. Pick up to four ready tasks that touch different crates and share no file. One task runs inline; two or more run through the Workflow tool, one agent per task, each in its own worktree, all building into the one shared target dir (Build output), and nothing else runs while they do.
4. Per task: branch `NNN/NN-<ID>-slug` from its parent (`main`, or the open PR it depends on); the failing test commit; the implementation commit; `scripts/check.sh <lane>` green, then its wasm32 output removed (Build output); `mutation-auditor` when the PR adds checks to Rust (`steward`, Local review); push; open the PR from the template with base = parent; label its tier; remove the attribution footer the server adds; subscribe.
5. Scope each PR by intent (ADR 0013): one task is one PR unless it carries two intents, and the `Intent` section says what it brings in and why. Never split by size, and never a reason to stop.
6. Append one comment to the feature's ledger (below), emit the handoff (below) as the session's last output, and end the session. Do not wait for CI or review here; the next firing walks the open PRs.

A step boundary is the end of a numbered step, or of one task inside step 4. At each, check the session's context: past 300k tokens, the firing stops there (Stop conditions, `context`) and goes straight to step 6. The step in hand always finishes first, so nothing is left half-pushed.

## Build output

- Every agent of a firing, inline or through the Workflow tool, builds into one shared target dir: `CARGO_TARGET_DIR` set to one path outside the worktrees, never a target dir per worktree. Waiting on cargo's lock is expected.
- After each step that built for a second target (`scripts/check.sh` steps `wasm-check` or `wasm`, or a push whose pre-push hook builds wasm32), remove `$CARGO_TARGET_DIR/wasm32-unknown-unknown`. If another agent's wasm32 build is running, leave it; the next step that ends removes it. A wasm32 build is several GB, and per-worktree copies have filled the disk.

## What a firing decides alone

Branching, committing, pushing to `NNN/NN-*` and `chore/*`, opening a PR and stacking it, labelling its tier, resolving a merge conflict with `main` or its parent, answering 🟡 findings and resolving their threads, and fixing 🔴 findings with the proof `steward` asks for.

## What it never does

Merge a PR itself; enable auto-merge, or apply `merge:ready` (the merge gate does both); force-push; edit a golden snapshot or `*.snap`; weaken a test; touch `.specify/`, `specs/*/spec.md`, or a contract except through a task that says so; accept a decision, unless a standing authorization says it covers decisions ([ADR 0016](../../../docs/adr/0016-standing-merge-authorization.md)); add scope without a task ID; add `rereview` or `mutation-audit` (the gate asks for a review of a new head; the maintainer requests any other rerun).

## Stop conditions

Each ends the firing after the ledger comment names it, then the handoff carries it. A kickoff stop writes nothing, not even the ledger comment; its reason goes in the handoff only:

| Condition | Reason written |
|---|---|
| A placeholder in the arguments (Arguments, Placeholders) | `kickoff: placeholder <text> (<field>)` |
| No ready task | `idle: next blocker <ID> (<token>)` |
| A gate fails twice on the same task after a fix | `gate: <ID> <step>` with the last error |
| A golden snapshot or `*.snap` would change | `golden: <ID>` with the diff summary |
| The task needs a decision, a spec change, or a constitution reading | `decision: <ID> <D-NNN or question>` |
| Usage limit or tool failure that a retry does not clear | `limit: <what>` |
| The session's context passes 300k tokens (One firing, step boundary) | `context: <N>k tokens after <step>` |

A gate or golden stop also leaves the branch pushed at its last green commit, so the work is not lost.

## Ledger

One GitHub issue per feature, titled `Factory ledger: <feature>`, labelled `factory`, created on first use. Each firing appends one comment:

```
**Firing <date> <time UTC>**
Opened: #N (<ID>, stacked on #M), …
Skipped: <ID> (<reason>), …
Stopped: <reason, or `one firing` when no stop condition ended it>
Next ready: <ID> or none
```

The maintainer reads the ledger, accepts decisions and ticks `human:` work in `tasks.md`, and merges tier H PRs; the gate merges the rest, bottom-up as each base becomes `main`. Ticks for landed tasks go in one `docs(tasks): tick …` PR per firing, as `steward` says.

## Handoff

The session's last output, after the ledger comment, from every firing: one that opens PRs, one that stops at its first step, and one that stops on a placeholder. It is what the maintainer pastes into a fresh session. Four sections, in this order; an empty one says `none`:

```
**State**: stopped: <reason>. Merged: #N, … Open: #N <head SHA> base <branch> tier:<X>, … Branch in hand: <branch>
**Next steps**: 1. … 2. …
**Blockers**: <facts>. Lives in <file> <ID>. Recommend: <one>. If unanswered: <default>.
**Kickoff**:
/factory <feature>
<the active grant>
```

- **State**: the `Stopped:` reason as the ledger wrote it (for a kickoff stop, its reason here); PRs merged since the last firing; every open PR this firing opened or drove, with its number, head SHA, base and tier; the branch checked out at the end, or `none`.
- **Next steps**: what the next firing or the maintainer does first, in order.
- **Blockers**: one per decision the firing needs: the facts, the file and ID where it lives, one recommendation, and what happens if nobody answers.
- **Kickoff**: five lines or fewer that start the next firing: `/factory <feature>` and the active grant. Every field is filled, so it passes the placeholder scan: a field the firing cannot fill is left out and asked for under Blockers. Until a structured grant is accepted, the grant is the maintainer's own words that started this run, carried verbatim; with none, `Grant: none`.

## Scheduling

A scheduled task fires `/factory <feature>` into a fresh session (`create_new_session_on_fire`), hourly. The maintainer enables it for the hours they want the factory running and disables it after. A firing that finds nothing to do costs one ledger comment and its handoff.
