## What

<!-- One or two sentences. What changed, not how. -->

## Why

<!-- The task, and what it unblocks. -->

## Task

<!-- Task: T050 (specs/001-pattern-pipeline/tasks.md)
     Or: "Trivial-change lane — docs fix" (see AGENTS.md for what qualifies). -->

## Edge cases

<!-- Degenerate geometry, empty input, seam and boundary cases, scale extremes.
     Which apply, and what happens in each. -->

## Failure modes

<!-- Each one, and the typed error it produces. "Never clamps, skips, or fixes up
     bad geometry" is the standard - say how this change meets it. -->

## Tests

<!-- Added or changed, and what each proves. Name the oracle if there is one.
     The test commit should come BEFORE the implementation commit. -->

## Skipped

<!-- Anything deliberately deferred. Error handling you did not write. Be explicit:
     a reviewer cannot see an absence. Empty is a red flag, not a clean bill. -->

## Size

<!-- Only if over the 400-line budget. Name what makes the change indivisible.
     "Easier to land together" is not a justification. -->

---

<!-- Before requesting review:
     - [ ] Under 400 changed lines of hand-written source, or Size explains why not
     - [ ] Test commit precedes implementation commit
     - [ ] CI green at the tip, and not green because a test was weakened
     - [ ] One story, one crate, one concern
     - [ ] No golden snapshot updated to make a test pass
     - [ ] Skipped section filled in honestly
-->
