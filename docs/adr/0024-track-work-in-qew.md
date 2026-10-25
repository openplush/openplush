---
status: Accepted
date: 2026-10-25
deciders: Lavender (maintainer)
---

# Track work in a qew ledger, and agents merge their own pull requests once CI passes

## Context and problem statement

Work was planned in three places: each feature's `tasks.md`, [`ROADMAP.md`](https://github.com/openplush/openplush/blob/e01752d/ROADMAP.md) for the order, and GitHub issues for anything else. None of them knows what is ready, blocked or proven done, so every agent session rebuilds that by reading files, and the maintainer merges every pull request by hand, which is the slowest step in the loop. The maintainer tried [qew](https://github.com/foundlings/qew), a file-based work ledger, on a practice repository on 2026-10-24: fifteen items went from ready to merged through thirteen pull requests, and qew refused every merge that had no recorded evidence. Where should OpenPlush's work live, and who merges?

## Decision drivers

* The maintainer is learning agent-driven development and wants one place that says what to do next, readable without decoding IDs.
* Constitution IV, test-first: a merge must rest on proof, not on an agent's word.
* Agents must not merge past failing or pending checks.
* No runtime change: the ledger is a development tool, so Constitution's privacy and license rules for shipped code are untouched [solid].

## Considered options

* 1: **qew ledger, agents merge after CI**: every item in a private ledger repository, mirrored to this repository's issues; an agent records evidence, opens the pull request with `qew pr open` and merges with `qew pr merge` once checks pass.
* 2: **qew ledger, the maintainer merges**: as 1, but every merge stays manual.
* 3: **Keep `ROADMAP.md` and `tasks.md`**.

## Decision outcome

Chosen option: **1**, the maintainer's choice of 2026-10-25.

* The ledger lives in [openplush/openplush-ledger](https://github.com/openplush/openplush-ledger) (private) and is checked out beside this repository; `.qew/config.toml` points at it. qew owns this repository's issue tracker (its default "repository" mode). Its first sync, on 2026-10-25, also imported the 12 issues filed before it as closed ledger items: it added the labels `P2` and `author:Lavender` and appended a qew marker and footer to each body. Their earlier text is kept in each issue's edit history, and they are left that way, because qew restores its own version of any edit made on GitHub [solid].
* `ROADMAP.md` is removed: its items are ledger items, in its order (each later group needs the one before), with the two that need a person (choosing a real plush model; printing and sewing it) held for the maintainer by the `needs:approval` label.
* A new feature's tasks are ledger items. The `tasks.md` files of features 001 to 004 stay as history (206 ticked lines, 204 open on 2026-10-25) and are not imported: the open lines are file-by-file plans, many of them stale. When a ledger item starts work on a feature, the open lines it covers that are still worth doing are filed as ledger items, and the rest are dropped.
* An agent merges its own pull request with `qew pr merge` only when every check has passed. qew refuses a merge whose head commit has no recorded evidence [solid]. The maintainer can stop the loop or revert a merge at any time.
* qew item IDs (`043bq0v2w`, epics `A`, `B`, …) are ledger IDs. They never appear in code, and status prose follows the AGENTS.md rule for IDs.

### Consequences

* Good, because `qew ready` answers "what now?" in one command, and dependencies decide the order instead of a list someone must keep current.
* Good, because a merge needs evidence on the exact commit, which is a stronger gate than a person's glance.
* Bad, because merges no longer wait for a human read; a wrong change can land and must be caught by tests or reverted. This is the same speed that produced 375 pull requests in ten days in October, so the maintainer watches the board.
* Bad, because qew needs Python 3.15, its hook installer fails on Windows (the hooks were added to `.claude/settings.json` by hand) and it runs from a source checkout there, not an install [solid].

### Confirmation

`qew check` passes on the ledger, `qew pr merge` refuses a pull request without evidence, and every merged pull request names its ledger item.

## Pros and cons of the options

### Option 2

* Good, because a person reads every change before it lands.
* Bad, because it keeps the slowest step, which the maintainer chose to remove.

### Option 3

* Good, because it needs no new tool.
* Bad, because readiness and proof stay in people's heads, and three lists drift apart.

## More information

This amends the merge rule in `AGENTS.md` and the tasks step of the constitution's development workflow. Revisit if merges start landing broken changes: option 2 is one line in `AGENTS.md` away.
