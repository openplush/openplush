# Feature Specification: Factory Hardening

**Feature Branch**: `005-factory-hardening`

**Created**: 2026-10-20

**Status**: Draft

**Input**: User description: "Harden the unattended task factory and its merge gate so the rules agents follow live in one checked place instead of in prose copied across several files that disagree. Input is issues #316–#322, one user story per issue. The stale-review-summary and human changes-requested cases from #322 are already handled by the merge gate and are out of scope except for confirming their tests."

> Spec Kit rule: this file states **what** and **why** only. Tool choices live in `plan.md`.

A process review of the factory's first week (issues [#316](https://github.com/openplush/openplush/issues/316)–[#322](https://github.com/openplush/openplush/issues/322)) found one pattern behind most of the maintainer's interventions: a rule that lives only in prose, or in several places that disagree, is followed inconsistently by unattended sessions. Each story below moves one such rule into a single checked home. Builds on [ADR 0015](../../docs/adr/0015-merge-gate.md) and [ADR 0018](../../docs/adr/0018-hands-off-merging.md); this feature changes neither without a new decision.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A kickoff with a blank field never starts (Priority: P1)

The maintainer starts a factory run from a kickoff prompt copied from a template. If a field is still a placeholder, the run stops before doing any work and names the field, instead of guessing and stalling for hours. Issue [#317](https://github.com/openplush/openplush/issues/317).

**Why this priority**: Two runs proceeded on unfilled placeholders; one stalled about 3 h. The fix is small and prevents the most expensive kind of wasted run.

**Independent Test**: Start a firing with an argument containing a placeholder; it stops at once, names the placeholder, and changes nothing. Start one with filled arguments; it proceeds.

**Acceptance Scenarios**:

1. **Given** a kickoff containing an angle-bracketed word or an angle-bracketed list of alternatives, **When** the firing starts, **Then** it stops before any branch, commit or comment, naming the first placeholder and the field it stands for.
2. **Given** a kickoff with every field filled, **When** a task inside the run is blocked, **Then** the run skips and records that task as before; the refusal applies to the kickoff only.

---

### User Story 2 - Every firing hands off, and a session stays short (Priority: P1)

Every factory firing ends with a handoff the maintainer can paste into a fresh session, and a session never runs past one firing or a set context size. Build output does not accumulate until the disk fills. Issue [#321](https://github.com/openplush/openplush/issues/321).

**Why this priority**: Handoffs were requested by hand in at least four sessions; one session ran about 17 h across 19 restarts; a build filled the disk.

**Independent Test**: Run one firing end to end. Its last output is a handoff with state, next steps, blockers and a kickoff of five lines or fewer; the session then stops; the build output from its steps is gone.

**Acceptance Scenarios**:

1. **Given** any firing, including one that stops early, **When** it ends, **Then** it emits a handoff naming open PRs with their head commits, the branch, the next steps in order, the blockers, and a kickoff of five lines or fewer carrying the active grant.
2. **Given** a session whose context passes the threshold the plan sets, **When** it reaches the next step boundary, **Then** it stops with that reason and the handoff.
3. **Given** a step that built for a second target, **When** the step ends, **Then** that target's build output is removed.

---

### User Story 3 - Commits and PRs carry the maintainer's identity only (Priority: P1)

One rule says whose name a commit carries and that no agent attribution appears; every other source defers to it, and CI fails a PR that breaks it. Issue [#319](https://github.com/openplush/openplush/issues/319).

**Why this priority**: Three sources disagreed. Since 2026-10-13, 760 of main's commits name an agent as author, 293 carry an agent co-author line and 280 a session link. History is not rewritten; the fix stops new cases.

**Independent Test**: Open a PR whose commits carry an agent author or trailer, or whose body carries an agent footer; CI fails naming each. Remove them; CI passes.

**Acceptance Scenarios**:

1. **Given** a PR with a commit authored by an agent identity, or carrying an agent co-author or session line, **When** CI runs, **Then** the attribution check fails and names the commit.
2. **Given** a PR whose description ends in an agent-generated footer, **When** CI runs, **Then** the check fails and names it.
3. **Given** commits merged before the check existed, **When** it runs, **Then** it never reports them.

---

### User Story 4 - The merge gate is set up without hand-holding (Priority: P2)

A maintainer turning the gate on runs one diagnostic that lists every missing prerequisite with its fix, on any operating system. The gate says when it is only rehearsing, and agents report a stalled check once instead of polling it. Issue [#322](https://github.com/openplush/openplush/issues/322).

**Why this priority**: Setup took about 30 minutes of hand-holding; stalled CI was noticed by the maintainer, not the agent.

**Independent Test**: Run the diagnostic against a repository with a label missing and auto-merge off; it lists both with their fixes and changes nothing. Put the gate in rehearsal mode; its comment says so and names what switches it on.

**Acceptance Scenarios**:

1. **Given** a repository missing a gate prerequisite (a label, the auto-merge setting, the App, the mode variable), **When** the diagnostic runs, **Then** it names each missing one with its fix and changes no setting.
2. **Given** the gate in rehearsal mode, **When** it comments on a PR, **Then** the comment says it is rehearsing and what switches it to live.
3. **Given** a check that has not started or finished within the time the plan sets, **When** an agent sees it, **Then** the agent reports it once and stops polling.
4. **Given** the stale review summary and human "changes requested" cases, **When** this feature is checked, **Then** their existing tests are named; no new behaviour is added.

---

### User Story 5 - Every decision prompt can be answered cold (Priority: P2)

Every decision an agent raises states the facts, where the decision lives (file and ID), one recommendation, and what happens if nobody answers, with project terms linked to a glossary. Issue [#318](https://github.com/openplush/openplush/issues/318).

**Why this priority**: A decision prompt that gives facts, the place it lives, one recommendation and a default can be answered without re-explanation.

**Independent Test**: Read three decision prompts an agent raised (a question, a ledger comment, a handoff blocker); each has the four parts and every process term in it is in the glossary.

**Acceptance Scenarios**:

1. **Given** an agent that needs a decision, **When** it asks, comments on a ledger issue or writes a handoff, **Then** the prompt has the facts, the file and ID, one recommendation and the default if unanswered.
2. **Given** a process term in a prompt (tier, grant, fixture, factory, stack position), **When** a reader meets it, **Then** the glossary defines it.

---

### User Story 6 - A grant is given once and honoured the same way (Priority: P2)

The maintainer gives a factory run one structured, time-boxed grant (hours, features, whether it may merge, which paths, whether it may accept decisions). Every session reads it the same way, it never contradicts the merge decisions without saying so, and scheduled runs stop when it ends. Issue [#316](https://github.com/openplush/openplush/issues/316).

**Why this priority**: Free-text grants were read inconsistently, an agent merged against ADR 0018, and a scheduled trigger fired after its grant ended (#316). It needs a decision on what a grant may override before any code.

**Independent Test**: Start a run with a grant; a second session renews it with one word; scheduled firings after the grant's end do nothing but report that it expired.

**Acceptance Scenarios**:

1. **Given** a grant argument, **When** a firing starts, **Then** it reads hours, features, merge, paths and decisions from it and states the grant in its handoff.
2. **Given** a grant that allows something a merge decision forbids, **When** the firing reads it, **Then** what wins is the rule [ADR 0020](../../docs/adr/0020-factory-grants.md) sets, with no session deciding alone.
3. **Given** a grant that has expired, **When** a scheduled firing starts, **Then** it does no work and the schedule is turned off.

---

### User Story 7 - Stacked PRs stop conflicting on shared lists (Priority: P3)

Parallel and stacked PRs that each add a page no longer conflict on the same folder index list. The decision ledger stays as it is: it caused 2 of 16 measured conflicts (F-003, accepted 2026-10-21). Issue [#320](https://github.com/openplush/openplush/issues/320).

**Why this priority**: Each conflict costs a merge-up push and a CI run, nearly every time; but the layout is a maintainer decision and the cost is time, not correctness.

**Independent Test**: Two PRs stacked on one parent each add a page to one folder; both merge into the parent without a conflict on the folder's index.

**Acceptance Scenarios**:

1. **Given** two PRs that each add a page to one folder, **When** both merge, **Then** the folder's index list does not conflict and lists both pages.

---

### Edge Cases

- **A placeholder inside quoted example text** in a kickoff (a code span showing the syntax): it is still refused; the kickoff is rewritten without it. A false stop costs one message, a false start costs hours.
- **A firing that stops on its first step** (no ready task, a usage limit): it still emits the handoff.
- **A merge commit or a bot's commit in a PR range**: only commits the PR adds are checked, and a bot that is not an agent identity is not attribution.
- **A diagnostic run without the rights to read a setting**: it reports that setting as unknown with the right it needs, never as present.
- **A grant naming a feature with no ready task**: the firing records "idle" as today.
- **A context threshold reached in the middle of a push**: the step finishes; the stop happens at the next step boundary, so nothing is left half-pushed.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A factory firing MUST scan its arguments for placeholders before any action and stop at the first, naming it (US1).
- **FR-002**: Task-level blockers MUST still be skipped and recorded, never turned into a stop (US1).
- **FR-003**: Every firing MUST end by emitting a handoff with state, next steps, blockers and a kickoff of at most five lines carrying the active grant (US2).
- **FR-004**: A factory session MUST stop after one firing, or at the next step boundary after its context passes a threshold the plan sets, and say which (US2).
- **FR-005**: Build output for secondary targets MUST be removed between steps (US2).
- **FR-006**: One file MUST be the source for commit identity and attribution; hooks and session instructions defer to it (US3).
- **FR-007**: CI MUST fail a PR whose own commits carry an agent author, co-author or session line, or whose description carries an agent footer, naming each (US3).
- **FR-008**: The attribution check MUST NOT report commits outside the PR's range (US3).
- **FR-009**: A diagnostic MUST check every merge-gate prerequisite and print each missing one with its fix, change nothing, and run the same on Linux, macOS and Windows (US4).
- **FR-010**: The gate's PR comment MUST say when the gate is in rehearsal (dry-run) mode and what switches it to live (US4).
- **FR-011**: An agent MUST report a check that has not started or finished within a set time once, then stop polling it (US4).
- **FR-012**: Every decision prompt an agent raises MUST state the facts, the file and ID, one recommendation and the default if unanswered (US5).
- **FR-013**: The glossary MUST define every process term used in decision prompts (US5).
- **FR-014**: A firing MUST accept a structured grant (hours, features, merge, paths, decisions) and a renewal of the last one (US6).
- **FR-015**: What a grant may and may not override MUST be stated in one accepted decision, and the factory rules MUST match it (US6).
- **FR-016**: A scheduled firing MUST do no work after its grant expires, and the schedule MUST be turned off (US6).
- **FR-017**: Adding a page in two parallel PRs MUST NOT conflict on a shared folder index (US7).

### Key Entities

- **Grant**: the maintainer's permission for a factory run: duration, features, whether it may merge, paths it may touch, whether it may accept decisions.
- **Kickoff**: the prompt or arguments that start a firing.
- **Handoff**: the text a firing ends with: state, next steps, blockers, kickoff.
- **Decision prompt**: any question, ledger comment or handoff blocker that asks the maintainer to decide.
- **Gate prerequisite**: a repository label, setting, App installation or variable the merge gate needs.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: No firing starts work on a kickoff with a placeholder (zero in the ledger after this lands).
- **SC-002**: Every firing's last output is a handoff; the maintainer requests none by hand.
- **SC-003**: No session runs more than one firing.
- **SC-004**: Zero commits merged after this lands carry agent attribution.
- **SC-005**: A maintainer enables the merge gate on a fresh repository in under 10 minutes using only the diagnostic's output.
- **SC-006**: The maintainer asks for no re-explanation of a decision prompt.
- **SC-007**: The maintainer states a grant once per run.
- **SC-008**: Ledger and index conflicts in stacked PRs drop to zero, measured over the next 20 stacked PRs.

## Assumptions

- The factory runs as Claude Code sessions driven by `.claude/skills/factory/SKILL.md`; skills have no test harness, so their rules are shown by worked examples.
- The maintainer's identity is the one in the repository's commit history; agent identities are the ones in the history check above.
- The merge gate's stale-summary and changes-requested handling (ADR 0018) is correct and tested; this feature only confirms it.
- US6 and US7 waited on decisions the maintainer accepted on 2026-10-21 (F-002, F-003).

## Out of scope

- Rewriting history to remove past attribution.
- Changing who merges which tier (ADR 0014, ADR 0018) except through the grant decision.
- Generating `docs/SUMMARY.md` itself, which stays hand-curated for its reading order (004's W-004 and W-018).
