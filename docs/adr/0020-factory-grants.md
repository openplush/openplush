---
status: Accepted
date: 2026-10-21
deciders: Lavender (maintainer)
---

# A factory grant is one structured argument that bounds a run; it never allows merging

> Revised 2026-10-24, when the merge gate was removed: option 2 rested on the gate's verdict, so the maintainer chose option 1, and the maintainer merges every PR.

## Context and problem statement

Grants given as free text were read inconsistently across factory sessions ([#316](https://github.com/openplush/openplush/issues/316)): most sessions kept treating PRs as "the maintainer merges", one merged a PR it should not have, and a scheduled firing kept running after its grant ended. The question: what is a grant's shape, what may it allow, and how does it end? Feature 005, decision D-002, task 005 T001.

## Decision drivers

* The maintainer states a grant once per run (005 SC-007).
* Every session reads the same grant the same way; no session decides alone what a grant allows (005 FR-015).
* What merges is decided by a person, never by a prompt alone.
* A grant ends on the wall clock, and scheduled firings end with it (005 FR-016).

## Considered options

* 1: **Scope only.** A grant sets hours, features, paths and decisions; every PR waits for the maintainer to merge it.
* 2: **A grant may allow merging, on an automated verdict.** Under a grant that says `merge=yes`, a run merges a PR once an automated gate shows every condition passing.
* 3: **Keep grants as free text** in the maintainer's words.

## Decision outcome

Chosen option: **1**, the maintainer's choice of 2026-10-24. Option 2 needs a gate to vouch for each merge, and the project no longer has one. Option 3 is the free-text grant that sessions read inconsistently (#316) [solid].

**The argument.** `/factory <features> grant hours=<N> paths=<prefixes> decisions=<yes|no>`, or `/factory continue` to renew the last grant with a new start time. The field names above are syntax; a kickoff carries values, so it passes the placeholder rule.

| Field | Meaning | Default |
|---|---|---|
| `hours` | The run ends at start + hours, wall clock; pauses do not extend it | Required; at most 8 |
| features | Only these features' tasks are picked | The positional feature |
| `paths` | Protected paths (the factory skill lists them) the run may edit beyond its tasks' own, such as `scripts/` | None |
| `decisions` | Whether the run may accept a Proposed decision on its own recommendation, recorded as "Accepted — <maintainer> (maintainer), <date>: standing authorization (ledger #<n>)" and listed in the run's ledger comment for the maintainer to revisit | `no` |

A grant the maintainer gives in plain words is recorded on the feature's ledger issue in this shape, verbatim alongside, before the run acts on it. No grant lets a run merge a PR, enable auto-merge or add `rereview`.

**How a grant ends.** Each firing checks the end time before any work. Past it, the firing does no work, emits its handoff with "grant expired", and turns its own scheduled task off, so a schedule never outlives its grant by more than one firing.

**The record.** The handoff's kickoff carries the active grant, so a fresh session renews it without the maintainer restating it.

### Consequences

* Good, because the maintainer writes one line per run, and every session reads it the same way.
* Good, because every merge is the maintainer's, so nothing lands unread by a person.
* Good, because schedules stop within one firing of a grant's end.
* Bad, because open PRs wait for the maintainer for the length of every run, and a stack grows until they merge it.

### Confirmation

* `.claude/skills/factory/SKILL.md` reads the argument and `continue`, and checks the end time first (005 T011, T012); its PR shows worked examples, since skills have no test harness.
* The ledger shows one grant per run, and no firing after a grant's end that did work.

## Pros and cons of the options

### Option 1: scope only

* Good, because a person merges every PR.
* Bad, because PRs wait for the maintainer however long the grant.

### Option 2: merge on an automated verdict

* Good, because a run's PRs would not wait on a person.
* Bad, because it needs a gate whose verdict a merge can rely on, and changes to CI, scripts and agent rules would merge unread while the grant runs.

### Option 3: free text

* Good, because no syntax to learn.
* Bad, because it is what failed: ten restatements in a day, one merge against the rules, a schedule that outlived its grant.

## More information

Spec: `specs/005-factory-hardening/spec.md` (US6). Plan: 005 D-002 in `specs/005-factory-hardening/plan.md`.
