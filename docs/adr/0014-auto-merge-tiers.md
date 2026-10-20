---
status: Proposed
date: 2026-10-19
deciders: Lavender (maintainer)
---

# Merge by tier: agents enable auto-merge on low-risk PRs, the maintainer merges the rest

## Context and problem statement

Every PR waits for the maintainer to merge it, including tick PRs, docs fixes and shell changes that CI and the automated review have already passed. Factory firings open PRs hourly ([ADR 0013](0013-intent-sized-pull-requests.md), `.claude/skills/factory/SKILL.md`), so the merge queue grows faster than one person clears it, and stacked work waits on merges nobody needs to judge.

The maintainer asked, 2026-10-19, for three tiers: "A: CI green + review no 🔴 → agent enables auto-merge; B: same, held 24 h, for golden/SVG/PDF output changes; C: crates/, specs/, ADRs, decisions, human: → maintainer merges", and for the automated review to skip `docs(tasks): tick` PRs, `chore/` PRs and docs-only PRs.

## Decision drivers

* The defect this project fears most is plausible-looking wrong geometry (Constitution III); engine code and specs keep a human merge.
* The rules, gates and dependencies that every other check relies on must not change without the maintainer seeing it, or one unreviewed PR can switch the other checks off.
* Review runs cost the maintainer's subscription (ADR 0013); a PR that cannot hold a geometry defect does not need one.
* Factory firings must make progress without the maintainer present.

## Considered options

* 1: keep every merge with the maintainer
* 2: tiers by path and label, A/B/C as the maintainer stated them
* 3: option 2, with C also covering the rules, gates and dependency manifests

## Decision outcome

Chosen option: **3**. Option 1 is the queue this replaces. Option 2 alone has a hole: a `chore/` PR skips the review and lands in tier A, so a chore that edits `review.yml`, `scripts/check.sh` or a lockfile would merge with no person and no review having looked at it [solid]. Option 3 closes it by putting those paths in C.

A PR's tier is the **highest** that any of its changed paths or tasks reaches. The agent that opens the PR labels it `tier:A`, `tier:B` or `tier:C` and relabels it when a push changes the tier. An agent never lowers a tier the maintainer set.

* **C, the maintainer merges.** Any of:
  * a path under `crates/`, `specs/`, `.specify/`, `docs/adr/`, `.github/`, `.claude/`, `scripts/` or `bindings/`; an `AGENTS.md` or `CLAUDE.md`; `GOVERNANCE.md` or `CODE_OF_CONDUCT.md`;
  * a dependency manifest, lockfile or toolchain file: any `Cargo.toml`, `Cargo.lock`, `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `deny.toml`, `rust-toolchain.toml`, or `.cargo/`;
  * a root rules file whose change passes CI silently: `clippy.toml` (the determinism lints Constitution V relies on), `rustfmt.toml`, `.gitattributes` (`eol=lf` and `binary`, which keep fixtures byte-identical), or a `LICENSE-*` file;
  * a decision: an ADR, or a Decision Ledger entry added or changed;
  * a task with a `human:` token, or a PR the maintainer labels `tier:C`.

  One exception: a PR titled `docs(tasks): tick …` whose diff touches only `specs/*/tasks.md`, ticking tasks that already merged and adding their as-built notes, is tier A. Adding, removing or re-scoping a task is not a tick.
* **B, held 24 hours, then auto-merge.** A change to expected output: `fixtures/golden/`, a `*.snap` file, or an expected SVG or PDF fixture, in a PR no C path puts in tier C. Same conditions as A, and the agent enables auto-merge no sooner than 24 hours after the head commit's CI went green; a push restarts the clock. The hold gives the maintainer a day to look at the output diff.
* **A, auto-merge.** Everything else: `apps/` and `packages/` source, `docs/` outside `docs/adr/`, `fixtures/` outside expected output, `research/`, the README and CONTRIBUTING, and tick PRs. The agent enables auto-merge (merge commit) once all of these hold on the current head:
  * the PR's base is `main` (a stacked PR merged into its parent breaks the stack);
  * CI is green and there is no merge conflict;
  * the automated review found no 🔴 that is not fixed and proven (`steward`, Reviews), or it skipped the PR;
  * every 🟡 is fixed or answered, and no review thread is open.

  Before pushing to a PR with auto-merge on, the agent disables it, and enables it again when the conditions hold on the new head, so a push can never merge before its CI has run.

The automated review (`.github/workflows/review.yml`) skips, on opening, a PR titled `docs(tasks): tick …`, a PR from a `chore/` branch, and a docs-only PR (every changed path under `docs/`). The maintainer's `rereview` label still runs it on any of them.

### Consequences

* Good, because tick, docs, fixture and shell PRs stop waiting on the maintainer, and stacks move as soon as their base merges.
* Good, because review runs drop further: chores, ticks and docs-only PRs spend none.
* Good, because what the project fears most (engine code, specs, decisions) and what can switch the checks off (gates, rules, dependencies) still need the maintainer's merge.
* Bad, because a docs-only or tick PR merges with no review at all; a wrong as-built note or reference page can land. The maintainer can read the merged history, and the next docs-reviewer run on a public change compares docs against code.
* Bad, because the tier is applied by the agent from the paths; nothing in CI checks the label yet. A mislabelled C PR would auto-merge. Revisit with a workflow that computes the tier from the diff if that happens once.
* Bad, because tier B rarely applies today: agents never edit expected output (`AGENTS.md`), and expected output changes come with engine code, which is C. B covers a maintainer's or contributor's output update outside `crates/`.
* `CONTRIBUTING.md`'s "a maintainer still merges" and the factory skill's "never merge" are amended to the tiers.

### Confirmation

* GitHub settings, done by the maintainer before agents use the tiers: "Allow auto-merge" on, and a branch-protection rule on `main` requiring the CI checks (`fmt, clippy, test`, `licenses and advisories`, `typecheck, unit, e2e`, `build and test on wasm32`) and `adversarial review`, so auto-merge waits for them. A skipped review job counts as passing. Without required checks GitHub merges an auto-merge PR at once.
* The labels exist (`scripts/sync-labels.sh`).
* The ledger and PR history show no tier-C PR merged by an agent.

## Pros and cons of the options

### Option 1: the maintainer merges everything

* Good, because nothing merges unseen.
* Bad, because the queue grows with every firing, and stacks stall.

### Option 2: A/B/C as first stated

* Good, because it is the simplest rule.
* Bad, because skipping review on `chore/` PRs puts gate and dependency changes in tier A with no reviewer at all.

### Option 3: option 2 with gates, rules and dependencies in C

* Good, because the unreviewed lanes can no longer change the checks.
* Bad, because most chores still wait on the maintainer.

## More information

The merge mechanics (who labels the tier, who enables auto-merge, when it is disabled) are superseded by [ADR 0015](0015-merge-gate.md); the tiers and their paths stand. [ADR 0016](0016-standing-merge-authorization.md) lets an agent merge tier C PRs under a time-boxed standing authorization.

Revisit once the factory has run a week under the tiers: count PRs per tier, and any tier-A merge that had to be reverted.
