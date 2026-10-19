## Stack

<!-- Branch: NNN/NN-slug (e.g. 001/115-slim-interior) or chore/slug. The slug names the intent.
     Parent: "main", or the intent this one is stacked on (its base branch). Stack
     interdependent intents: X, then fix Y, then feature Z. Child: the PR stacked on this one,
     or "none". A folded stack lists here the PRs it absorbed.
     Merge: merge commit only, and only once this PR's base is main. -->

- **Branch**: 
- **Parent**: 
- **Child**: 

## Intent

<!-- One sentence: what this PR brings in toward which epic, and why. Someone following
     git blame here later must learn what brought the change in and the idea behind it.
     Unrelated results joined by "and" are two PRs (ADR 0013). A folded stack lists each
     intent, in merge order. -->

## Tasks

<!-- Every task this PR finishes, each ticked in tasks.md in this PR:
     T080, T082 (specs/001-pattern-pipeline/tasks.md)
     Or: "Trivial-change lane — docs fix" (see AGENTS.md for what qualifies). -->

## Reading order

<!-- How to review it: the commits in order (each task is a test commit, then the
     implementation that makes it pass), or the files to start from. -->

## Why

<!-- What this unblocks. -->

## Edge cases

<!-- Degenerate geometry, empty input, seam and boundary cases, scale extremes.
     Which apply, and what happens in each. -->

## Failure modes

<!-- Each one, and the typed error it produces. "Never clamps, skips, or fixes up
     bad geometry" is the standard - say how this change meets it. -->

## Tests

<!-- Added or changed, and what each proves. Name the oracle if there is one. -->

## Skipped

<!-- Anything deliberately deferred. Error handling you did not write. Be explicit:
     a reviewer cannot see an absence. Empty is a red flag, not a clean bill. -->

## Docs

<!-- The as-built docs this PR updates (rustdoc, docs/reference/, docs/explanation/),
     or "n/a: no public change". See AGENTS.md, Specs and docs. -->

---

<!-- Before requesting review:
     - [ ] Title and Intent say what this brings in and why; every task it touches finished and ticked here
     - [ ] Each task's test commit precedes its implementation commit
     - [ ] CI green at the tip, and not green because a test was weakened
     - [ ] No unrelated intent or chore mixed in; refactor, fix and feature in separate commits
     - [ ] Branch named NNN/NN-slug or chore/slug; base is the parent if this depends on unmerged work
     - [ ] No golden snapshot updated to make a test pass
     - [ ] Skipped section filled in honestly
     - [ ] Public changes have their as-built docs, or Docs says n/a
-->
