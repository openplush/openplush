# How to scope and open a pull request

<!-- index: sizing by intent, stacking, branch names, and the PR template. -->

This page takes you from a set of finished tasks to pull requests a reviewer can judge: how to group the tasks into intents, lay out the commits, name and stack the branches, and merge. The binding rules live in [`AGENTS.md`](https://github.com/openplush/openplush/blob/main/AGENTS.md#commits-and-prs); this page links to them, explains the reasoning, and works through the cases you will actually hit.

## You will

Have one pull request per intent, its commits in test-then-implementation order, stacked on its parent if it depends on unmerged work, ready for its one review.

## Before you start

- Read [Commits and PRs in `AGENTS.md`](https://github.com/openplush/openplush/blob/main/AGENTS.md#commits-and-prs): the rules this page explains.
- Open your feature's `tasks.md` and find its PR packaging section, which lists the tasks to batch and the tasks to isolate, as in [feature 001's PR packaging](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/tasks.md#pr-packaging).
- If you are an agent that will drive the PR through CI and review, also read the [`steward` skill](https://github.com/openplush/openplush/blob/main/.claude/skills/steward/SKILL.md).

## Steps

### 1. Group the tasks by intent

An intent is **what the PR brings in toward an epic (a feature, or a bigger issue), and why**, in one sentence, with the PR title saying the same. The test comes later: someone runs `git blame` on a line, lands on the PR, and has to learn what brought it in and the idea behind it. A line count never answered that; an intent does.

Group the tasks you have collected by intent, not by size and not one PR per task:

> The flattener lays a chart's boundary flat with every edge at its 3D length. *(one task: boundary prescription)*
> A chart flattens end to end: SLIM interior, fallback when the boundary cannot close, and step-wise progress for the worker. *(three tasks, one outcome: "a chart flattens")*

Not one intent:

> ~~OBJ import and the weld step.~~ *(two outcomes)*
> ~~Half of the SLIM solver.~~ *(not finishable: nothing to check it against)*

No CI job judges whether a PR is one intent; reviewers do. A PR whose intent needs "and" to join two results is split.

Every task the PR touches is finished, and ticked with its as-built note, in the same PR. There is no line budget; a large PR stays reviewable because of how its commits are laid out (step 2).

**Why one intent per PR.** This project's worst failure mode is a pattern that looks right and is wrong, and you find out after cutting fabric. Review is the main defence, and a reviewer catches that kind of defect by checking a stated outcome against its tests: does the boundary come out at its 3D lengths, does a developable chart flatten with zero distortion. That needs the whole capability in one place. The line budget this replaced cut capabilities into pieces that could not be judged alone: [boundary prescription](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/tasks.md) landed as three PRs (closing the polygon, finding the boundary loop, and the curvature step), and none of them showed a chart's boundary laid flat. Each piece cost a review run, a CI run and a merge.

The cases you will meet:

- **Several tiny tasks, one PR.** The [workspace setup tasks](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/tasks.md) are the Cargo workspace, `deny.toml`, `pnpm-workspace.yaml` and `rustfmt.toml`: four tasks, one intent (the workspace is configured), one PR on `chore/workspace-config`.
- **One task, one PR, however large.** The [weld step](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/tasks.md) is a spatial hash, a deterministic tie-break, and the glTF split-vertex handling that is the reason it matters. One task, one outcome ("welded geometry"), one PR, committed as three test-then-implementation pairs if that reads best.
- **A change that must not be bundled.** The [seam-parity assertion](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/tasks.md#isolate-these--own-pr-regardless-of-size) is the last line of defence against the failure this project exists to prevent. It gets a PR of its own, however small the diff, because blast radius, not size, decides isolation. Each feature's `tasks.md` lists these.
- **Never together.** Two unrelated intents, or chores with feature work, never share a PR, and inside one PR a refactor, a fix and a feature are separate commits, so each reads as what it is ([`AGENTS.md`](https://github.com/openplush/openplush/blob/main/AGENTS.md#commits-and-prs)).

### 2. Lay out the commits

Each task is a test commit, then the implementation that makes it pass. A reviewer reads the test commit to learn what was promised, then the implementation to check it was delivered. In a PR with several tasks the commits run task by task. The PR that brings in the SLIM interior solve and the boundary fallback, two tasks, reads:

```
001/115-slim-interior
├── test(flatten): a developable chart flattens with no distortion   ← task 1, fails
├── feat(flatten): SLIM interior solve                                ← task 1, passes
├── test(flatten): an unclosable boundary is refused with darts      ← task 2, fails
└── feat(flatten): boundary fallback                                  ← task 2, passes
```

The PR's `Reading order` section says where to start. Commit by commit, a 1,500-line PR is still a series of small, checkable steps.

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/): `feat(flatten): …`, `fix(mesh): …`, `docs(spec): …`.

**Why not a PR per task.** A PR containing only a failing test cannot merge with green CI, and the ways to force it green (`#[ignore]`, `#[should_panic]`, commenting out the assertion) are all forbidden by the [workflow rules](https://github.com/openplush/openplush/blob/main/AGENTS.md#workflow). So a test and its implementation always travel together. Beyond that, splitting a capability into one PR per task buys a review of a part nobody can judge alone.

### 3. Name the branch and stack it

The [branch name rules](https://github.com/openplush/openplush/blob/main/AGENTS.md#commits-and-prs) give a list you can read without opening anything:

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

`NNN` is the feature, `NN` is the PR's position in it (assigned when the PR opens, never reused), and the slug is the intent it carries. Stacked PRs merge in `NN` order. You should be able to pick any branch and know what it is and what it waits on.

If your intent depends on another that has not merged, branch from that intent's branch and open the PR against it:

```zsh
git checkout 002/02-research
git checkout -b 002/03-plan
# work, commit, push
git push -u origin 002/03-plan
# open PR: base = 002/02-research
```

The PR body's `Stack` section lists position, parent, and child. When you open a child, edit the parent's body to link it, so the chain can be walked from either end.

**Why stack.** Spec work is a chain: the plan cites the research, the data model implements the plan, the tasks cite all of them. Code is the same: the weld step needs the mesh types. If every PR branches from `main` instead:

- Each PR's diff is correct, but its links point at files that don't exist on that branch, so a reviewer can't follow them.
- Merge order lives only in prose ("merge after `002/research`"), and nothing enforces it.
- Nobody can check out one branch and see the feature as it will land.

Stacking fixes all three. Each PR targets its parent, so its diff is still just that PR, and the top branch of the stack is the whole feature. For the same reason there is **no depth cap**: the old cap of 2 assumed a deep stack shows reviewers changes they haven't read, which isn't true when the diff shown is always exactly one PR.

**Stacked intents.** Intents often depend on each other: X must land first, then bug Y is fixed, then feature Z is built on both. Each is its own PR, stacked on the one before, so each keeps a clear "what and why":

```
001/120-chart-provenance      X: charts keep their 3D provenance          base: main
001/121-weld-tie-break-fix    Y: weld ties break by vertex ID, not order   base: 120
001/122-grain-on-pieces       Z: grain arrows follow the 3D direction      base: 121
```

At the merge gate the stack sometimes turns out to make sense only as a whole: Z is the reason X and Y exist, and nobody would review X alone. Then it is folded as [`AGENTS.md`](https://github.com/openplush/openplush/blob/main/AGENTS.md#commits-and-prs) describes: here, 122 merges into 121 and 121 into 120, 120 is retitled and lists all three intents in its `Intent` section, and 121 and 122 close with a link to 120. Merge commits keep each intent's commits, so `git blame` still lands on them, and the one PR explains all three. A change isolated for blast radius, or a decision the maintainer must accept, is never folded into the code around it: each must be judged on its own.

**Agent sessions.** Agent harnesses assign their own branch names (`claude/<slug>-<id>`), which are scratch. When a session cannot push the real name, the PR title and body carry it and the stack position, so a maintainer can rename the branch on GitHub, which keeps the PR.

### 4. Write the description

[`.github/pull_request_template.md`](https://github.com/openplush/openplush/blob/main/.github/pull_request_template.md) pre-fills the description with each required section as an HTML comment saying what goes there, so an unfilled section is visibly empty rather than silently absent. Fill every section.

You are responsible for everything you submit, whoever or whatever wrote it; an AI coding agent's work is yours once you open the PR.

`Skipped` is the section people leave empty because it feels like an admission. It is the most useful one: an unstated gap becomes a surprise later, and the [definition of done](https://github.com/openplush/openplush/blob/main/AGENTS.md#definition-of-done) asks for it precisely so deferral is a decision rather than an oversight.

### 5. Open it for its one review

The automated review runs when the PR opens; when it runs again is in [Get a pull request reviewed and merged](review-and-merge.md). Its findings arrive as review comments marked 🔴 blocking, 🟡 optional or 🟣 pre-existing; answer every 🔴, by fixing it or by replying with why not. Each run costs the maintainer's subscription, and reruns used to be spent twice on one PR within ten minutes. How a finding is fixed, proven and answered is in [`steward`, Reviews](https://github.com/openplush/openplush/blob/main/.claude/skills/steward/SKILL.md#reviews).

### 6. Merge the stack bottom-up

Who merges depends on the PR's tier, which the merge gate computes from the diff; the tiers and the gate's conditions are in [Get a pull request reviewed and merged](review-and-merge.md). [`.github/workflows/merge-gate.yml`](https://github.com/openplush/openplush/blob/main/.github/workflows/merge-gate.yml) judges every merge condition on the current head and keeps one comment per PR saying which fail.

The repo uses merge commits only, because of what each strategy does to the PR above:

| Strategy | What happens to the PR above |
|---|---|
| Merge commit ✅ | Its commits are already on `main` byte-for-byte. GitHub retargets it to `main` and the diff is just that PR. Nothing to do. |
| Squash ❌ | `main` gets a new commit; the child still carries the original. The child's diff against `main` shows the parent's changes again until someone runs `git rebase --onto`. It also destroys the test-then-implementation commit order step 2 requires. |
| Rebase-merge ❌ | Same SHA problem as squash, less severe. |

So a stack merges from the bottom:

1. The bottom PR (the only one whose base is `main`) merges.
2. Its branch is deleted on merge, and GitHub retargets the next PR to `main`; the gate judges the retargeted PR on its next event or on its scheduled backstop.
3. Repeat.

Only a PR whose base is `main` is merged. Merging one whose base is another PR's branch merges it into the parent's branch, not into `main`, and the stack falls apart quietly.

**Changing a lower PR.** Review asks for a change in `002/02-research` while `03`–`05` are open:

```zsh
git checkout 002/02-research   # fix, commit, push
git checkout 002/03-plan && git merge 002/02-research && git push
git checkout 002/04-data-model-quickstart && git merge 002/03-plan && git push
# ...up to the top
```

Merge upward, don't rebase: rebasing forces a force-push on every branch above, which throws away review context. When to carry a fix up early and when to wait for the parent to merge is in [`steward`, Stacks](https://github.com/openplush/openplush/blob/main/.claude/skills/steward/SKILL.md#stacks).

## Check it worked

- The `Intent` sentence states one outcome without "and", and the title says the same.
- The commit list alternates a failing test and the implementation that makes it pass, task by task.
- The diff GitHub shows is only this PR's change, and its `Stack` section names the parent and child.
- CI is green at the tip, and not because a test was weakened or a golden snapshot updated ([`AGENTS.md`, Workflow](https://github.com/openplush/openplush/blob/main/AGENTS.md#workflow)).
- Every box in the template's closing checklist is true.

**Decisions**: [ADR 0013](../adr/0013-intent-sized-pull-requests.md) (intent-sized PRs, one review), [ADR 0014](../adr/0014-auto-merge-tiers.md) (merge tiers), [ADR 0015](../adr/0015-merge-gate.md) (merge gate), [ADR 0018](../adr/0018-hands-off-merging.md) (the gate merges tier C; tier H is the maintainer's).
