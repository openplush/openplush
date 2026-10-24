---
status: Accepted
date: 2026-10-20
deciders: Lavender (maintainer)
---

# The merge gate merges every tier; the maintainer keeps only the files that define the gate and the rules

## Context and problem statement

[ADR 0014](0014-auto-merge-tiers.md) reserved tier C for the maintainer: engine crates, specs, decisions, dependency manifests, CI and the agent rules. [ADR 0015](0015-merge-gate.md) built the gate that merges tiers A and B. Agents could merge tier C only under a time-boxed authorization the maintainer gave in their own words. In practice nearly every PR of substance is tier C: feature 002 is Rust under `crates/`, its decisions are rows in `specs/*/plan.md`, and its ticks need those to merge first. An agent that cannot merge tier C stacks PRs until a person returns, and an authorization that has to be renewed every few hours is that person returning on a timer. The maintainer decided, 2026-10-20, that merging should need them only where it must.

The first runs also showed two gaps in the gate's review condition: it trusted the newest review summary without checking it was written for the current head, and it ignored a human's "changes requested". With tier C auto-merging, both have to close.

## Decision drivers

* The maintainer's time is the scarce resource; a merge that needs a person must be rare and worth the person.
* A gate that can merge changes to its own rules, its own code, or the project's constitution is no gate; those files need a person whatever else merges.
* Code no review has seen must not merge; a push after the review must bring the review back, without a person adding a label.
* A human who asks for changes must be heard by the gate, on every tier.
* Everything else the gate already checks (CI on the head, resolved 🔴 threads, no open thread, base `main`, no conflict) stays.

## Considered options

* 1: keep the time-boxed authorizations, and have an agent merge tier C under them
* 2: the gate auto-merges tier C too, with one human tier for the rules, the gate and the constitution, and the two review gaps closed
* 3: option 2 with no human tier at all

## Decision outcome

Chosen option: **2**. Option 1 puts the judgement back in an agent's session and in a prompt, which ADR 0015 moved out of both for a reason, and makes the maintainer renew the authorization every few hours. Option 3 lets a PR change `merge-gate.py`, a workflow, `AGENTS.md` or the constitution and merge itself; the files that decide what merges cannot be judged by what they decide.

* **Tier C auto-merges once ready, with no hold.** Its paths: `crates/`, `bindings/`, `specs/`, and the dependency manifests, lockfiles and `.cargo/` as ADR 0014 lists them. A Decision Ledger row in `specs/*/plan.md` is tier C: a decision an agent records and the review has read merges like the code it governs. The conditions are tier A's.
* **A new tier H, the maintainer merges.** Any path under `.github/`, `scripts/`, `.claude/`, `docs/adr/`; any `AGENTS.md` or `CLAUDE.md`; `CONSTITUTION.md`, `GOVERNANCE.md`, `CODE_OF_CONDUCT.md`; the files whose change passes CI silently: `deny.toml`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.gitattributes`, `LICENSE-*`; a tick of a `human:` task; a PR of more than 3000 files; a PR the maintainer labels `tier:H`. The gate never labels `merge:ready` or merges tier H and its comment says so. Tiers A and B are unchanged; the tick exception stands.
* **The review summary must be for the current head.** The summary's first line names the head it reviewed. When the newest summary is for another head, the review condition fails and the gate itself adds the `rereview` label, once per head; `review.yml` removes the label when it starts, posts a new summary for the head, and the gate judges that. If the review for a head completes without a summary, the gate asks no second time and the comment says the maintainer must look. This supersedes ADR 0013's "one automated review per PR" for pushes after the review: agents still never add `rereview` or `mutation-audit`; the gate does, and only when the summary is stale.
* **A human's "changes requested" blocks every tier.** The gate reads the PR's review decision; `CHANGES_REQUESTED` fails a condition until the reviewer re-reviews or dismisses it.
* **Agents never merge, enable auto-merge, apply `merge:ready` or add `rereview`.**

### Consequences

* Good, because agents' engine and spec work merges as it goes, stacks move on their own, and the maintainer merges only a PR that changes the rules, the gate, CI, the constitution or the licences.
* Good, because the two review gaps close for every tier, not only C: no head merges on a summary written for another head, and a person's objection stops a merge.
* Good, because the merge decision is a tested script on the PR, not an agent reading a clock.
* Bad, because engine code and decisions now merge with no person having read them. The automated review, CI and the ledger are what stand between a wrong geometry and `main`; the maintainer reads merged history and can revert. The constitution's "no silent wrong output" is enforced by tests and typed errors, not by this gate.
* Bad, because the mutation audit stays on request (`mutation-audit`), so a tier C PR can merge without one. Requiring it would make every engine PR wait on a label only the maintainer adds, which is the queue this ADR removes. The maintainer adds the label when they want the audit, and the gate waits for its summary like any other.
* Bad, because every push after a review spends a review run on tier C, which ADR 0013 set out to avoid. Pushes after a review are meant to be 🔴 fixes only (`steward`); the run is the price of merging them seen by no one.
* Bad, because a review that fails to post a summary stalls the PR until the maintainer looks, by design.
* Supersedes, in ADR 0014: tier C's "the maintainer merges", and the list of paths that put a PR there. Supersedes, in ADR 0015: "tier C never gets the label", and the review condition's acceptance of the newest summary whatever its head. Supersedes, in ADR 0013: the single review per PR, for pushes after it.

### Confirmation

* `scripts/pr-tier.py` and `scripts/merge-gate.py` have table-driven tests for tier H, the stale-summary rereview and its once-per-head cap, and the changes-requested condition, run by `scripts/check.sh shell`.
* `scripts/sync-labels.sh` creates `tier:H`; the gate labels it.
* Settings, done by the maintainer: `MERGE_GATE_MODE=on`; the ruleset on `main` still requires only the four CI checks (the review check exists only on reviewed heads and is judged by the gate).
* The ledger and PR history show no tier-H PR with `merge:ready`, no PR merged on a summary for another head, and no PR merged by an agent.

## Pros and cons of the options

### Option 1: an agent merges tier C under time-boxed authorizations

* Good, because no gate change.
* Bad, because the merge decision lives in a session that may be gone, judged by a prompt and a clock rather than a tested script, and the maintainer has to renew the authorization every few hours.

### Option 2: the gate merges tier C; tier H stays human

* Good, because one deterministic rule, tested, visible on the PR; and the gate cannot change itself.
* Bad, because engine code merges unread by a person, and post-review pushes cost review runs.

### Option 3: no human tier

* Good, because nothing ever waits.
* Bad, because the gate, the workflows, the rules and the constitution would merge under their own judgement.

## More information

The maintainer's own working preferences, which have them merging geometry crates, specs and ADRs, predate this decision; this ADR is the newer instruction and the preferences are theirs to update.

Revisit if a tier-C merge has to be reverted for a defect the review and CI missed: the first such revert is the case for a hold on tier C like tier B's, or for the mutation audit as a gate condition.
