---
status: Accepted
date: 2026-10-18
deciders: Lavender (maintainer)
---

# Size pull requests by intent, not by line count, and review each one once

## Context and problem statement

Since 2026-10-13 a pull request has been "one reviewable slice": at most 400 changed lines of hand-written source, one test task plus its implementation, one story and one crate. The budget kept early reviews focused, but on algorithm work it cut tasks into pieces that cannot be judged alone. T079 landed as three PRs (#194, #195, #197), each with its own review run, CI run and merge, and no single PR showed the boundary laid flat end to end. Each split also added stack bookkeeping and a separate PR just to tick the task.

Review runs cost the maintainer's subscription. They were spent twice on one PR within ten minutes, because the PR steward skill told agents to add the `rereview` label after fixing a 🔴 finding, sometimes with `mutation-audit` beside it, and each label is a separate trigger.

The maintainer set two goals on 2026-10-18: size a pull request by its intent, a finishable set of tasks toward an epic, so that `git blame` → PR answers what brought a change in and the idea behind it, with interdependent intents landing in order or folded together at the merge gate; and spend fewer automated review runs.

## Decision drivers

* The defect this project fears most is plausible-looking wrong geometry, so review must still be able to check what a PR claims.
* A reviewer judges a change by whether it does what it says, which needs the whole capability in view, not a third of it.
* Fewer review runs, CI runs and merges per finished task.
* A change traced back to its PR explains itself: what brought it in, and why.
* Interdependent work lands in order without forcing it into one diff early.

## Considered options

* 1: keep the 400-line budget and only stop agents from requesting reruns
* 2: raise the budget, say to 1,000 lines
* 3: size a PR by intent: one finishable outcome, with no line budget; one automated review per PR

## Decision outcome

Chosen option: **3**, because it is the only one that stops splitting a capability across PRs, and because a reviewer checks a stated intent against its tests, which a line count never measured. Option 1 keeps the splitting cost; option 2 moves the line without removing it.

* **An intent PR** brings in one thing toward an epic (a feature, or a bigger issue) and says why: one or more tasks that, merged, leave something done end to end, stated in one sentence. The test is traceability: someone who follows `git blame` to the PR, months later, must find what brought the change in and the idea behind it. The PR title states the intent, so the merge commit carries it. Every task the PR touches is finished, and ticked, in it. There is no line budget.
* **Agents formulate the intents.** Tasks and issues collect toward an epic; before opening PRs the agent groups them by intent, rather than by size or by task.
* **Readable by commit.** Each task is a test commit followed by the implementation commit that makes it pass, so a large PR is reviewed commit by commit. The PR body names the tasks and a reading order.
* **Stacked intents** carry interdependencies: X must land, then bug Y is fixed, then feature Z builds on both. Each is a PR stacked on the one before. At the merge gate, a stack whose intents only make sense together may be folded into one PR: merge each child into its parent upward (merge commits, so every intent's commits keep their history), list every intent in the remaining PR's `Intent` section, and close the folded PRs with a link to it.
* **Never in one PR**: two unrelated intents, or chores mixed with feature work. A decision the maintainer must accept (a ledger entry, an ADR) is its own intent, merged before the code that implements it; it is never folded into that code. Inside one PR, a refactor, a fix and a feature stay separate commits.
* **Still isolated**: the changes `tasks.md` lists for their blast radius get a PR of their own and are never folded.
* **One automated review per PR**, on opening. Agents never add `rereview` or `mutation-audit`: they fix findings, verify the fix locally (tests, and re-running a mutation the fix was meant to kill) and answer the thread. Only the maintainer requests a rerun, with `rereview`, or `mutation-audit` alone for one that includes the mutation auditor. When both labels are added in one operation, the workflow runs once.
* **Ticks ride along**: a PR ticks the tasks it finishes; no separate tick PR.

### Consequences

* Good, because a capability lands and is reviewed whole, with one review, one CI run and one merge, and its PR explains it to whoever traces it later.
* Good, because review runs drop to one per PR unless the maintainer asks for more.
* Bad, because a large PR is harder to review in one sitting; the commit order and the reading order in its body are the mitigation, and a PR whose intent needs "and" is split.
* Bad, because a defect a rerun would have caught after a fix is now caught only by the agent's local verification and CI. Every 🔴 fix must still be shown to work before pushing.
* Supersedes the budget clause in [ADR 0003](0003-as-built-docs.md) ("`docs/` prose does not count against the 400-line PR budget"): there is no budget to count against. Docs still ship with the change.
* The 400-line limit on a single module (`scripts/check-hygiene.sh`) is a code-structure rule, not a PR rule, and stays.

## More information

The single review per PR is amended by [ADR 0018](0018-hands-off-merging.md) for pushes after the review: the merge gate adds `rereview` once per head whose newest summary is for another head, so no head merges on a summary that did not read it. Agents still never add the label.
