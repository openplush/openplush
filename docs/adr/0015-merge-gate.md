---
status: Accepted
date: 2026-10-20
deciders: Lavender (maintainer)
---

# A workflow computes the tier and enables auto-merge; agents never do

## Context and problem statement

[ADR 0014](0014-auto-merge-tiers.md) sets the tiers and leaves the mechanics to the agent that opens a PR: it labels the tier from the paths, judges the conditions, and enables auto-merge. The ADR names its own hole: "nothing in CI checks the label yet. A mislabelled C PR would auto-merge. Revisit with a workflow that computes the tier from the diff." Events also reach only the session that opened a PR, and that session is gone by the time CI finishes, so a tier-A PR waits for the next factory firing to enable auto-merge.

This ADR supersedes the merge mechanics of ADR 0014: who computes the tier, who enables auto-merge, and what the visible verdict is. The tiers themselves, their paths and the tick exception are unchanged.

## Decision drivers

* One reviewer-independent decision: the same diff must get the same tier whoever opened the PR, and a PR must never merge because an agent mislabelled it.
* A new push must never merge before its own CI and review state have been judged.
* A stalled PR must explain itself without anyone reading workflow logs.
* The gate must not be switchable off, or widened, by the PR it is judging.
* Rollout must be reversible without a PR: the maintainer must be able to stop it from the repository settings.
* `main`'s own CI must keep running after a merge (ADR 0014, Confirmation).

## Considered options

* 1: keep ADR 0014 as written; agents label and enable auto-merge
* 2: a GitHub Actions workflow computes the tier from the diff, judges the conditions, labels `merge:ready` and enables auto-merge; agents only read its verdict
* 3: option 2 with an LLM judging whether 🟡 replies answer their findings

## Decision outcome

Chosen option: **2**. Option 1 is the hole ADR 0014 names. Option 3 puts a non-deterministic step in the decision path for little gain: the thread-resolution proxy below is deterministic, and a 🟡 is optional by definition.

* **The gate is one workflow, `.github/workflows/merge-gate.yml`, with no LLM in the decision path.** It runs when a CI or review workflow completes, on PR events (opened, synchronize, reopened, edited, labeled, unlabeled), and every 30 minutes as the backstop for what sends no event: a review thread being resolved, the tier-B hold expiring, and comments posted with `GITHUB_TOKEN`, which start no workflow [solid].
* **The tier is computed from the diff** by `scripts/pr-tier.py`, from the changed files (status, previous name, patch), the title and the labels, by the rules of ADR 0014. A rename counts both its old and its new path. A PR of more than 3000 changed files, the API's listing cap, is tier C: the gate cannot see all of it. The tick exception holds only when the title starts `docs(tasks): tick`, every file is a `specs/*/tasks.md`, and every changed line is a `- [ ]` → `- [x]` flip of an unchanged task line with an as-built note appended; a flip of a `human:` task is not a tick. A `tier:` label is never lowered: the effective tier is the higher of the computed one and the label, and when the computed one is higher the gate relabels and says so once.
* **`merge:ready` is the gate's output.** The gate applies it when every condition holds on the current head, and removes it, and disables auto-merge, the moment one stops holding, including on every push. Nobody else sets it; a hand-applied `merge:ready` is ignored and removed at the next evaluation. The gate keeps one comment per PR, updated in place, with the tier and each condition's pass or fail, so a stalled PR explains itself.
* **The conditions** on the current head are those of ADR 0014, made checkable:
  * not a draft, head in this repository, base `main`, mergeable with no conflict;
  * every required CI check (`fmt, clippy, test`, `licenses and advisories`, `typecheck, unit, e2e`, `build and test on wasm32`) completed with success or skipped on the head commit; a review check (`changed paths`, `adversarial review`) that exists on the head commit must have too, but its absence is not a failure, because the review runs once per PR (ADR 0013) and never on a later head;
  * the review: its newest summary for the PR reports 🔴 0, or every thread containing 🔴 is resolved, or the review skipped the PR by `review.yml`'s own rules (a `docs(tasks): tick` title, a `chore/` branch, or a docs-only diff);
  * **no unresolved review thread.** This is the proxy for "every 🟡 is fixed or answered": `steward` has agents answer a finding and then resolve its thread, so an open thread is an unanswered finding. The gate does not read the replies.
  * tier A; or tier B and at least 24 hours since the latest required check on the head completed; tier C never gets the label, and the comment says the maintainer merges.
* **The gate enables auto-merge** (merge commit, matched to the head commit so a push between judgement and merge cannot merge unjudged) with a GitHub App's installation token, not `GITHUB_TOKEN`: a merge made with `GITHUB_TOKEN` starts no `push` workflow, so `main`'s CI would silently stop running [solid]. The App needs contents write, pull requests write and checks read; its ID and private key are repository secrets. Without them the gate skips with a notice.
* **The gate never checks out or runs the PR's code.** It runs on `pull_request_target`, `workflow_run` and `schedule`, all of which take the workflow from the base branch, reads the PR through the API, and checks out only `scripts/` from the default branch. A PR therefore cannot change the gate that judges it, which `pull_request` would allow [solid].
* **Rollout by a repository variable, `MERGE_GATE_MODE`**: unset or `off` runs nothing; `dry-run` computes, comments and labels `merge:ready` but never touches auto-merge; `on` enables auto-merge too. It ships as `dry-run`. The maintainer flips it after reading a handful of verdicts, and can stop the gate from the settings page with no PR.
* **Agents read the verdict.** `steward`, Merging becomes: read the gate's comment; never enable auto-merge yourself; on a stall, fix what the comment names. `factory` no longer enables auto-merge on its walk of the open PRs.

### Consequences

* Good, because the tier is computed the same way for every PR, and a mislabelled PR can no longer merge.
* Good, because a tier-A PR merges as soon as its last check completes, with no session awake.
* Good, because a push revokes `merge:ready` and auto-merge at once, so no head merges before the gate has judged it.
* Good, because the maintainer can read why a PR waits, and can stop the gate without a PR.
* Bad, because "no unresolved thread" trusts that whoever resolved a thread answered it. An agent that resolves without answering passes the gate; the review summary and the thread history still show it. Revisit with a comment-only check if it happens.
* Bad, because the backstop is a 30-minute schedule, so a thread resolved by hand merges up to 30 minutes later, and the schedule spends a run every half hour whether or not a PR is open.
* Bad, because the gate needs a GitHub App, two secrets and a variable that only the maintainer can create.
* Bad, because the review check cannot be a required status check in the ruleset: it exists only on the head the review ran on, and a ruleset requiring it would block every PR pushed after its review. The gate judges the review itself, so the ruleset requires only the four CI checks.
* Supersedes, in ADR 0014: "The agent that opens the PR labels it … and relabels it"; "the agent enables auto-merge"; "Before pushing to a PR with auto-merge on, the agent disables it". The tiers, paths and tick exception stand.

### Confirmation

* `scripts/pr-tier.py` has a table-driven test per ADR 0014 rule, run by `scripts/check.sh`, and `scripts/merge-gate.py` has one per condition.
* `scripts/check.sh workflows` lints the workflows with a pinned `actionlint`.
* Settings, done by the maintainer: "Allow auto-merge" on; a ruleset on `main` requiring a pull request and the four CI checks, not requiring branches to be up to date (that stalls stacked and auto-merged PRs; a merge queue is the later upgrade if semantic conflicts appear); the App installed with its secrets; `MERGE_GATE_MODE=dry-run`.
* The ledger and PR history show no tier-C PR with `merge:ready`.

## Pros and cons of the options

### Option 1: agents label and enable auto-merge

* Good, because no new workflow, App or secret.
* Bad, because the tier is only as right as the agent's reading, and only the session that opened the PR acts on its events.

### Option 2: a deterministic workflow

* Good, because one rule, one place, testable, and visible on the PR.
* Bad, because it needs an App token and a schedule, and judges "answered" by thread resolution only.

### Option 3: option 2 with an LLM reading the 🟡 replies

* Good, because an empty reply to a finding would not pass.
* Bad, because a non-deterministic step would sit in the merge decision, cost a credential, and spend the subscription on optional findings.

## More information

Accepted on 2026-10-20 after the first evening in `dry-run`. Every verdict was checked by hand against ADR 0014 and the PR's state on GitHub, and every one was right: tier C from `crates/` and `specs/` paths (#252, #253, #254), from `docs/adr/` (#257) and from `scripts/` (#258); tier A through the tick exception with `merge:ready` applied once CI was green and the review skipped (#256); "base is main" refused for a stacked PR (#253, #258) and granted once the parent merged; a verdict revoked on push and re-judged on the new head six minutes after its CI completed (#254). The verdicts are recorded in the 002 factory ledger (#241).

One limit stays with the conventions rather than the gate: the review row accepts the newest review summary without checking that it was written for the current head, because a push starts no new review (steward). The rule that a push only fixes a 🔴 or a confirmed defect is what keeps that honest.

The `dry-run` → `on` switch is the maintainer's, in the repository's Actions variables, and needs no PR.

[ADR 0018](0018-hands-off-merging.md) supersedes two clauses: "tier C never gets the label" (the gate merges tier C; a new tier H is what the maintainer merges), and the review condition's acceptance of the newest summary whatever its head (the summary must name the current head, and the gate asks for a review when it does not). It also adds a condition: a human's "changes requested" blocks every tier.
