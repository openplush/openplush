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
- Push only to fix a 🔴, a failing check, or a defect you have confirmed. Answer the other 🟡 findings in one short PR comment, and resolve their threads.
- If a finding repeats an earlier one, link to the earlier answer rather than writing the answer again.

## Local review

- Don't run `pr-reviewer`, `spec-reviewer` or `docs-reviewer` locally: the review on opening runs them, and a rerun reviews only what changed since.
- Run `mutation-auditor` once before opening a PR that adds checks to Rust under `crates/` or `bindings/`, in its own worktree (`isolation: "worktree"`) so it never shares a checkout with other work. Close each real survivor with a test and re-run that mutation to prove the test bites.
- At most two subagents at a time. A `factory` firing may run up to four through the Workflow tool, one task each in its own worktree, with nothing else running beside them.

## Stacks

- Stack interdependent intents rather than merging them into one diff early. Merge a fix to a lower PR upward only after that PR merges, unless a child needs the change to build or to make sense. GitHub retargets the child when its parent merges, and merging `main` then carries every fix up in one push.
- Before every push: `git grep -n '^<<<<<<< \|^>>>>>>> '` finds nothing.
- Merge commits only, bottom-up, with the expected head SHA; auto-merge only on a PR whose base is `main` (Merging). Never force-push.
- Folding a stack at the merge gate (AGENTS.md): merge each child into its parent upward, update the remaining PR's title and `Intent` to list every intent, close each folded PR with a link to it. Never fold a blast-radius PR or a decision into its code.

## Merging

[ADR 0014](../../../docs/adr/0014-auto-merge-tiers.md) sets who merges. A PR's tier is the highest any of its paths or tasks reaches. Label it `tier:A`, `tier:B` or `tier:C` on opening, relabel it when a push changes the tier, and never lower a tier the maintainer set.

| Tier | When | Who merges |
|---|---|---|
| C | `crates/`, `bindings/`, `specs/`, `.specify/`, `docs/adr/`, `.github/`, `.claude/`, `scripts/`; any `AGENTS.md`, `CLAUDE.md`, `GOVERNANCE.md`, `CODE_OF_CONDUCT.md`; any `Cargo.toml`, `Cargo.lock`, `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `deny.toml`, `rust-toolchain.toml`, `.cargo/`; an ADR or Decision Ledger entry; a `human:` task; labelled `tier:C` by the maintainer | The maintainer |
| B | `fixtures/golden/`, `*.snap`, or an expected SVG or PDF fixture | Auto-merge, enabled 24 h after the head's CI went green |
| A | Everything else, and a `docs(tasks): tick …` PR whose diff touches only `specs/*/tasks.md` with ticks and as-built notes | Auto-merge, enabled once ready |

- Enable auto-merge, merge commit, on a tier A or B PR only when: its base is `main`; CI is green on the head with no conflict; the review found no 🔴 that is not fixed and proven, or it skipped the PR; every 🟡 is fixed or answered; no review thread is open. For B, also 24 h since the head's CI went green.
- Disable auto-merge before pushing to a PR that has it, and enable it again when the conditions hold on the new head.
- Never merge a PR yourself, and never enable auto-merge on tier C.

## Waiting

- After opening a PR, subscribe to it, so CI and review results wake you. Also schedule one safety-net check-in of about 30 minutes, for what sends no event (a conflict with `main`, another agent's push); re-arm it silently while nothing changed. Unsubscribe when the PR merges or closes, and cancel the check-in.
- Every check-in starts with the PR's `mergeable_state`. `dirty` means a conflict with `main`, and then CI and the review never start: merge `origin/main` in, resolve, push.
- Before reading CI or a review, check it is for the PR's current head SHA; an event for an older head says nothing about the new one.
- A turn that would only say "nothing new" is not worth taking. Report when a merge needs the maintainer, something is blocked, or a PR has merged.

## Pushing and posting

- Chain gates and git steps with `&&` (`scripts/check.sh engine && git commit … && git push …`), so a failing gate is never committed or pushed.
- The server appends a "Generated by Claude Code" footer to PR bodies and comments: edit it out after posting. Review-thread replies cannot be edited with the GitHub tools, so answer findings in one PR comment and resolve the threads without replying in them. The server also escapes `>` in titles, so keep `>` out of them.
- Tick each task the PR finishes, with its as-built note, in the same PR. A tick for work that landed earlier goes in the next PR that touches `tasks.md`, or in one PR titled `docs(tasks): tick …`, which the review skips.

## Sessions

- One intent per session, or one `factory` firing. When it merges or the context grows long, write a short handoff for a fresh session (merged PRs, open PRs with their heads, next steps, these rules) rather than carrying on.
- Read only the files a step needs: grep `tasks.md` and the plan's ledger for the IDs in hand instead of reading them whole. Ask GitHub for one thread or the latest summary, not every thread on the PR.
- Check `main`, the open PRs and the next free branch position, decision and task numbers once, at the start of a session.
