# How agents work in this repo

<!-- index: how AI agents take a feature from spec to merged pull request: Spec Kit, the factory, steward, the review on opening and the merge gate. -->

AI coding agents write most of OpenPlush's pull requests. This page explains, for a contributor who wants to know how that is organised, what the agents do from a feature idea to a merged change, and what keeps a person in charge. The rules themselves live in the skills and ADRs this page links to; it explains them and never replaces them.

## The short answer

Work starts as a Spec Kit feature: a spec, a plan and a numbered task list. An agent picks a ready task, writes the failing test and then the code, and opens one pull request per intent. An automated review runs once, when the PR opens. A workflow called the merge gate then works out from the diff who may merge: low-risk PRs merge on their own, expected-output changes wait a day first, and engine code, specs, decisions and the rules themselves wait for the maintainer. The one exception is a standing authorization: the maintainer can let an agent merge for a few hours, inside written bounds.

## How it works

### Every rule has one home

Agents read [`AGENTS.md`](https://github.com/openplush/openplush/blob/main/AGENTS.md) first; `CLAUDE.md` only imports it. Rules for a single job live in a skill under [`.claude/skills/`](https://github.com/openplush/openplush/tree/main/.claude/skills), which an agent loads when it starts that job, and reviewers are described as agents under [`.claude/agents/`](https://github.com/openplush/openplush/tree/main/.claude/agents). Above all of them sits the [constitution](https://github.com/openplush/openplush/blob/main/.specify/memory/constitution.md), which wins any conflict. The reasoning behind a process rule is in an ADR, so a skill can stay short and point there.

### From an idea to tasks: Spec Kit

OpenPlush is built spec first, with [Spec Kit](https://github.com/github/spec-kit). Each feature gets a numbered folder under [`specs/`](https://github.com/openplush/openplush/tree/main/specs), and work moves through it in order: specify, clarify, plan, tasks, implement. Each step is a skill an agent or a person runs as a slash command:

- [`speckit-specify`](https://github.com/openplush/openplush/blob/main/.claude/skills/speckit-specify/SKILL.md) writes `spec.md`: what the feature does and why, with no technology in it.
- [`speckit-clarify`](https://github.com/openplush/openplush/blob/main/.claude/skills/speckit-clarify/SKILL.md) asks the few questions the spec leaves open and writes the answers back.
- [`speckit-plan`](https://github.com/openplush/openplush/blob/main/.claude/skills/speckit-plan/SKILL.md) writes `plan.md` and its companions: research, data model, contracts and a quickstart. The plan holds the Decision Ledger, where each design choice waits for the maintainer to accept it.
- [`speckit-tasks`](https://github.com/openplush/openplush/blob/main/.claude/skills/speckit-tasks/SKILL.md) turns the plan into `tasks.md`: numbered tasks, each naming its files and, where there is code, its test first.
- [`speckit-implement`](https://github.com/openplush/openplush/blob/main/.claude/skills/speckit-implement/SKILL.md) works through those tasks.

What each file in a feature folder holds is in the [specs index](https://github.com/openplush/openplush/blob/main/specs/README.md). The task list matters most to agents: every change that is not trivial maps to a task ID, and an agent with no task stops rather than invent scope. Tasks also carry tokens that say when they are ready: `needs:` names tasks that must land first, `blocked:` names a decision only the maintainer can clear, and `human:` marks work no agent picks up. The Spec Kit files themselves are generated, so they are changed with the `specify` tool, never by hand.

### Landing a task as a pull request

A task becomes a branch, a test commit that fails, and the commit that makes it pass. Tasks are grouped into pull requests by intent: one PR brings in one thing toward a feature and says why, so that someone who follows `git blame` back to it months later finds what brought the change in. There is no line budget. Work that depends on other unfinished work is stacked, each PR based on the one below it. [How to scope and open a pull request](../how-to/pull-requests.md) walks through this with examples, and [ADR 0013](../adr/0013-intent-sized-pull-requests.md) records why intents replaced the old 400-line limit.

### The factory: unattended firings

The [`factory`](https://github.com/openplush/openplush/blob/main/.claude/skills/factory/SKILL.md) skill is how that work happens without anyone at the keyboard. A scheduled task fires `/factory <feature>` into a fresh session, hourly, for the hours the maintainer turns it on. One firing:

1. looks at `main` and the open PRs, and first drives any open PR that has a blocking finding, red CI or a conflict;
2. picks up to four ready tasks that touch different crates and share no file;
3. lands each one as a stacked PR, test commit first, with its local gates green;
4. writes one comment on the feature's factory ledger issue (what it opened, skipped and merged, and why it stopped), then ends with a handoff: the state of its PRs, the next steps, the decisions it needs and a kickoff of five lines or fewer that starts the next firing in a fresh session.

A firing never asks a question. Anything that would need one is a skip or a stop, and every stop has a named reason: an unfilled placeholder such as `<window>` left in the kickoff (checked before anything else, so such a firing writes nothing), no ready task, a gate failing twice, a golden snapshot that would change, a task needing a decision or a spec change, a usage limit, or the session's context growing past a set size. A session runs one firing, so long sessions do not drift, and its agents share one build directory whose WebAssembly output is removed after each step, so builds do not fill the disk. The firing does not wait for CI or review; the next firing picks the PR up. The maintainer reads the ledger, accepts decisions and ticks the `human:` work.

### Steward: driving a pull request

Once a PR is open, the [`steward`](https://github.com/openplush/openplush/blob/main/.claude/skills/steward/SKILL.md) skill says how an agent sees it through: subscribe to the PR so CI and review results arrive as events, check in now and then for what sends no event (a conflict with `main`, another agent's push), fix what is blocking, answer what is not, and keep stacks in order. Its rules are written down because each one, skipped once, cost a review run or a long turn. Push only to fix a blocking finding, a failing check or a confirmed defect, because every push restarts the merge gate's judgement and no push gets a fresh review.

### The review on opening

[`review.yml`](https://github.com/openplush/openplush/blob/main/.github/workflows/review.yml) reviews a PR once: when it opens, or when a draft is marked ready. A push starts no new review. The run coordinates the reviewer agents by what the PR changed:

| Reviewer | Runs when |
|---|---|
| [`pr-reviewer`](https://github.com/openplush/openplush/blob/main/.claude/agents/pr-reviewer.md) | always |
| [`spec-reviewer`](https://github.com/openplush/openplush/blob/main/.claude/agents/spec-reviewer.md) | `specs/` or `.specify/memory/` changed |
| [`docs-reviewer`](https://github.com/openplush/openplush/blob/main/.claude/agents/docs-reviewer.md) | `docs/` or a public surface changed: the pipeline crate's source, `bindings/`, the engine client, or a feature's contracts; or a file that can state what is built changed: a feature's `tasks.md`, `README.md` or `CONTRIBUTING.md`, so a status claim outside [the status page](../start/status.md) is caught |
| [`mutation-auditor`](https://github.com/openplush/openplush/blob/main/.claude/agents/mutation-auditor.md) | only when the maintainer asks for it, and Rust under `crates/` or `bindings/` changed |

The review is advisory. It posts each finding on its line, marked 🔴 blocking, 🟡 optional or 🟣 pre-existing, then one summary comment, and it never approves or requests changes. Three kinds of PR are skipped on opening: a `docs(tasks): tick …` PR, a PR from a `chore/` branch, and a PR that changes only `docs/`. A second run happens only when the maintainer adds the `rereview` or `mutation-audit` label, and it reviews only what changed since the last summary. Runs draw on the maintainer's Claude subscription, which is why there is one per PR ([ADR 0013](../adr/0013-intent-sized-pull-requests.md)). Agents do not run the first three reviewers locally, since the review on opening runs them; the mutation auditor is the one they run themselves, before opening a PR that adds checks to Rust under `crates/` or `bindings/` ([`steward`](https://github.com/openplush/openplush/blob/main/.claude/skills/steward/SKILL.md#local-review)). A person can run any of the reviewers before pushing by asking Claude Code in this repository to use it by name, for example "run the mutation-auditor agent on my branch against main".

### The merge gate: tiers and who merges

Whether a PR may merge without a person is decided by [`merge-gate.yml`](https://github.com/openplush/openplush/blob/main/.github/workflows/merge-gate.yml), with no language model in the decision. It computes the PR's tier from its diff, title and labels, judges the conditions on the current head commit, and keeps one comment on the PR listing each condition as passing or failing, so a stalled PR explains itself.

- **Tier A** is most of `docs/`, the apps and packages, research and tick PRs. Once CI is green, the review has nothing blocking left, and no review thread is open, the gate labels the PR `merge:ready`. It turns on auto-merge only when the repository variable `MERGE_GATE_MODE` is `on`; the gate ships in `dry-run`, which labels and comments but never touches auto-merge ([ADR 0015](../adr/0015-merge-gate.md)).
- **Tier B** is a change to expected output, such as a golden fixture or a snapshot. The same conditions apply, plus a 24-hour hold after CI goes green, so the maintainer has a day to look at the output.
- **Tier C** is engine code, specs, decisions and dependency manifests. The same conditions as tier A apply and the gate merges it once they hold, with no hold ([ADR 0018](../adr/0018-hands-off-merging.md)).
- **Tier H** is what defines the gate and the rules: CI workflows, `scripts/`, the agent files, ADRs, the constitution, the licence files, and a tick of a `human:` task. The gate never marks it ready; the maintainer merges it, or a run the maintainer granted that right (below), so no PR can change the rules it is judged by without a person choosing to let it.

A PR's tier is the highest that any of its files reaches, and the gate can raise a tier label but never lower one. A push removes `merge:ready` and turns auto-merge off at once, so a new commit is never merged on the old commit's verdict. The review summary must name the current head: after a push the gate adds the `rereview` label itself, once per head, and waits for the new summary. A reviewer's "changes requested" blocks every tier until it is lifted. The gate also never runs the PR's own code. [ADR 0014](../adr/0014-auto-merge-tiers.md) sets tiers A and B and the tick exception; [ADR 0018](../adr/0018-hands-off-merging.md) sets tiers C and H; [ADR 0015](../adr/0015-merge-gate.md) explains the gate and its conditions; the tier table, which agents and people both work from, is in [Get a pull request reviewed and merged](../how-to/review-and-merge.md#tiers).

### Standing authorizations

For one day, tier C was the maintainer's and a standing authorization let an agent merge it for a while ([ADR 0016](../adr/0016-standing-merge-authorization.md)). Since the gate merges tier C itself ([ADR 0018](../adr/0018-hands-off-merging.md)), a grant matters only where the gate stops: tier H, and decisions. Grants given in free text were restated about ten times in one day and read differently by each session, so a grant is now one structured, time-boxed argument to a factory run, and under one that allows merging, a run merges a tier H PR only on the gate's verdict for every condition but the tier: the grant stands in for the one condition the gate leaves to a person. [ADR 0020](../adr/0020-factory-grants.md) sets the fields, the limits and what never merges under a grant.

## Why not the alternative

The obvious alternatives are the two ends: the maintainer merges every PR, or an agent merges whatever passes. The first was the starting point, and the queue grew faster than one person could clear it as hourly firings opened PRs ([ADR 0014](../adr/0014-auto-merge-tiers.md)). The second would let an agent loosen the very checks it is judged by, and would put engine code, where a plausible-looking wrong pattern is the defect this project fears most, on `main` with no person choosing to let it in. The first design, where the agent that opened a PR labelled its tier and turned on auto-merge, was replaced too: the tier was only as right as the agent's reading, and only the session that opened a PR heard its events ([ADR 0015](../adr/0015-merge-gate.md)). So the decision moved into a deterministic workflow, and the one thing it never merges is the set of files that define it: the gate, the checks, the rules and the constitution stay with the maintainer, or with a run the maintainer granted them to for a few hours, on the gate's verdict for everything but the tier.

## Decisions

[ADR 0013](../adr/0013-intent-sized-pull-requests.md), [ADR 0014](../adr/0014-auto-merge-tiers.md), [ADR 0015](../adr/0015-merge-gate.md), [ADR 0016](../adr/0016-standing-merge-authorization.md), [ADR 0018](../adr/0018-hands-off-merging.md), [ADR 0020](../adr/0020-factory-grants.md).
