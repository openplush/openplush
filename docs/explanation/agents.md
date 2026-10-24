# How agents work in this repo

<!-- index: how AI agents take a feature from spec to merged pull request: specs, steward, the review on opening and who merges. -->

AI coding agents write most of OpenPlush's pull requests. This page explains, for a contributor who wants to know how that is organised, what the agents do from a feature idea to a merged change, and what keeps a person in charge. The rules themselves live in the skills and ADRs this page links to; it explains them and never replaces them.

## The short answer

Work starts as a feature: a spec, a plan and a numbered task list. An agent picks a ready task, writes the failing test and then the code, and opens one pull request per intent. An automated review runs once, when the PR opens. The maintainer merges every PR; agents never do.

## How it works

### Every rule has one home

Agents read [`AGENTS.md`](https://github.com/openplush/openplush/blob/main/AGENTS.md) first; `CLAUDE.md` only imports it. Rules for a single job live in a skill under [`.claude/skills/`](https://github.com/openplush/openplush/tree/main/.claude/skills), which an agent loads when it starts that job, and reviewers are described as agents under [`.claude/agents/`](https://github.com/openplush/openplush/tree/main/.claude/agents). Above all of them sits the [constitution](https://github.com/openplush/openplush/blob/main/CONSTITUTION.md), which wins any conflict. The reasoning behind a process rule is in an ADR, so a skill can stay short and point there.

### From an idea to tasks

OpenPlush is built spec first. Each feature gets a numbered folder under [`specs/`](https://github.com/openplush/openplush/tree/main/specs), and work moves through it in order: specify, clarify, plan, tasks, implement. Each file starts from a template in [`specs/templates/`](https://github.com/openplush/openplush/tree/main/specs/templates):

- `spec.md`: what the feature does and why, with no technology in it. Questions the spec leaves open are asked and answered in it before planning.
- `plan.md` and its companions: research, data model, contracts and a quickstart. The plan holds the Decision Ledger, where each design choice waits for the maintainer to accept it.
- `tasks.md`: numbered tasks, each naming its files and, where there is code, its test first. Agents then work through those tasks.

What each file in a feature folder holds is in the [specs index](https://github.com/openplush/openplush/blob/main/specs/README.md). The task list matters most to agents: every change that is not trivial maps to a task ID, and an agent with no task stops rather than invent scope. Tasks also carry tokens that say when they are ready: `needs:` names tasks that must land first, `blocked:` names a decision only the maintainer can clear, and `human:` marks work no agent picks up.

### Landing a task as a pull request

A task becomes a branch, a test commit that fails, and the commit that makes it pass. Tasks are grouped into pull requests by intent: one PR brings in one thing toward a feature and says why, so that someone who follows `git blame` back to it months later finds what brought the change in. There is no line budget. Work that depends on other unfinished work is stacked, each PR based on the one below it. [How to scope and open a pull request](../how-to/pull-requests.md) walks through this with examples, and [ADR 0013](../adr/0013-intent-sized-pull-requests.md) records why intents replaced the old 400-line limit.

### Steward: driving a pull request

Once a PR is open, the [`steward`](https://github.com/openplush/openplush/blob/main/.claude/skills/steward/SKILL.md) skill says how an agent sees it through: subscribe to the PR so CI and review results arrive as events, check in now and then for what sends no event (a conflict with `main`, another agent's push), fix what is blocking, answer what is not, and keep stacks in order. Its rules are written down because each one, skipped once, cost a review run or a long turn. Push only to fix a blocking finding, a failing check or a confirmed defect, because no push gets a fresh review.

### The review on opening

[`review.yml`](https://github.com/openplush/openplush/blob/main/.github/workflows/review.yml) reviews a PR once: when it opens, or when a draft is marked ready. A push starts no new review. The run coordinates the reviewer agents by what the PR changed:

| Reviewer | Runs when |
|---|---|
| [`pr-reviewer`](https://github.com/openplush/openplush/blob/main/.claude/agents/pr-reviewer.md) | always |
| [`spec-reviewer`](https://github.com/openplush/openplush/blob/main/.claude/agents/spec-reviewer.md) | `specs/` or `CONSTITUTION.md` changed |
| [`docs-reviewer`](https://github.com/openplush/openplush/blob/main/.claude/agents/docs-reviewer.md) | `docs/` or a public surface changed: the pipeline crate's source, `bindings/`, the engine client, or a feature's contracts; or a file that can state what is built changed: a feature's `tasks.md`, `README.md` or `CONTRIBUTING.md`, so a status claim outside [the status page](../start/status.md) is caught |
| [`mutation-auditor`](https://github.com/openplush/openplush/blob/main/.claude/agents/mutation-auditor.md) | only when the maintainer asks for it, and Rust under `crates/` or `bindings/` changed |

The review is advisory. It posts each finding on its line, marked 🔴 blocking, 🟡 optional or 🟣 pre-existing, then one summary comment, and it never approves or requests changes. Three kinds of PR are skipped on opening: a `docs(tasks): tick …` PR, a PR from a `chore/` branch, and a PR that changes only `docs/`. A second run happens only when the maintainer adds the `rereview` or `mutation-audit` label, and it reviews only what changed since the last summary. Runs draw on the maintainer's Claude subscription, which is why there is one per PR ([ADR 0013](../adr/0013-intent-sized-pull-requests.md)). Agents do not run the first three reviewers locally, since the review on opening runs them; the mutation auditor is the one they run themselves, before opening a PR that adds checks to Rust under `crates/` or `bindings/` ([`steward`](https://github.com/openplush/openplush/blob/main/.claude/skills/steward/SKILL.md#local-review)). A person can run any of the reviewers before pushing by asking Claude Code in this repository to use it by name, for example "run the mutation-auditor agent on my branch against main".

### Who merges

The maintainer merges every pull request, bottom-up through a stack, with a merge commit. Agents never merge and never turn on auto-merge. A PR is ready for the maintainer once CI is green on its current head, the review has nothing blocking left, its threads are answered, and its base is `main`.

## Why not the alternative

The alternative is to let something other than a person merge: an agent that judges its own PR, or a workflow that computes a risk tier from the diff and turns on auto-merge. Either one moves the last look at a change away from a person, and the defect this project fears most, a plausible-looking wrong pattern, is the kind that passes CI. An automated gate also has to be trusted not to loosen the checks it is judged by. Keeping every merge with the maintainer costs waiting time on open PRs and buys a person reading everything that lands on `main`.

## Decisions

[ADR 0013](../adr/0013-intent-sized-pull-requests.md).
