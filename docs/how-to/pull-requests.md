# How to size and open a pull request

The binding rules are in [`AGENTS.md`](../../AGENTS.md#commits-and-prs). This page explains the reasoning and works through the cases you will actually hit.

## Why small PRs, specifically here

This project's worst failure mode is a pattern that looks right and is wrong — you find out after cutting fabric. Review is the main defence, and review quality collapses with size. A 2,000-line PR gets approved, not reviewed.

Geometry makes this sharper than usual. A sign error in a dihedral angle, an epsilon compared against the wrong scale, a `HashMap` iteration on an output path — each is a one-line defect that produces plausible-looking output. None survives a focused review of 200 lines. All three survive a sweep through 2,000.

## The budget

**400 changed lines of hand-written source.**

Not counted:

| Excluded | Why |
|---|---|
| Fixture models (`fixtures/meshes/**`) | Binary; reviewed by looking at them, not by reading a diff |
| Generated TypeScript types | Reviewed by checking the generator, once |
| Snapshot files (`*.snap`, `fixtures/golden/**`) | Reviewed as rendered output in the PR comment |
| Lock files | Reviewed by `cargo deny` |
| Docs prose (`docs/**`) | Docs ship with the change they describe, and counting them would push them out of the PR; the docs-reviewer agent checks them instead |

Going over is allowed when the work is genuinely indivisible — one algorithm, one solver, one parser. Say so in a `**Size**` section and name what makes it indivisible. These are real:

> **Size**: 520 lines. The SLIM interior solve is one energy, one line search, and one convergence test; splitting it leaves a half-solver that cannot be tested against the developable oracle.

These are not:

> ~~**Size**: 1,400 lines. Implements US1.~~
> ~~**Size**: 900 lines. Easier to land together.~~

## What goes in one PR

**The test task plus the implementation that makes it pass.** Commit the test first, then the implementation, so the history shows test-first and CI is green at the PR tip.

```
001/42-T050-obj-import
├── test(io): OBJ import preserves per-corner UVs and materials   ← fails
└── feat(io): OBJ import via tobj                                  ← passes
```

A reviewer reads the test commit to learn what was promised, then the implementation to check it was delivered. That ordering is the whole point and it is lost if they land separately.

### Why not a PR per task, strictly

Because `AGENTS.md` also says tests come first and never weaken a test. A PR containing only a failing test cannot merge with green CI, and the ways to force it green — `#[ignore]`, `#[should_panic]`, commenting out the assertion — are all explicitly forbidden. So the test and its implementation travel together. Everything else stays one task per PR.

## Worked examples

### Several tiny tasks, one PR

`T009`–`T012` are the Cargo workspace, `deny.toml`, `pnpm-workspace.yaml`, and `rustfmt.toml`. Four tasks, one concern (workspace configuration). One PR:

```
chore/workspace-config     Tasks: T009, T010, T011, T012
```

Splitting these into four PRs is ceremony, not review.

### One task, several PRs

`T060` is the weld step. It is one task and it will exceed the budget, because welding needs a spatial hash, a deterministic tie-break, and the glTF split-vertex handling that is the entire reason it matters. Split by capability, each with its own test:

```
001/57-T060a-weld-spatial-hash      the hash and its determinism test
001/58-T060b-weld-gltf-split-verts  the case that makes or breaks glTF import   (stacked on 57)
```

Then annotate `T060` in `tasks.md` with both branch names so the task's history is findable. Splitting a task is fine; losing the trace is not.

### A task that must not be bundled

`T079` prescribes boundary edge lengths — decision `D-013`, the one the entire pattern stage rests on, with an unverified injectivity risk tracked as spike `S7`. It gets its own PR with its own review, however small the diff. Size is not the only reason to isolate a change; blast radius is.

## PR description

```markdown
## What
One or two sentences. What changed, not how.

## Why
The task, and what it unblocks.

## Task
Task: T050 (specs/001-pattern-pipeline/tasks.md)

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

## Size
Only if over budget. What makes the change indivisible.
```

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

`NNN` is the feature, `NN` is the slice's position in it (assigned when the PR opens, never reused), and the slug is what the slice contains. Stacked slices merge in `NN` order. You should be able to pick any branch and know what it is and what it waits on without opening it.

### Why stack

Spec Kit work is a chain: the plan cites the research, the data model implements the plan, the tasks cite all of them. Code is the same: the weld step needs the mesh types.

If every slice branches from `main` instead:

- Each PR's diff is correct, but its links point at files that don't exist on that branch, so a reviewer can't follow them.
- Merge order lives only in prose ("merge after `002/research`"), and nothing enforces it.
- Nobody can check out one branch and see the feature as it will land.

Stacking fixes all three. Each PR targets its parent, so its diff is still one slice. The top branch of the stack is the whole feature.

### How

```zsh
git checkout 002/02-research
git checkout -b 002/03-plan
# work, commit, push
git push -u origin 002/03-plan
# open PR: base = 002/02-research
```

The PR body's `Stack` section (from the template) lists position, parent, and child. When you open a child, edit the parent's body to link it, so the chain can be walked from either end.

**No depth cap.** The old cap of 2 assumed a deep stack shows reviewers changes they haven't read. That isn't true when each PR targets its parent: the diff shown is always exactly one slice.

### Merging a stack

The repo uses **merge commits only**:

| Strategy | What happens to the PR above |
|---|---|
| Merge commit ✅ | Its commits are already on `main` byte-for-byte. GitHub retargets it to `main` and the diff is just that slice. Nothing to do. |
| Squash ❌ | `main` gets a new commit; the child still carries the original. The child's diff against `main` shows the parent's changes again until someone runs `git rebase --onto`. It also destroys the test → implementation commit order this policy requires. |
| Rebase-merge ❌ | Same SHA problem as squash, less severe. |

Steps:

1. Merge the bottom PR (the only one whose base is `main`).
2. Its branch is deleted on merge, and GitHub retargets the next PR to `main`.
3. Repeat.

Never merge a PR whose base is another slice. That merges it into the parent's branch, not into `main`, and the stack falls apart quietly.

### Changing a lower slice

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
- No CI job measures PR size; reviewers enforce the budget. Whether a large PR is justified is a judgement a CI job cannot make, and a check that merely verified a `Size` section exists would be satisfied by typing anything at all.

## Checklist before opening

- [ ] Under 400 lines, or `**Size**` explains why not
- [ ] Test commit precedes implementation commit
- [ ] CI green at the tip — not green because a test was weakened
- [ ] One story, one crate, one concern
- [ ] Branch is `NNN/NN-slug` (or `chore/slug`); stacked on its parent if it depends on unmerged work
- [ ] No golden snapshot updated to make a test pass ([`AGENTS.md`](../../AGENTS.md))
- [ ] Task ID referenced, or the change is in the trivial-change lane
- [ ] `Skipped` section filled in honestly
- [ ] Public changes have their as-built docs, or `Docs` says n/a
