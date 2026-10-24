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
/factory <features> grant hours=<N> paths=<prefixes> decisions=<yes|no>
/factory continue
```

- Features and `paths` are comma-separated, without spaces (`004,005`, `paths=scripts/,.github/`). Fields come in any order; one left out takes its default. Tasks are picked from the features in the order listed, and the first one's ledger is the run's ledger.
- **Record.** A new grant is posted on the run's ledger before the run acts on it (Ledger, Grant record), with its end: `ends=`, now plus `hours`, in UTC. A grant given in plain words is read into this shape field by field, a field the words do not mention taking its default, and posted with the words verbatim beside it.
- **Continue.** The handoff's kickoff carries the grant with its `ends=` (Handoff). A kickoff with `ends=` continues the grant the ledger records with that end; the clock does not restart.
- **Renew.** `/factory continue` renews the newest grant record, by posting time, across the `factory` ledgers: the same fields, a new start time, a new record on that grant's run ledger. Only the maintainer renews: a `/factory continue` that arrives as a scheduled task's prompt rather than a maintainer's message stops at kickoff.
- A grant with no `hours` or more than 8, an `ends=` that no grant record carries, and a `continue` that finds no record or comes from a schedule each stop at kickoff (Stop conditions).
- With no grant, a firing runs on the defaults and has no end time.

### Placeholders

Before step 1 of One firing, scan the arguments this firing received: the text after `/factory` and any kickoff prompt it was started with. It never scans this file, whose `<feature>`, `<ID>` and `<reason>` are notation. A placeholder is either:

- an angle-bracketed word, `<[A-Za-z][A-Za-z0-9_-]*>`: `<window>`, `<feature>`;
- angle brackets containing a pipe, `<[^<>]*\|[^<>]*>`: `<accepted | change it to ...>`.

Stop at the first one, in reading order, with reason `kickoff: placeholder <text> (<field>)`: the matched text verbatim, and the field it stands for, which is the label before it on its line (`001 D-056 (4) overflow`), or the bracketed word when there is no label. Nothing is written: no branch, commit, push or comment, not even the ledger comment. A placeholder inside a code span or a quoted example is refused too; the maintainer rewrites the kickoff without it, because a false stop costs one message and a false start hours.

The scan covers the kickoff only. A blocker inside a task (`blocked:`, `human:`, an open decision) is not a placeholder: that task is skipped and recorded in the ledger as before.

## Readiness

A task is **ready** when every line holds:

1. Unticked, and no open PR names it: list the open PRs and look in each title and body for the ID with its feature (`001 T084`), or, in a PR whose branch is the feature's (`001/…`), for the ID alone, as PRs written before [ADR 0021](../../../docs/adr/0021-one-id-scheme.md) name it. GitHub's search matches the two words apart, so it cannot answer this.
2. No `blocked:` or `human:` token.
3. Every ID in its `needs:` token is ticked.
4. Every task its phase depends on is ticked. The phase order is in the file's `## Dependencies` section (or, for 001, the phase checkpoints) and, for 002 and 003, the `## Blocking dependency on 001` table. The file's prose dependencies count too: the tokens add precision where the prose was ambiguous, they do not replace it.
5. Its test task, if the list names one, is ready or ticked. A test task and the implementation that makes it pass are one PR (`docs/how-to/pull-requests.md`).

Pick ready tasks in ID order within the earliest phase that has one. A spike whose output is a decision for the maintainer is `human:` by nature: skip it with that reason.

## One firing

The placeholder scan (Arguments) runs first, then the grant is read and recorded (Arguments, Grant), then the end check; then the steps below. A session runs one firing and never starts a second: the handoff's kickoff starts the next one in a fresh session.

1. Check `main`, the open PRs, and the next free slice number once (`steward`, Sessions).
2. Walk the open PRs first: drive any with a 🔴, red CI, or a conflict (`steward`, Waiting and Reviews). Events reach only the session that opened a PR, and that session is gone.
3. Pick up to four ready tasks that touch different crates and share no file. One task runs inline; two or more run through the Workflow tool, one agent per task, each in its own worktree, all building into the one shared target dir (Build output), and nothing else runs while they do.
4. Per task: branch `NNN/NN-slug` from its parent (`main`, or the open PR it depends on), the slug naming the intent; the failing test commit; the implementation commit; `scripts/check.sh <lane>` green, then its wasm32 output removed (Build output); `mutation-auditor` when the PR adds checks to Rust (`steward`, Local review); push; open the PR from the template with base = parent; remove the attribution footer the server adds; subscribe.
5. Scope each PR by intent (ADR 0013): one task is one PR unless it carries two intents, and the `Intent` section says what it brings in and why. Never split by size, and never a reason to stop.
6. Append one comment to the feature's ledger (below), emit the handoff (below) as the session's last output, and end the session. Do not wait for CI or review here; the next firing walks the open PRs.

A step boundary is the end of a numbered step, or of one task inside step 4. At each, check the session's context: past 300k tokens, the firing stops there (Stop conditions, `context`) and goes straight to step 6. The step in hand always finishes first, so nothing is left half-pushed.

**End check.** Before any work, and again at each step boundary, a firing under a grant compares the clock with the grant's `ends=`. Past it, the firing does no more work: it turns off each enabled scheduled task whose prompt is this firing's kickoff, then goes straight to step 6 with reason `grant expired`. So a schedule never outlives its grant by more than one firing.

## Build output

- Every agent of a firing, inline or through the Workflow tool, builds into one shared target dir: `CARGO_TARGET_DIR` set to one path outside the worktrees, never a target dir per worktree. Waiting on cargo's lock is expected.
- After each step that built for a second target (`scripts/check.sh` steps `wasm-check` or `wasm`, or a push whose pre-push hook builds wasm32), remove `$CARGO_TARGET_DIR/wasm32-unknown-unknown`. If another agent's wasm32 build is running, leave it; the next step that ends removes it. A wasm32 build is several GB, and per-worktree copies have filled the disk.

## What a firing decides alone

Branching, committing, pushing to `NNN/NN-*` and `chore/*`, opening a PR and stacking it, resolving a merge conflict with `main` or its parent, answering 🟡 findings and resolving their threads, and fixing 🔴 findings with the proof `steward` asks for.

Under a grant, also what its fields allow: editing the protected paths `paths` lists (Protected paths), beyond its tasks' own files; and accepting a Proposed decision on its own recommendation, recorded as [ADR 0020](../../../docs/adr/0020-factory-grants.md)'s `decisions` field says (`decisions=yes`).

## What it never does

Merge a PR or enable auto-merge, under any grant (the maintainer merges every PR); force-push; edit a golden snapshot or `*.snap`; weaken a test; touch `.specify/`, `specs/*/spec.md`, or a contract except through a task that says so; edit a protected path that neither its task nor the grant's `paths` names; accept a decision without `decisions=yes`; pick a task outside the grant's features; add scope without a task ID; add `rereview` or `mutation-audit` (the maintainer requests any rerun).

## Protected paths

The rules, gates and agent configuration every other check relies on. A firing edits one only when its task or the grant's `paths` names it: `.github/`, `scripts/`, `.claude/`, `docs/adr/`, `.specify/`; any `AGENTS.md`, `CLAUDE.md`, `GOVERNANCE.md` or `CODE_OF_CONDUCT.md`; `deny.toml`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.gitattributes` and `LICENSE-*`.

## Stop conditions

Each ends the firing after the ledger comment names it, then the handoff carries it. A kickoff stop writes nothing, not even the ledger comment; its reason goes in the handoff only:

| Condition | Reason written |
|---|---|
| A placeholder in the arguments (Arguments, Placeholders) | `kickoff: placeholder <text> (<field>)` |
| A grant with no `hours` or more than 8, an unrecorded `ends=`, or a `continue` with no record or from a schedule (Arguments, Grant) | `kickoff: grant <what>` |
| No ready task | `idle: next blocker <ID> (<token>)` |
| A gate fails twice on the same task after a fix | `gate: <ID> <step>` with the last error |
| A golden snapshot or `*.snap` would change | `golden: <ID>` with the diff summary |
| The task needs a decision, a spec change, or a constitution reading | `decision: <ID> <D-NNN or question>` |
| Usage limit or tool failure that a retry does not clear | `limit: <what>` |
| The session's context passes 300k tokens (One firing, step boundary) | `context: <N>k tokens after <step>` |
| The grant's end has passed (One firing, End check) | `grant expired: ended <ends>; scheduled task <id> turned off`, or `none found` |

In a reason, an ID carries its feature (`gate: 001 T084 clippy`).

A gate or golden stop also leaves the branch pushed at its last green commit, so the work is not lost.

## Ledger

One GitHub issue per feature, titled `Factory ledger: <feature>`, labelled `factory`, created on first use. Each firing appends one comment, written as `AGENTS.md`'s status prose: words first, then at most two IDs per line, each with its feature and a link. A task links to its line in `tasks.md` at a commit (`blob/<sha>/specs/<feature>/tasks.md#L<line>`), a decision to its plan's `#decision-ledger`.

```
**Firing <date> <time UTC>**
Grant: <the kickoff's grant, with ends=> or none
Opened: <what it brings in> (#N, [<feature> <task>](<link>), stacked on #M), … or none
Accepted: <the decision in words> ([<feature> <decision>](<link>)), … or none
Skipped: <why, in words> ([<feature> <task>](<link>)), …
Stopped: <reason, or `one firing` when no stop condition ended it>
Next ready: <the task in words> ([<feature> <task>](<link>)) or none
IDs: [glossary](https://github.com/openplush/openplush/blob/main/docs/reference/glossary.md#id-prefixes)
```

`Accepted:` lists what this firing accepted under its grant. A new or renewed grant gets its own comment first, its **grant record**:

```
**Grant <date> <time UTC>**
/factory <features> grant hours=<N> paths=<prefixes> decisions=<yes|no> ends=<YYYY-MM-DDTHH:MMZ>
Given as: <the maintainer's words verbatim, or `argument`>
```

The maintainer reads the ledger, accepts the decisions, merges the open PRs bottom-up, and ticks `human:` work in `tasks.md`. Ticks for landed tasks go in one `docs(tasks): tick …` PR per firing, as `steward` says.

## Handoff

The session's last output, after the ledger comment, from every firing: one that opens PRs, one that stops at its first step, and one that stops on a placeholder. It is what the maintainer pastes into a fresh session. Four sections, in this order; an empty one says `none`:

```
**State**: stopped: <reason>. Merged: #N, … Open: #N <head SHA> base <branch>, … Branch in hand: <branch>
**Next steps**: 1. … 2. …
**Blockers**: <facts, in words>. Lives in [<feature> <ID>](<link to the file>). Recommend: <one>. If unanswered: <default>.
**Kickoff**:
/factory <features> grant hours=<N> paths=<prefixes> decisions=<yes|no> ends=<time>
```

- **State**: the `Stopped:` reason as the ledger wrote it (for a kickoff stop, its reason here); PRs merged since the last firing; every open PR this firing opened or drove, with its number, head SHA and base; the branch checked out at the end, or `none`.
- **Next steps**: what the next firing or the maintainer does first, in order.
- **Blockers**: one per decision the firing needs, as status prose (`AGENTS.md`): the facts, the ID where it lives linked to its file, one recommendation, and what happens if nobody answers.
- **Kickoff**: five lines or fewer that start the next firing: the `/factory` line with the active grant and its `ends=`, so the next firing continues it (Arguments, Grant), or `/factory <feature>` alone with no grant. After `grant expired` it is `/factory continue`, which renews the grant only when the maintainer pastes it. Every field is filled, so it passes the placeholder scan: a field the firing cannot fill is left out and asked for under Blockers.

## Scheduling

A scheduled task fires a handoff's kickoff into a fresh session (`create_new_session_on_fire`), hourly; the maintainer creates it for a run. Under a grant, the kickoff carries `ends=`, so the schedule expires with the grant: the first firing past it turns the task off (One firing, End check). Its prompt is never `/factory continue`, which would renew the grant at every firing. Without a grant, the maintainer disables it after the hours they want. A firing that finds nothing to do costs one ledger comment and its handoff.
