---
status: Accepted
date: 2026-10-22
deciders: Lavender (maintainer)
---

# Every feature numbers tasks T and decisions D-, renamed in place, and an ID outside its feature names the feature

## Context and problem statement

Each feature chose its own letters: tasks are `T` in 001, `H` in 002, `U` in 003 and `W` in 004, and decisions `D-`, `H-`, `S-` and `W-`. Spikes carry a second number that collides with those letters: `S7` is a spike in 001 and `S-014` a decision in 003; `H012`, `H2` and `H-003` are a task, a spike and a decision in 002; `Q` is a clarification question in 001 and 002 but a spike in 003. A reader decodes the kind from the feature, and agent status lines such as the ledger on [#241](https://github.com/openplush/openplush/issues/241) carry several such IDs each. Which scheme holds in every feature, and what happens to the IDs already written?

## Decision drivers

* An ID's kind is readable without knowing its feature.
* The Spec Kit task generator numbers tasks `T001`, `T002`, …; a scheme that matches it needs no override per feature.
* Git history, merged pull requests and ledger comments cannot be rewritten, so old IDs live on and must stay resolvable.
* The scripts that read task lines (`scripts/gen-status.py`, `scripts/report-tick-lag.sh`) match any `[A-Z][0-9]{3}`, and no script reads a decision or spike ID [solid].

## Considered options

* 1: **One letter per kind in every feature, renamed in place**, and an ID written outside its feature's files names the feature: `001 T084`, `003 D-014`.
* 2: **A hyphenated feature-scoped form everywhere**: `001-T084`, `003-D-014`.
* 3: **Fix only the collisions**: retire spike numbers, keep each feature's letters.
* 4: **Option 1 for new features only**, with 001 to 004 frozen behind an alias table.

## Decision outcome

Chosen option: **1**, the maintainer's choice of 2026-10-22.

* **Tasks** are `T` and **decisions** `D-` followed by three digits, in every feature. 001 already uses this form; 002 to 004 are renamed in place, each ID keeping its number.
* **Spike numbers are retired.** Every spike is already a task line (001's S1 to S8 are T001 to T008, 002's H1, H2, H4 and H5 are H005 to H008, 003's Q1 to Q4 are U008 to U011) [solid], so a spike is named by its task ID. Research folders named after a spike drop the ID (`research/h4_symmetry_margin` becomes `research/symmetry_margin`).
* **`Q` names only a clarification question** in a spec, and `R` a research section.
* **Inside a feature's own files an ID is bare**; anywhere else it names its feature, as the glossary already asks for requirement IDs (`001 FR-023`). Two features now share task numbers, so `002 T008` and `001 T008` are different tasks: the feature is part of the ID.
* **Old IDs map by rule**: in 002 to 004, swap the letter and keep the number (002 `H012` is `002 T012`, 003 `S-014` is `003 D-014`); a spike label maps to its task. The glossary states the rule with one row per feature.

Option 2 reads like a feature folder (`001-pattern-pipeline`) and doubles the hyphen on decisions; on task lines it breaks the status page's pattern. Option 3 leaves five task letters, so the kind still depends on the feature. Option 4 leaves 002 and 003, the features with the most open work, on the old letters for as long as they run.

This amends [ADR 0019](0019-spec-maintenance-model.md) by one exception: its rule that a spec and plan stop changing once their code lands does not apply to this rename, which changes IDs and no intent.

### Consequences

* Good, because a task or decision reads the same in every feature, and the next feature needs no letter of its own.
* Good, because every collision named above is gone, and the checks can reject any other form.
* Bad, because the rename touches every spec file in 002 to 004 once, and old IDs in history need the glossary's rule to read.
* Bad, because a bare `T008` outside its feature's files is ambiguous; the status-prose rule in `AGENTS.md` and the checks have to hold that line.

### Confirmation

`scripts/check.sh` fails on a task line or ledger row in any other form. The rename's pull request carries a script run that resolves every bare `T` and `D-` in a feature's files to that feature.

## Pros and cons of the options

### Option 1

* Good, because it matches Spec Kit and 001, the feature with the most IDs, so the largest set of IDs stays as it is.
* Bad, because the feature is required wherever the context does not give it.

### Option 2

* Good, because every ID is unique on its own.
* Bad, because task lines would no longer match the status page's pattern, and Spec Kit cannot emit it.

### Option 3

* Good, because it is the smallest change.
* Bad, because the kind of an ID still depends on its feature.

### Option 4

* Good, because nothing written changes.
* Bad, because two schemes run side by side for as long as 002 and 003 have open work.

## More information

Accepted by the maintainer on 2026-10-22.
