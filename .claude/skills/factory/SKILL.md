---
name: "factory"
description: "One unattended firing of the task factory: pick the next ready tasks for a feature, land each as a stacked PR, record what was skipped and why, and stop at a named condition. Invoked as `/factory <feature>`; a scheduled task fires it into a fresh session."
---

# Factory firing

An unattended session that takes ready tasks from a feature's `tasks.md` to open PRs. The binding rules in `AGENTS.md` and the loop in `steward` still apply; this file says what a firing may decide on its own and when it stops. A firing never asks a question: anything that would need one is a stop condition or a skip.

## Arguments

`/factory <feature>` names the feature (`001`, `002`, `003`). Without one, `.specify/feature.json` names it; if that is missing too, the firing stops with reason `no feature`.

### Grant

What a run may do beyond this file's defaults is one argument, the grant. [ADR 0020](../../../docs/adr/0020-factory-grants.md) sets its fields, their defaults and limits; this section says how a firing reads it.

```
/factory <features> grant hours=<N> merge=<yes|no> paths=<prefixes> decisions=<yes|no>
/factory continue
```

- Features and `paths` are comma-separated, without spaces (`004,005`, `paths=scripts/,.github/`). Fields come in any order; one left out takes its default. Tasks are picked from the features in the order listed, and the first one's ledger is the run's ledger.
- **Record.** A new grant is posted on the run's ledger before the run acts on it (Ledger, Grant record), with its end: `ends=`, now plus `hours`, in UTC. A grant given in plain words is read into this shape field by field, a field the words do not mention taking its default, and posted with the words verbatim beside it.
- **Continue.** The handoff's kickoff carries the grant with its `ends=` (Handoff). A kickoff with `ends=` continues the grant the ledger records with that end; the clock does not restart.
- **Renew.** `/factory continue` renews the latest grant record on any `factory` ledger: the same fields, a new start time, a new record.
- A grant with no `hours` or more than 8, an `ends=` that no grant record carries, and a `continue` that finds no record each stop at kickoff (Stop conditions).
- With no grant, a firing runs on the defaults and has no end time.

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

The placeholder scan (Arguments) runs first, then the grant is read and recorded (Arguments, Grant); then the steps below. A session runs one firing and never starts a second: the handoff's kickoff starts the next one in a fresh session.

1. Check `main`, the open PRs, and the next free slice number once (`steward`, Sessions).
2. Walk the open PRs first: drive any with a 🔴, red CI, or a conflict (`steward`, Waiting and Reviews), and fix whatever the merge gate's comment names on a tier A, B or C PR that is not `merge:ready` (`steward`, Merging). Events reach only the session that opened a PR, and that session is gone; the gate enables auto-merge on its own. Under `merge=yes`, merge each tier H PR the rule allows (Merging under a grant).
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

Under a grant, also what its fields allow: merging a tier H PR by the rule below (`merge=yes`); editing the tier H paths `paths` lists, beyond its tasks' own files; and accepting a Proposed decision on its own recommendation, recorded as [ADR 0016](../../../docs/adr/0016-standing-merge-authorization.md)'s decisions clause says (`decisions=yes`).

## What it never does

Merge a PR, except by Merging under a grant; enable auto-merge or apply `merge:ready`, under any grant (the merge gate does both); force-push; edit a golden snapshot or `*.snap`; weaken a test; touch `.specify/`, `specs/*/spec.md`, or a contract except through a task that says so; edit a tier H path that neither its task nor the grant's `paths` names; accept a decision without `decisions=yes`; pick a task outside the grant's features; add scope without a task ID; add `rereview` or `mutation-audit` (the gate asks for a review of a new head; the maintainer requests any other rerun).

## Merging under a grant

Under `merge=yes`, in step 2, a firing merges an open tier H PR on one of the grant's features (`NNN/` branch) only when every line holds; this is [ADR 0020](../../../docs/adr/0020-factory-grants.md)'s merge rule:

1. None of its files is `scripts/merge-gate.py`, `scripts/pr-tier.py` or `.github/workflows/merge-gate.yml`: the gate cannot vouch for its own replacement.
2. The gate's latest comment names the PR's current head (`head <sha7>`) and shows every row but `tier` passing.
3. GitHub reports no conflict.
4. The grant's end has not passed.

It merges with a merge commit at that head SHA (the merge call's `sha`), so a push since the comment fails the merge instead of landing unjudged code. Row `base is main` keeps it bottom-up: a child waits for a later firing, after the gate judges it against `main`. Each merge goes on the ledger comment's `Merged:` line with its head SHA and a link to the gate comment it relied on. A PR that fails a line stays for the maintainer, under `Skipped:` with the line it failed.

## Stop conditions

Each ends the firing after the ledger comment names it, then the handoff carries it. A kickoff stop writes nothing, not even the ledger comment; its reason goes in the handoff only:

| Condition | Reason written |
|---|---|
| A placeholder in the arguments (Arguments, Placeholders) | `kickoff: placeholder <text> (<field>)` |
| A grant with no `hours` or more than 8, an unrecorded `ends=`, or a `continue` with no record (Arguments, Grant) | `kickoff: grant <what>` |
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
Grant: <the kickoff's grant, with ends=> or none
Opened: #N (<ID>, stacked on #M), …
Merged: #N <head SHA> (gate <comment URL>), … or none
Accepted: <decision ID>, … or none
Skipped: <ID> (<reason>), …
Stopped: <reason, or `one firing` when no stop condition ended it>
Next ready: <ID> or none
```

`Merged:` and `Accepted:` list what this firing did under its grant. A new or renewed grant gets its own comment first, its **grant record**:

```
**Grant <date> <time UTC>**
/factory <features> grant hours=<N> merge=<yes|no> paths=<prefixes> decisions=<yes|no> ends=<YYYY-MM-DDTHH:MMZ>
Given as: <the maintainer's words verbatim, or `argument`>
```

The maintainer reads the ledger, accepts the decisions and merges the tier H PRs no grant took, and ticks `human:` work in `tasks.md`; the gate merges the rest, bottom-up as each base becomes `main`. Ticks for landed tasks go in one `docs(tasks): tick …` PR per firing, as `steward` says.

## Handoff

The session's last output, after the ledger comment, from every firing: one that opens PRs, one that stops at its first step, and one that stops on a placeholder. It is what the maintainer pastes into a fresh session. Four sections, in this order; an empty one says `none`:

```
**State**: stopped: <reason>. Merged: #N, … Open: #N <head SHA> base <branch> tier:<X>, … Branch in hand: <branch>
**Next steps**: 1. … 2. …
**Blockers**: <facts>. Lives in <file> <ID>. Recommend: <one>. If unanswered: <default>.
**Kickoff**:
/factory <features> grant hours=<N> merge=<yes|no> paths=<prefixes> decisions=<yes|no> ends=<time>
```

- **State**: the `Stopped:` reason as the ledger wrote it (for a kickoff stop, its reason here); PRs merged since the last firing; every open PR this firing opened or drove, with its number, head SHA, base and tier; the branch checked out at the end, or `none`.
- **Next steps**: what the next firing or the maintainer does first, in order.
- **Blockers**: one per decision the firing needs: the facts, the file and ID where it lives, one recommendation, and what happens if nobody answers.
- **Kickoff**: five lines or fewer that start the next firing: the `/factory` line with the active grant and its `ends=`, so the next firing continues it (Arguments, Grant), or `/factory <feature>` alone with no grant. Every field is filled, so it passes the placeholder scan: a field the firing cannot fill is left out and asked for under Blockers.

## Scheduling

A scheduled task fires `/factory <feature>` into a fresh session (`create_new_session_on_fire`), hourly. The maintainer enables it for the hours they want the factory running and disables it after. A firing that finds nothing to do costs one ledger comment and its handoff.
