---
status: Accepted
date: 2026-10-20
deciders: Lavender (maintainer)
---

# A time-boxed standing authorization lets an agent merge tier C PRs

> Amended 2026-10-21 by [ADR 0020](0020-factory-grants.md): a grant is now its structured argument, at most 8 hours; the decisions clause stands, keyed to `decisions=yes`.

## Context and problem statement

Tier C PRs wait for the maintainer to merge them ([ADR 0014](0014-auto-merge-tiers.md), [ADR 0015](0015-merge-gate.md)), and almost all engine work is tier C, so a stack of factory PRs stalls whenever the maintainer is away. On 2026-10-19 a three-hour grant given in a session let an agent merge ten tier C PRs (#252 to #270) and accept decisions D-061 to D-063 on its own recommendation (ledger [#233](https://github.com/openplush/openplush/issues/233)). The grant lived only in chat and the ledger, set no bounds, and was read as continuing past its end after a usage-limit pause. The maintainer decided, 2026-10-20, that agents may merge tier C PRs under a time-boxed standing authorization, with bounds written down.

## Decision drivers

* Constitution III: plausible-looking wrong geometry is the defect this project fears most, so the checks a tier C merge relies on (CI, the review, the mutation audit) must still all hold.
* ADR 0014's reason for tier C: an agent must not be able to loosen the checks it is judged by.
* A grant must be checkable afterwards: who granted what, until when, and what was merged under it.
* Stacks of tier C PRs should land while the maintainer is away, when the maintainer chooses.

## Considered options

* 1: keep every tier C merge with the maintainer (ADR 0014 as it stands)
* 2: a standing authorization in the maintainer's words, bounded by time and scope, recorded in the ledger
* 3: lower engine PRs to tier A when CI, review and audit pass

## Decision outcome

Chosen option: **2**. Option 1 stalls stacks; option 3 removes the human merge from engine code for good, where the maintainer wants to grant it for a time they choose [solid].

**The grant.** A standing authorization is in the maintainer's own words, in a session or a comment on the feature's ledger issue. It names:

* an end time, as an absolute UTC time, at most **4 hours** after the grant;
* the feature or features it covers;
* whether it covers accepting decisions (below); it does not unless it says so.

Before its first merge under a grant, the agent posts the grant verbatim on the ledger issue, with its end time and scope. A grant that names no end time, or one more than 4 hours away, covers nothing until the maintainer gives the end time.

**The window is wall clock.** A pause (a usage limit, a crash, a context reset) does not extend it, and "continue" after a pause resumes work, not the grant. Once the end time passes, the agent merges nothing more: it drives its open PRs to green, writes the ledger entry and hands off. The maintainer ends a grant early by saying so.

**What a grant may merge.** A tier C PR for a covered feature, opened or driven in the run, once all of these hold on its current head:

* its base is `main`; stacks merge bottom-up, a merge commit, with the full expected head SHA;
* CI is green and there is no merge conflict;
* the automated review found no 🔴 that is not fixed and proven, every 🟡 is fixed or answered, and no review thread is open (`steward`, Reviews);
* for Rust under `crates/` or `bindings/`, the mutation audit ran and every survivor is closed by a test or recorded as equivalent in the PR body.

**What stays the maintainer's, grant or not.** A PR that changes any of:

* the gates and the agents' rules: `.github/`, `scripts/`, `.claude/`, any `AGENTS.md` or `CLAUDE.md`;
* the merge rules themselves: this ADR, ADR 0014 or ADR 0015, so a grant can never widen itself;
* a task with a `human:` token, a PR the maintainer labelled `tier:C` by hand, and expected output (tier B keeps its 24-hour hold).

**Decisions.** Under a grant that says it covers decisions, the agent may accept a Decision Ledger entry on its own recommendation, recorded as "Accepted — <maintainer> (maintainer), <date>: standing authorization (ledger #<n>)", and listed in the run's ledger entry for the maintainer to revisit. Without that, a decision stops the run as `factory` says.

**The record.** Each merge under a grant goes in the run's ledger entry with its merge SHA; so does each decision accepted, and any rule the run broke. The last entry of a run says when the grant ended and what is still open.

### Consequences

* Good, because a stack of engine PRs lands in one sitting while the maintainer is away, on the same checks the maintainer merges on.
* Good, because the grant, its window and every merge under it are on the ledger, so the maintainer can audit and revert.
* Good, because the paths that set the checks, and the rules that set who merges, still need the maintainer's merge.
* Bad, because engine code reaches `main` with no person having read it; a defect CI, the review and the audit all miss lands. The maintainer reads the ledger and the merged history after the run.
* Bad, because the window is enforced by the agent reading the clock, not by GitHub; nothing stops a merge past the end time but the rule and the ledger.
* ADR 0014's "C, the maintainer merges", `steward`'s "Never merge a PR yourself" and `factory`'s "Merge a PR itself" are amended to "unless a standing authorization covers it (ADR 0016)", in their own PR once this is accepted.

### Confirmation

* Every merge an agent makes under a grant appears in that run's ledger entry, between the grant's comment and its end time.
* No PR changing an excluded path carries an agent's merge.
* Revisit after the first three grants: merges per grant, and any that had to be reverted.

## Pros and cons of the options

### Option 1: the maintainer merges every tier C PR

* Good, because nothing in engine code merges unseen.
* Bad, because stacks stall whenever the maintainer is away, as before the 2026-10-19 grant.

### Option 2: a bounded standing authorization

* Good, because the maintainer chooses when merges may happen without them, and for how long.
* Bad, because it relies on the agent keeping to the window and the exclusions.

### Option 3: engine PRs to tier A on green checks

* Good, because it needs no grant.
* Bad, because it removes the human merge from the code Constitution III worries about, permanently.

## More information

The grant of 2026-10-19 set none of these bounds; under them, its merges after its end time (about 01:45 UTC) would not have been allowed.

[ADR 0018](0018-hands-off-merging.md), the same day, supersedes everything here about merging: the merge gate merges tier C itself, and an agent never merges, grant or no grant. The clause on accepting decisions under a grant that says it covers them stands.
