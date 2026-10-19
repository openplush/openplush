# How to scope and open a pull request

The binding rules are in [`AGENTS.md`](../../AGENTS.md#commits-and-prs); the decision is [ADR 0013](../adr/0013-intent-sized-pull-requests.md). This page explains the reasoning and works through the cases you will actually hit.

## Why one intent per PR

This project's worst failure mode is a pattern that looks right and is wrong — you find out after cutting fabric. Review is the main defence, and a reviewer catches that kind of defect by checking a stated outcome against its tests: does the boundary come out at its 3D lengths, does a developable chart flatten with zero distortion. That needs the whole capability in one place.

A line budget cut capabilities into pieces that could not be judged alone. Boundary prescription (T079) landed as three PRs, closing the polygon, finding the boundary loop, and the curvature step, and none of them showed a chart's boundary laid flat. Each piece cost a review run, a CI run and a merge. So the unit is the intent, not the line count.

## What an intent is

**What the PR brings in toward an epic (a feature, or a bigger issue), and why**, in one sentence, with the PR title saying the same. The test comes later: someone runs `git blame` on a line, lands on the PR, and has to learn what brought it in and the idea behind it. A line count never answered that; an intent does.

Tasks and issues collect toward an epic. Before opening PRs, group them by intent, not by size and not one PR per task:

> The flattener lays a chart's boundary flat with every edge at its 3D length. *(T079)*
> A chart flattens end to end: SLIM interior, fallback when the boundary cannot close, and step-wise progress for the worker. *(T080, T082, T083: one outcome, "a chart flattens")*

Not one intent:

> ~~OBJ import and the weld step.~~ *(two outcomes)*
> ~~Half of the SLIM solver.~~ *(not finishable: nothing to check it against)*

Every task the PR touches is finished, and ticked with its as-built note, in the same PR. There is no line budget. A large PR stays reviewable because of how its commits are laid out.

## How a PR is laid out

**Each task is a test commit, then the implementation that makes it pass.** A reviewer reads the test commit to learn what was promised, then the implementation to check it was delivered. In a PR with several tasks the commits run task by task:

```
001/115-slim-interior          Tasks: T080, T082
├── test(flatten): a developable chart flattens with no distortion   ← T080, fails
├── feat(flatten): SLIM interior solve                                ← T080, passes
├── test(flatten): an unclosable boundary is refused with darts      ← T082, fails
└── feat(flatten): boundary fallback                                  ← T082, passes
```

The PR's `Reading order` section says where to start. Commit by commit, a 1,500-line PR is still a series of small, checkable steps.

### Why not a PR per task

A PR containing only a failing test cannot merge with green CI, and the ways to force it green — `#[ignore]`, `#[should_panic]`, commenting out the assertion — are all forbidden (`AGENTS.md`). So a test and its implementation always travel together. Beyond that, splitting a capability into one PR per task buys a review of a part nobody can judge alone.

## Worked examples

### Several tiny tasks, one PR

`T009`–`T012` are the Cargo workspace, `deny.toml`, `pnpm-workspace.yaml`, and `rustfmt.toml`. Four tasks, one intent (the workspace is configured). One PR:

```
chore/workspace-config     Tasks: T009, T010, T011, T012
```

### One task, one PR, however large

`T060` is the weld step: a spatial hash, a deterministic tie-break, and the glTF split-vertex handling that is the reason it matters. One task, one outcome ("welded geometry"), one PR, committed as three test-then-implementation pairs if that reads best.

### A change that must not be bundled

`T087` is the seam-parity assertion, the last line of defence against the failure this project exists to prevent. It gets a PR of its own, however small the diff, because blast radius, not size, decides isolation. `tasks.md` lists these.

### Stacked intents

Intents often depend on each other: X must land first, then bug Y is fixed, then feature Z is built on both. Each is its own PR, stacked on the one before, so each keeps a clear "what and why":

```
001/120-chart-provenance      X: charts keep their 3D provenance          base: main
001/121-weld-tie-break-fix    Y: weld ties break by vertex ID, not order   base: 120
001/122-grain-on-pieces       Z: grain arrows follow the 3D direction      base: 121
```

At the merge gate the stack sometimes turns out to make sense only as a whole: Z is the reason X and Y exist, and nobody would review X alone. Then fold it: merge 122 into 121 and 121 into 120 (merge commits, so each intent's commits keep their history), retitle 120 and list all three intents, in order, in its `Intent` section, and close 121 and 122 with a link to 120. `git blame` still lands on each intent's own commits, and the PR explains all three.

Never fold a change `tasks.md` isolates for blast radius, or a decision the maintainer must accept into the code that implements it.

### Never together

Two unrelated intents; chores and feature work. Inside one PR, a refactor, a fix and a feature are separate commits, so each reads as what it is.

## PR description

```markdown
## Intent
One sentence: the outcome this PR finishes.

## Tasks
T080, T082 (specs/001-pattern-pipeline/tasks.md), ticked in this PR.

## Reading order
The commits in order, or the files to start from.

## Why
What it unblocks.

## Edge cases
Degenerate geometry, empty input, seam and boundary cases, scale extremes —
which ones apply, and what happens.

## Failure modes
Each one, and the typed error it produces.

## Tests
Added or changed, and what each proves. Name the oracle if there is one.

## Skipped
Anything deliberately deferred. Error handling you did not write. Say it
explicitly — a reviewer cannot see an absence.

## Docs
The as-built docs this PR updates, or "n/a: no public change".
```

## Review

The automated review runs once, when a PR opens. Only the maintainer asks for another run (the `rereview` label, or `mutation-audit` alone for one that includes the mutation auditor). An agent that fixes a 🔴 proves the fix with a test that fails before and passes after, then answers the thread; it does not request a rerun.

`Skipped` is the section people leave empty because it feels like an admission. It is the most useful one in the template: an unstated gap becomes a surprise later, and `AGENTS.md` requires it precisely so deferral is a decision rather than an oversight.

## Branches and stacks

### Reading the branch list

```
001/01-spec-clarifications
001/02-plan-research-data-model
001/03-contracts-engine-worker
001/04-contracts-project-schema
001/05-quickstart
001/06-tasks
002/01-spec
002/02-research
...
chore/pr-policy
```

`NNN` is the feature, `NN` is the PR's position in it (assigned when the PR opens, never reused), and the slug is the intent it carries. Stacked PRs merge in `NN` order. You should be able to pick any branch and know what it is and what it waits on without opening it.

### Why stack

Stack whenever one intent depends on another that has not merged (see Stacked intents above).

Spec Kit work is a chain: the plan cites the research, the data model implements the plan, the tasks cite all of them. Code is the same: the weld step needs the mesh types.

If every PR branches from `main` instead:

- Each PR's diff is correct, but its links point at files that don't exist on that branch, so a reviewer can't follow them.
- Merge order lives only in prose ("merge after `002/research`"), and nothing enforces it.
- Nobody can check out one branch and see the feature as it will land.

Stacking fixes all three. Each PR targets its parent, so its diff is still just that PR. The top branch of the stack is the whole feature.

### How

```zsh
git checkout 002/02-research
git checkout -b 002/03-plan
# work, commit, push
git push -u origin 002/03-plan
# open PR: base = 002/02-research
```

The PR body's `Stack` section (from the template) lists position, parent, and child. When you open a child, edit the parent's body to link it, so the chain can be walked from either end.

**No depth cap.** The old cap of 2 assumed a deep stack shows reviewers changes they haven't read. That isn't true when each PR targets its parent: the diff shown is always exactly one PR.

### Merging a stack

The repo uses **merge commits only**:

| Strategy | What happens to the PR above |
|---|---|
| Merge commit ✅ | Its commits are already on `main` byte-for-byte. GitHub retargets it to `main` and the diff is just that PR. Nothing to do. |
| Squash ❌ | `main` gets a new commit; the child still carries the original. The child's diff against `main` shows the parent's changes again until someone runs `git rebase --onto`. It also destroys the test → implementation commit order this policy requires. |
| Rebase-merge ❌ | Same SHA problem as squash, less severe. |

Steps:

1. Merge the bottom PR (the only one whose base is `main`): the maintainer for tier C, the merge gate for A and B.
2. Its branch is deleted on merge, and GitHub retargets the next PR to `main`; the gate judges the retargeted PR on its next event or within 30 minutes.
3. Repeat.

Never merge a PR whose base is another PR's branch. That merges it into the parent's branch, not into `main`, and the stack falls apart quietly.

### Changing a lower PR

Review asks for a change in `002/02-research` while `03`–`05` are open:

```zsh
git checkout 002/02-research   # fix, commit, push
git checkout 002/03-plan && git merge 002/02-research && git push
git checkout 002/04-data-model-quickstart && git merge 002/03-plan && git push
# ...up to the top
```

Merge upward, don't rebase. Rebasing forces a force-push on every branch above, which throws away review context.

### Agent sessions

Agent harnesses assign their own branch names (`claude/<slug>-<id>`). Those are scratch. A PR comes from an `NNN/NN-slug` branch. If the session cannot push one, the PR title and body must state the intended name and stack position so a maintainer can rename the branch on GitHub, which keeps the PR.

## Tooling

- [`.github/pull_request_template.md`](../../.github/pull_request_template.md) pre-fills the description with the required sections as HTML comments, so an unfilled section is visibly empty rather than silently absent.
- No CI job judges whether a PR is one intent; reviewers do. A PR whose `Intent` needs "and" is split.
- [`.github/workflows/merge-gate.yml`](../../.github/workflows/merge-gate.yml) ([ADR 0015](../adr/0015-merge-gate.md)) computes the PR's tier from its diff ([ADR 0014](../adr/0014-auto-merge-tiers.md)), judges every merge condition on the current head, keeps one comment per PR saying which fail, and applies `merge:ready` and auto-merge when all hold. Tier C waits for the maintainer. Nobody enables auto-merge by hand.

## Checklist before opening

- [ ] One intent; every task it touches finished and ticked here
- [ ] Each task's test commit precedes its implementation commit
- [ ] CI green at the tip — not green because a test was weakened
- [ ] One story; no spec change, refactor or chore mixed in
- [ ] Branch is `NNN/NN-slug` (or `chore/slug`); stacked on its parent if it depends on unmerged work
- [ ] No golden snapshot updated to make a test pass ([`AGENTS.md`](../../AGENTS.md))
- [ ] Task IDs referenced, or the change is in the trivial-change lane
- [ ] `Skipped` section filled in honestly
- [ ] Public changes have their as-built docs, or `Docs` says n/a
