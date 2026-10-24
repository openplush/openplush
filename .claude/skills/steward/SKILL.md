---
name: "steward"
description: "How an agent drives an OpenPlush pull request (CI, review findings, stacks, merges, check-ins) without wasting review runs or tokens. Read before acting on a PR's CI or review events."
---

# Driving a pull request

The binding rules are in `AGENTS.md` (Commits and PRs). This file covers how to run the loop. Each rule here exists because skipping it once cost a review run or a full-context turn.

## Reviews

- `.github/workflows/review.yml` reviews a PR once, when it opens or is marked ready. A push starts no review.
- Never add `rereview` or `mutation-audit`: only the maintainer requests a rerun (ADR 0013). If the review run failed without a summary, say so to the maintainer once.
- Fix a 🔴 yourself and prove it before pushing: the test that shows the defect fails before and passes after, and, for a missing check, the mutation it should kill. Then reply on the thread with the commit and resolve it.
- A PR is ready to merge when CI is green on its head, the review found no 🔴 that is not fixed and proven, no 🔴 thread is open, and every 🟡 has been fixed or answered.
- Push only to fix a 🔴, a failing check, or a defect you have confirmed. Answer the other 🟡 findings in one short PR comment, written as `AGENTS.md`'s status prose, and resolve their threads.
- If a finding repeats an earlier one, link to the earlier answer rather than writing the answer again.

## Local review

- Don't run `pr-reviewer`, `spec-reviewer` or `docs-reviewer` locally: the review on opening runs them, and a rerun reviews only what changed since.
- Run `mutation-auditor` once before opening a PR that adds checks to Rust under `crates/` or `bindings/`, in its own worktree (`isolation: "worktree"`) so it never shares a checkout with other work. Close each real survivor with a test and re-run that mutation to prove the test bites.
- At most two subagents at a time.

## Stacks

- Stack interdependent intents rather than merging them into one diff early. Merge a fix to a lower PR upward only after that PR merges, unless a child needs the change to build or to make sense. GitHub retargets the child when its parent merges, and merging `main` then carries every fix up in one push.
- Before every push: `git grep -n '^<<<<<<< \|^>>>>>>> '` finds nothing.
- Merge commits only, bottom-up, with the expected head SHA; auto-merge only on a PR whose base is `main` (Merging). Never force-push.
- Folding a stack at the merge gate (AGENTS.md): merge each child into its parent upward, update the remaining PR's title and `Intent` to list every intent, close each folded PR with a link to it. Never fold a blast-radius PR or a decision into its code.

## Merging

[ADR 0014](../../../docs/adr/0014-auto-merge-tiers.md) and [ADR 0018](../../../docs/adr/0018-hands-off-merging.md) set who merges; the merge gate ([ADR 0015](../../../docs/adr/0015-merge-gate.md), `.github/workflows/merge-gate.yml`) computes the tier from the diff and enables auto-merge. A PR's tier is the highest any of its paths or tasks reaches. Label it `tier:A`, `tier:B`, `tier:C` or `tier:H` on opening as your reading of the diff; the gate raises a label that is too low and never lowers one.

Which paths put a PR in which tier, and who merges each: [the tier table](../../../docs/how-to/review-and-merge.md#tiers).

- Read the gate's comment on the PR (it starts "Merge gate"): it lists the tier and each condition as pass or fail, and `merge:ready` is its verdict. Never enable auto-merge yourself, on any tier, and never apply `merge:ready`: the gate removes what it did not set.
- On a stall, fix what the comment names: a failing check, an open thread (answer it, then resolve it: an unresolved thread is an unanswered finding to the gate), a conflict, a base that is not `main`, or a reviewer's changes requested. A push revokes the verdict; the gate judges the new head when its CI completes.
- The review summary must name the current head. After a push the gate adds `rereview` itself, once per head, and judges the new summary; you never add the label. If the review ran on a head without posting a summary, the comment says the maintainer must look.
- Never merge a PR yourself, on any tier. Tier H waits for the maintainer; the comment says so.

## Waiting

- After opening a PR, subscribe to it, so CI and review results wake you. Also schedule one safety-net check-in of about 30 minutes, for what sends no event (a conflict with `main`, another agent's push); re-arm it silently while nothing changed. Unsubscribe when the PR merges or closes, and cancel the check-in.
- Every check-in starts with the PR's `mergeable_state`. `dirty` means a conflict with `main`, and then CI and the review never start: merge `origin/main` in, resolve, push.
- Before reading CI or a review, check it is for the PR's current head SHA; an event for an older head says nothing about the new one.
- A check not started 15 minutes after the head was pushed, or not finished 60 minutes after it started, is stalled (runners start within minutes; the slowest job takes under 30). Report it once, to the user, and stop polling it; look again only on the next event or check-in.
- A turn that would only say "nothing new" is not worth taking. Report when a merge needs the maintainer, something is blocked, or a PR has merged.

## Pushing and posting

- Chain gates and git steps with `&&` (`scripts/check.sh engine && git commit … && git push …`), so a failing gate is never committed or pushed.
- The server appends a "Generated by Claude Code" footer to PR bodies and comments, which `AGENTS.md` (Commits and PRs, Identity) forbids: edit it out after posting. Review-thread replies cannot be edited with the GitHub tools, so answer findings in one PR comment and resolve the threads without replying in them. The server also escapes `>` in titles, so keep `>` out of them.
- Tick each task the PR finishes, with its as-built note, in the same PR. A tick for work that landed earlier goes in the next PR that touches `tasks.md`, or in one PR titled `docs(tasks): tick …`, which the review skips.

## Sessions

- One intent per session. When it merges or the context grows long, write a short handoff for a fresh session in status prose (`AGENTS.md`): merged PRs, open PRs with their heads, next steps, these rules rather than carrying on.
- Read only the files a step needs: grep `tasks.md` and the plan's ledger for the IDs in hand instead of reading them whole. Ask GitHub for one thread or the latest summary, not every thread on the PR.
- Check `main`, the open PRs and the next free branch position, decision and task numbers once, at the start of a session.
