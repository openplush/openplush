---
status: Proposed
date: 2026-10-20
deciders: Lavender (maintainer)
---

# A factory grant is one structured argument that scopes a run and never overrides who merges

## Context and problem statement

On one day the maintainer restated a time-boxed grant about ten times across factory sessions ("N hour grant, can merge, no permission limits"), twice within a minute ([#316](https://github.com/openplush/openplush/issues/316)). Sessions read it differently: most kept treating PRs as "the maintainer merges"; one merged a PR before it read [ADR 0018](0018-hands-off-merging.md), then froze all merges and asked. An hourly scheduled firing kept running for about 2 h after its grant ended. ADR 0018 says agents never merge, grant or no grant, and the gate merges tiers A, B and C; [ADR 0016](0016-standing-merge-authorization.md)'s clause on accepting decisions under a grant still stands. The question: what is a grant's shape, what may it override, and how does it end? Feature 005, decision F-002, task F001.

## Decision drivers

* The maintainer states a grant once per run (005 SC-007).
* Every session reads the same grant the same way; no session decides alone what a grant overrides (005 FR-015).
* The files that decide what merges cannot be judged by what they decide (ADR 0018's reason for tier H).
* A grant ends on the wall clock, and scheduled firings end with it (005 FR-016).

## Considered options

* 1: **A grant scopes the run and never overrides who merges.** Fields: hours, features, paths, decisions. No `merge` field: the gate already merges tiers A, B and C, and tier H stays the maintainer's.
* 2: **A grant may let the run merge tier H PRs under named paths** (for example `scripts/`), restoring a reading of ADR 0016 for tier H.
* 3: **Keep grants as free text** in the maintainer's words (ADR 0016 today), with a clearer factory skill.

## Decision outcome

Chosen option, recommended: **1**. Option 2 lets a run merge a change to the gate, the workflows or the agent rules under its own judgement, which is the case ADR 0018 made tier H for; it also brings back the prompt-and-clock merge decision ADR 0018 replaced with a tested script. Option 3 is the status quo that produced ten restatements and an unauthorised merge [solid].

**The argument.** `/factory <feature>[,<feature>…] grant hours=<N> paths=<prefix>[,<prefix>…] decisions=<yes|no>`, or `/factory continue` to renew the last grant with a new start time. The syntax above shows field names; a kickoff carries values, so it passes the placeholder rule (005 US1).

| Field | Meaning | Default |
|---|---|---|
| `hours` | The run ends at start + hours, wall clock; pauses do not extend it | Required; at most 8 |
| features | Only these features' tasks are picked | The positional feature |
| `paths` | Tier H paths the run may **edit** (open PRs for), such as `scripts/` or `.claude/skills/`; never merge | None: tier H edits only through a task that names them |
| `decisions` | Whether the run may accept a Proposed decision on its own recommendation, as ADR 0016's decisions clause says | `no` |

**What a grant never overrides.** Who merges (ADR 0018: the gate merges A, B, C; the maintainer merges H); `merge:ready`, auto-merge and `rereview`, which only the gate sets; the never-list in `.claude/skills/factory/SKILL.md` (force-push, goldens, weakened tests, scope without a task ID). A kickoff that says "can merge" is answered by this ADR, not by the session: the run states once in its handoff that the gate merges A–C and tier H waits for the maintainer.

**How a grant ends.** Each firing checks the end time before any work. Past it, the firing does no work, emits its handoff with "grant expired", and turns its own scheduled task off (the scheduling tool's enable flag), so a schedule never outlives its grant by more than one firing.

**The record.** The handoff's kickoff carries the active grant, so a fresh session renews it without the maintainer restating it.

### Consequences

* Good, because the maintainer writes one line per run and every session reads it the same way.
* Good, because no session can widen its own authority: the merge rules live in ADR 0018 and a tested script, not in the grant.
* Good, because schedules stop within one firing of a grant's end.
* Bad, because "can merge" no longer means anything; a maintainer who wants tier H merged unattended needs a new ADR that changes ADR 0018.
* Bad, because a run that needs a tier H path outside its grant stops and asks, as today.
* Amends ADR 0016: its grant becomes this argument (end time from `hours`; at most 8 h instead of 4, since the grant no longer merges anything); its decisions clause stands, keyed to `decisions=yes`.

### Confirmation

* `.claude/skills/factory/SKILL.md` reads the argument and `continue` and checks the end time first (005 F011, F012); its PRs show worked examples, since skills have no test harness.
* The ledger shows one grant per run, no merge by an agent, and no firing after a grant's end that did work.

## Pros and cons of the options

### Option 1: scope only

* Good, because it matches ADR 0018 exactly and adds no merge path.
* Bad, because tier H still waits for the maintainer, however long the grant.

### Option 2: tier H merges under named paths

* Good, because a long unattended run could finish its own CI and script work.
* Bad, because the gate's own files and the agent rules would merge unread by a person, under a judgement made in a prompt.

### Option 3: free text

* Good, because no syntax to learn.
* Bad, because it is what failed: ten restatements in a day, one merge against ADR 0018, a schedule that outlived its grant.

## More information

Spec: `specs/005-factory-hardening/spec.md` (US6). Plan: F-002 in `specs/005-factory-hardening/plan.md`. Default if unanswered: no grant syntax lands; factory runs keep ADR 0016's free-text grant, and F011 and F012 stay blocked. Revisit if tier H work regularly waits more than a day for a merge.
