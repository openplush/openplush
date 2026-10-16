---
name: "steward"
description: "How an agent drives an OpenPlush pull request (CI, review findings, stacks, merges, check-ins) without wasting review runs or tokens. Read before acting on a PR's CI or review events."
---

# Driving a pull request

The binding rules are in `AGENTS.md` (Commits and PRs). This file covers how to run the loop. Each rule here exists because skipping it once cost a review run or a full-context turn.

## Reviews

- `.github/workflows/review.yml` runs when a PR opens or is marked ready, and again only when the `rereview` label is added. A push alone starts no review.
- Add `rereview` only after a run failed (a usage-limit stop, a crash) or after pushing a fix to a 🔴 finding. Never add it just to get a summary on a newer commit.
- A PR is ready to merge when CI is green on its head, a review on a recent commit found no 🔴, no 🔴 thread is open, and every 🟡 has been fixed or answered.
- Push only to fix a 🔴, a failing check, or a defect you have confirmed. Answer the other 🟡 findings in one short PR comment, and resolve their threads.
- If a finding repeats an earlier one, link to the earlier answer rather than writing the answer again.

## Local review

- Code PRs: run `pr-reviewer`, plus `mutation-auditor` for Rust under `crates/` or `bindings/`, once before opening the PR. The review the PR gets on opening is the gate.
- Docs and spec PRs: no local reviewers. The review on opening covers them.
- At most two subagents at a time.

## Stacks

- Merge a fix to a lower slice upward only after that slice merges, unless a child needs the change to build or to make sense. GitHub retargets the child when its parent merges, and merging `main` then carries every fix up in one push.
- Before every push: `git grep -n '^<<<<<<< \|^>>>>>>> '` finds nothing.
- Merge commits only, bottom-up, with the expected head SHA. Never force-push.

## Waiting

- Subscribe only to PRs you are actively changing. While a PR only waits on CI, a review, or the maintainer, schedule one check-in instead of reacting to every CI event.
- A turn that would only say "nothing new" is not worth taking. Report when a merge needs the maintainer, something is blocked, or a PR has merged.

## Sessions

- One task per session. When a task ends or the context grows long, write a short handoff for a fresh session (merged PRs, open PRs with their heads, next steps, these rules) rather than carrying on.
- Read only the files a step needs. Ask GitHub for one thread or the latest summary, not every thread on the PR.
