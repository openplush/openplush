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

Going over is allowed when the work is genuinely indivisible — one algorithm, one solver, one parser. Say so in a `**Size**` section and name what makes it indivisible. These are real:

> **Size**: 520 lines. The SLIM interior solve is one energy, one line search, and one convergence test; splitting it leaves a half-solver that cannot be tested against the developable oracle.

These are not:

> ~~**Size**: 1,400 lines. Implements US1.~~
> ~~**Size**: 900 lines. Easier to land together.~~

## What goes in one PR

**The test task plus the implementation that makes it pass.** Commit the test first, then the implementation, so the history shows test-first and CI is green at the PR tip.

```
001/T050-obj-import
├── test(io): OBJ import preserves per-corner UVs and materials   ← fails
└── feat(io): OBJ import via tobj                                  ← passes
```

A reviewer reads the test commit to learn what was promised, then the implementation to check it was delivered. That ordering is the whole point and it is lost if they land separately.

### Why not a PR per task, strictly

Because `AGENTS.md` also says tests come first and never weaken a test. A PR containing only a failing test cannot merge with green CI, and the ways to force it green — `#[ignore]`, `#[should_panic]`, commenting out the assertion — are all explicitly forbidden. So the test and its implementation travel together. Everything else stays one task per PR.

## Worked examples

### Several tiny tasks, one PR

`T010`–`T012` are `deny.toml`, `pnpm-workspace.yaml`, and `rustfmt.toml`. Three tasks, about 60 lines, one concern (workspace configuration). One PR:

```
chore/workspace-config     Tasks: T010, T011, T012
```

Splitting these into three PRs is ceremony, not review.

### One task, several PRs

`T060` is the weld step. It is one task and it will exceed the budget, because welding needs a spatial hash, a deterministic tie-break, and the glTF split-vertex handling that is the entire reason it matters. Split by capability, each with its own test:

```
001/T060a-weld-spatial-hash      the hash and its determinism test
001/T060b-weld-gltf-split-verts  the case that makes or breaks glTF import
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

## Size
Only if over budget. What makes the change indivisible.
```

`Skipped` is the section people leave empty because it feels like an admission. It is the most useful one in the template: an unstated gap becomes a surprise later, and `AGENTS.md` requires it precisely so deferral is a decision rather than an oversight.

## Stacking

Sometimes work genuinely depends on unmerged work. Then:

1. Branch from the parent branch, not `main`.
2. Target the parent branch in the PR.
3. Put `Stacked on #12` as the first line of the body.
4. When the parent merges, retarget to `main`. GitHub does this automatically for depth 1.

**Maximum depth 2.** At depth 3 the diff shown is relative to a branch the reviewer has not read, and nobody can tell what is new. Deeper than 2 means the first PR should have been merged already.

## Tooling

- [`.github/pull_request_template.md`](../../.github/pull_request_template.md) pre-fills the description with the required sections as HTML comments, so an unfilled section is visibly empty rather than silently absent.
- [`.github/workflows/pr-size.yml`](../../.github/workflows/pr-size.yml) measures the hand-written diff and comments when a PR is over budget. **It never fails and never blocks.** Whether a large PR is justified is a judgement a CI job cannot make, and a check that merely verified a `Size` section exists would be satisfied by typing anything at all.

## Checklist before opening

- [ ] Under 400 lines, or `**Size**` explains why not
- [ ] Test commit precedes implementation commit
- [ ] CI green at the tip — not green because a test was weakened
- [ ] One story, one crate, one concern
- [ ] No golden snapshot updated to make a test pass ([`AGENTS.md`](../../AGENTS.md))
- [ ] Task ID referenced, or the change is in the trivial-change lane
- [ ] `Skipped` section filled in honestly
