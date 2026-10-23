---
status: Accepted
date: 2026-10-21
deciders: Lavender (maintainer)
---

# A factory grant is one structured argument, and under it a run may merge a tier H PR the gate shows green but for its tier

## Context and problem statement

Grants given as free text were read inconsistently across factory sessions ([#316](https://github.com/openplush/openplush/issues/316)): most sessions kept treating PRs as "the maintainer merges", one merged a PR against [ADR 0018](0018-hands-off-merging.md), and a scheduled firing kept running after its grant ended. ADR 0018 says agents never merge, grant or no grant, and the gate merges tiers A, B and C; [ADR 0016](0016-standing-merge-authorization.md)'s clause on accepting decisions under a grant still stands. A run that merged tier H PRs only once the gate showed every condition but the tier passing on their heads kept `main` green. The question: what is a grant's shape, what may it allow, and how does it end? Feature 005, decision D-002, task 005 T001.

## Decision drivers

* The maintainer states a grant once per run (005 SC-007).
* Every session reads the same grant the same way; no session decides alone what a grant allows (005 FR-015).
* The files that decide what merges must not be judged by a prompt alone (ADR 0018's reason for tier H).
* A grant ends on the wall clock, and scheduled firings end with it (005 FR-016).

## Considered options

* 1: **Scope only.** A grant sets hours, features, paths and decisions; tier H always waits for the maintainer.
* 2: **A grant may allow merging tier H, on the gate's verdict.** Under a grant that says `merge=yes`, a run may merge a tier H PR only when the merge gate's comment for its current head shows every condition but the tier passing.
* 3: **Keep grants as free text** in the maintainer's words (ADR 0016 today).

## Decision outcome

Chosen option: **2**, the maintainer's choice of 2026-10-21. Option 1 leaves tier H waiting on a person for the length of every run, which is the stall #316 describes. Option 3 is the free-text grant that sessions read inconsistently (#316) [solid]. Option 2 keeps the judgement in the gate's tested conditions (CI, a review of this head with no 🔴, no open thread, no changes requested, base `main`, no conflict) and lets the grant stand in for the one condition the gate leaves to a person: that someone chose to let tier H through.

**The argument.** `/factory <features> grant hours=<N> merge=<yes|no> paths=<prefixes> decisions=<yes|no>`, or `/factory continue` to renew the last grant with a new start time. The field names above are syntax; a kickoff carries values, so it passes the placeholder rule.

| Field | Meaning | Default |
|---|---|---|
| `hours` | The run ends at start + hours, wall clock; pauses do not extend it | Required; at most 8 |
| features | Only these features' tasks are picked | The positional feature |
| `merge` | Whether the run may merge tier H PRs, on the rule below | `no` |
| `paths` | Tier H paths the run may edit beyond its tasks' own, such as `scripts/` | None |
| `decisions` | Whether the run may accept a Proposed decision on its own recommendation, as ADR 0016's decisions clause says | `no` |

A grant the maintainer gives in plain words is recorded on the feature's ledger issue in this shape, verbatim alongside, before the run acts on it.

**The merge rule.** Under `merge=yes`, a run merges a tier H PR only when, on its current head:

* the merge gate's latest comment is for that head and every row but `tier` passes;
* GitHub reports no conflict;
* it merges with a merge commit, at the expected head SHA, bottom-up in its stack;

and it records each merge on the ledger. It still never applies `merge:ready`, enables auto-merge or adds `rereview`; those stay the gate's. A PR that changes `scripts/merge-gate.py`, `scripts/pr-tier.py` or `.github/workflows/merge-gate.yml` is never merged under a grant: the gate cannot vouch for its own replacement.

**How a grant ends.** Each firing checks the end time before any work. Past it, the firing does no work, emits its handoff with "grant expired", and turns its own scheduled task off, so a schedule never outlives its grant by more than one firing.

**The record.** The handoff's kickoff carries the active grant, so a fresh session renews it without the maintainer restating it.

### Consequences

* Good, because the maintainer writes one line per run, every session reads it the same way, and tier H work no longer stalls for the length of a run.
* Good, because every merge under a grant passed the same tested conditions as an auto-merge; the grant adds only the person's say-so.
* Good, because schedules stop within one firing of a grant's end.
* Bad, because tier H changes (CI, scripts, agent rules) can merge with no person having read them during a grant; the review on opening and CI are the readers, and the maintainer reads the ledger and can revert.
* Bad, because the gate's own files are excluded, so a run that needs them stops for the maintainer.
* Amends ADR 0018: "agents never merge, grant or no grant" holds except under a grant with `merge=yes`, by the merge rule above.
* Amends ADR 0016: its grant becomes this argument (end time from `hours`, at most 8 h); its decisions clause stands, keyed to `decisions=yes`.

### Confirmation

* `.claude/skills/factory/SKILL.md` reads the argument and `continue`, checks the end time first, and merges only by the rule above (005 T011, T012); its PR shows worked examples, since skills have no test harness.
* The ledger shows one grant per run, each merge with its head SHA and the gate verdict it relied on, and no firing after a grant's end that did work.

## Pros and cons of the options

### Option 1: scope only

* Good, because it matches ADR 0018 exactly.
* Bad, because tier H waits for the maintainer however long the grant.

### Option 2: merge on the gate's verdict

* Good, because the conditions are the gate's tested ones, and the grant only replaces the human tier.
* Bad, because tier H merges unread by a person while the grant runs.

### Option 3: free text

* Good, because no syntax to learn.
* Bad, because it is what failed: ten restatements in a day, one merge against ADR 0018, a schedule that outlived its grant.

## More information

Spec: `specs/005-factory-hardening/spec.md` (US6). Plan: 005 D-002 in `specs/005-factory-hardening/plan.md`. Revisit after the first revert of a tier H change merged under a grant.
