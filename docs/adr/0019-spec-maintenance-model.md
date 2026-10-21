---
status: Proposed
date: 2026-10-20
deciders: Lavender (maintainer)
---

# Specs flow forward; ticks and status are the only facts kept live, and copies of them are generated or checked

## Context and problem statement

[ADR 0003](0003-as-built-docs.md) made code and `docs/` authoritative once built, and froze built spec sections as the record of intent. It did not say how the facts that still change after a feature starts (which tasks are done, what is built) stay true in the places that repeat them. On `main` on 2026-10-20 three had drifted: the status page undercounted 002's ticked tasks by one, the README's task count for 001 was ten short and listed neither 003 nor 004, and ADR 0014 was Proposed while ADRs 0015, 0016 and 0018, all Accepted, amended it. Each was copied by hand, and the only check was a diff-scoped, advisory review that never sees a file the PR does not touch. Spec Kit itself never reconciles `spec.md`, `plan.md` and `tasks.md`; it leaves the maintenance model to the project ([Spec persistence models](https://github.github.com/spec-kit/concepts/spec-persistence.html)). Feature 004, task W042.

## Decision drivers

* One home per fact (AGENTS.md, DRY); a copy is a fact with two homes.
* A wrong page that looks authoritative is the docs version of plausible but wrong output (Constitution III).
* A PR that makes a sentence false rarely touches the file that holds it, so a diff-scoped review cannot be the guard.
* Agents cannot change a decision's status; a check that meets one must report it, not fix it.

## Considered options

* 1: **Flow-forward**: each feature directory is a historical record; nothing in it changes after it is planned. Status lives only in code and `docs/`.
* 2: **Flow-back**: every discovery is reconciled back into spec, plan and tasks, kept live for the feature's life.
* 3: **Flow-forward hybrid**: spec and plan flow forward (ADR 0003's freeze); `tasks.md` ticks and as-built notes are the one live part of a feature directory; every other statement of status or a count is generated from them or caught by a check.

## Decision outcome

Chosen option, recommended: **3**. Option 1 loses the task list as the factory's work queue, which needs live ticks. Option 2 is the hand reconciliation that drifted [solid].

The model, as `AGENTS.md` "Specs and docs" already states it in part:

* **Spec and plan record intent and decisions**, and stop changing once the code they describe lands (ADR 0003). A later change edits the code and its as-built docs.
* **`tasks.md` is live**: a tick and its as-built note land in the PR that finishes the task.
* **Status is stated on one page** (`docs/start/status.md`); its counts are generated from the ticks, and its prose is hand-written.
* **A copied count or status claim anywhere else fails a check**, or, where a check cannot tell, is graded blocking in review.
* **A lint that meets a decision only the maintainer can make reports it through a reviewed exceptions file**, never by editing the decision (004's W-017).

What enforces it (004, Phase 8):

| Task | Mechanism | Kind |
|---|---|---|
| W037 | `scripts/gen-status.py` writes the status counts into an include the `docs` step generates before building the book; it is never committed, so it cannot go stale | Generated |
| W038 | `scripts/check-docs-facts.sh` fails on a copied task count outside the include and warns on unlinked status words | Gate (hygiene) |
| W039 | `scripts/check-adr-lifecycle.sh` fails when an Accepted ADR amends or supersedes one that does not record it, with exceptions in `scripts/adr-lifecycle-exceptions.txt` | Gate (hygiene) |
| W040 | `docs-reviewer` runs on `tasks.md`, `README.md`, `CONTRIBUTING.md` and `docs/adr/`, and grades a status claim outside the status page 🔴 | Review |
| W041 | `scripts/report-tick-lag.sh` lists tasks named in `main`'s commits but still unticked | Report only |

### Consequences

* Good, because the three drifts found on 2026-10-20 become a failing check or a listed exception, not a reader's surprise.
* Good, because the checks run on every PR over the whole tree, so the PR that makes a sentence false fails even when it never touched that sentence.
* Bad, because a bare `mdbook build` needs the include generated first; `scripts/check.sh docs` does it.
* Bad, because the copied-facts check matches patterns, so a count phrased another way passes; review (W040) is the backstop.
* Bad, because ticks can still lag merged work; W041 reports the lag but never gates on it, since a tick waits on a person for `human:` tasks.

### Confirmation

* `scripts/check.sh` generates the counts in its `docs` step and runs the two hygiene checks on every PR; each has a test under `scripts/tests/` that fails before it and passes after.
* The tick-lag report's baseline is recorded in its PR; a growing lag is the signal to revisit.

## Pros and cons of the options

### Option 1: flow-forward

* Good, because a feature directory never changes after planning.
* Bad, because the factory picks work from live ticks.

### Option 2: flow-back

* Good, because every artifact always says the latest.
* Bad, because it is reconciled by hand, which is what drifted.

### Option 3: flow-forward hybrid

* Good, because exactly one live artifact per feature, and every copy of it is generated or checked.
* Bad, because it needs the generator, two checks and a review rule to hold.

## More information

Builds on ADR 0003. Decisions: W-016 (generated, not `mdbook-cmdrun`), W-017 (exceptions file) in `specs/004-docs-site/plan.md`. Default if unanswered: the checks still land as 004 tasks; this ADR stays Proposed and the model stays implicit in `AGENTS.md`. Revisit if the exceptions file grows past a handful of lines.
