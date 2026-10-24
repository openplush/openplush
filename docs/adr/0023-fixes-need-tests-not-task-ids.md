---
status: Proposed
date: 2026-10-24
deciders: Lavender (maintainer)
---

# A fix to built code needs a failing test, not a task ID

## Context and problem statement

The constitution's workflow and `AGENTS.md` required a spec ID and a task ID for every non-trivial PR. A bug found in built code, such as a reader that panics on a damaged file, then needs a new line in a `tasks.md` before anyone may fix it, and that line only records what the fix's own test and PR already say. Should a fix to built code need a task ID?

## Decision drivers

* Constitution IV: test-first with oracles; the failing test is what proves a fix.
* Keep one home per fact: a task line that restates a PR is a second copy.
* Contributor burden: the ID bookkeeping made small fixes slow to land.

## Considered options

* Keep the rule: every fix gets a task ID first.
* A fix to built code needs a failing test first and no task ID; new feature work keeps its spec and task IDs.

## Decision outcome

Chosen option: **a fix needs a failing test, not a task ID**. New features still go through the spec flow and its task IDs. The trivial-change lane is unchanged. Fix branches are named `fix/slug`. The constitution moves to 0.1.1, a clarification of the workflow, not a principle.

### Consequences

* Good, because a found bug can be fixed in one PR: the failing test, then the fix.
* Bad, because a fix is no longer listed in a `tasks.md`; its PR and commits are its record.

### Confirmation

`CONSTITUTION.md`'s Development Workflow and `AGENTS.md`'s Workflow section state the rule; the PR template's Tasks section accepts "Fix: no task ID".
