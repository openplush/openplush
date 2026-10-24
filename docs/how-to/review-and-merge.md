# How to get a pull request reviewed and merged

<!-- index: the automated review, answering its findings, and when the maintainer merges. -->

This page takes an open pull request through the automated review to `main`. Use it once your PR is open; how to scope and open one is in [Scope and open a pull request](pull-requests.md).

## You will

Have your PR ready for the maintainer to merge.

## Before you start

- A PR opened from the template, with CI running ([Scope and open a pull request](pull-requests.md)).
- What the review is for, and why it works this way: [How agents work](../explanation/agents.md#the-review-on-opening).

## Steps

1. **Wait for the review summary.** The review runs when the PR opens or is marked ready for review, except on a `docs(tasks): tick …` PR, a PR from a `chore/` branch, and a PR that changes only files under `docs/`. It posts a comment that starts `Review summary (PR N, head SHA)` and review comments on the lines it found something on, each marked 🔴 blocking, 🟡 optional or 🟣 pre-existing.
2. **Answer every 🔴.** Fix it, with a test that fails before the fix and passes after, or reply on its thread saying why it stands. Answer the 🟡 findings in one comment on the PR, saying which you fixed and why the rest stay. A 🟣 finding was there before your PR and needs nothing.
3. **Resolve each thread you answered.**
4. **Push your fixes in one go.** A push starts no new review. Only the maintainer adds the `rereview` label, to ask for another run, or `mutation-audit`, for a run that also checks that the tests catch deliberate bugs. A rerun reviews only what changed since the last summary.
5. **Get the PR ready to merge.** On its current head: CI green, no open 🔴, no unresolved thread, no conflict, and base `main`. For a stacked PR, the base turns to `main` when the PR below it merges.
6. **Let the maintainer merge it.** The maintainer merges every PR, with a merge commit, bottom-up through a stack. Agents never merge or turn on auto-merge.

## Check it worked

The PR shows as merged into `main`.
