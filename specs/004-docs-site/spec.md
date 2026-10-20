# Feature Specification: A Documentation Site for Contributors and Makers

**Feature Branch**: `004-docs-site`

**Created**: 2026-10-20

**Status**: Draft

**Input**: User description: "A developer-friendly wiki for the repo: somewhere easy to go. Build it as a published site from `docs/`, not a separate wiki. Today's docs are written for AI agents; a junior developer must be able to get the whole picture without it being cognitively jarring. Add a separate section for what end users will see once the project is released: a user guide and an FAQ. Back the whole thing with a docs skill, templates and a plan, so humans and agents write pages the same way. Get to a good place to refine later, not a huge undertaking."

> Spec Kit rule: this file states **what** and **why** only. Tool choices live in `research.md` and `plan.md`.

Builds on [ADR 0003](../../docs/adr/0003-as-built-docs.md): once code lands, the code and `docs/` are the source of truth. This feature does not change that rule. It gives `docs/` readers, a shape, and a way to publish it.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A junior developer finds their way in (Priority: P1)

A developer in their first job opens the site from the repository's README. A "Start here" page tells them what OpenPlush is, what is built today, and the path to follow. They set up, run every check, understand the pipeline at the level of "which crate does what, and why", and pick a first task, without opening an agent file, a spec or an ADR unless they want to go deeper.

**Why this priority**: This is the request. What a newcomer needs exists today, but it is scattered across the README, CONTRIBUTING, agent files, specs and ADRs, written as terse rules and decision IDs. Newcomers' biggest barriers are finding where to start and the knowledge they don't yet have.

**Independent Test**: Someone new to the repository follows only the site from the landing page, reaches a passing local check run, and answers five orientation questions (what does each pipeline stage do, which crate owns seams, who merges a PR, what does a task's `needs:` token mean, where is the project status).

**Acceptance Scenarios**:

1. **Given** the published site, **When** a reader opens it, **Then** the first page names the audience of each part (contributors, makers) and the path through the contributor part, in order.
2. **Given** a reader following the contributor path, **When** they reach the setup page, **Then** it gets them to a passing run of every gate in three documented commands or fewer (Constitution IX).
3. **Given** a page that uses a domain term (half-edge, chart, dart, seam allowance, nap) or a process ID (D-013, T074, FR-002), **When** a reader meets it, **Then** the term or the ID's prefix is defined in one glossary that the page links to.
4. **Given** a question "is X built yet?", **When** a reader looks it up, **Then** exactly one page answers it, and every other place that mentions status links to that page.

---

### User Story 2 - Every reader can trust what the site shows (Priority: P1)

Whatever page a reader lands on is current, reachable from the navigation, free of dead links, and loads nothing from the network. The site is rebuilt and republished from the main branch without anyone remembering to.

**Why this priority**: A wrong page that looks authoritative is the docs version of plausible but wrong output (Constitution III). A site that leaks requests breaks the project's privacy promise (Constitution VIII) on its most public surface.

**Independent Test**: Build the site from a clean checkout. The checks fail on an unlisted page, a dead internal link, or any reference to a remote font, script or stylesheet, and pass otherwise. Building the same commit twice gives identical output.

**Acceptance Scenarios**:

1. **Given** a change that adds a page without listing it in the navigation, **When** the checks run, **Then** they fail and name the page.
2. **Given** a link from the site to a file outside the published docs (a spec, the constitution, an agent file), **When** the site is built, **Then** the link opens that file on the repository host, never a dead page.
3. **Given** any published page, **When** it loads, **Then** the browser makes no request outside the site.
4. **Given** a change merged to the main branch, **When** the publish run finishes, **Then** the site shows it, with no manual step.
5. **Given** a proposed change to the docs, **When** its checks run, **Then** a reviewer can download and open the built site for that change.

---

### User Story 3 - A maker gets help using OpenPlush (Priority: P2)

Once OpenPlush is released, a maker (someone who sews plush, not a programmer) finds a user guide and an FAQ: what OpenPlush does, how to go from a 3D model to a printed pattern, what each message in the app means and what to do about it, and the questions every new maker asks (Is it free? Does my model leave my computer? Which files can I use?). It is written in the maker's vocabulary, with no code and no process IDs.

**Why this priority**: Makers are who the product is for. Writing their guide alongside the features, instead of after release, means it is never a catch-up project, and it forces each maker-facing feature to be explainable in plain words. It is P2 because nothing maker-facing ships yet.

**Independent Test**: The user guide builds and passes the same checks as the contributor part. Every page describes behaviour that exists in the code. A plain-language check finds no process ID and no code identifier in it.

**Acceptance Scenarios**:

1. **Given** the maker part, **When** a reader opens it, **Then** it is separate from the contributor part, with its own navigation and landing page, and never links into contributor pages for something a maker needs.
2. **Given** a change that adds or changes behaviour a maker sees, **When** it is merged, **Then** the user-guide page for that behaviour changes in the same change, as code and as-built docs already do.
3. **Given** the project has not had a first public release, **When** the site is published, **Then** the maker part is built and checked but not published, so the site never promises makers something that does not exist.
4. **Given** a message the app shows a maker (an error or a warning), **When** they look it up, **Then** the guide explains it in plain words and says what to do.

---

### User Story 4 - Any author, human or agent, writes a page the same way (Priority: P2)

Someone about to write documentation, a person or an agent, starts from one guide that says: which audience and which kind of page this is (tutorial, how-to, explanation, reference, maker guide, FAQ entry), which template to copy, where the page goes in the navigation, and the writing rules. Agents follow the same guide through a docs skill that adds only the procedure.

**Why this priority**: Without a shared shape, a site grows into whatever each author's habit is, which today means agent-style rule lists. With one, a reader meets the same page structure everywhere, which lowers the effort of each new page. It is P2 because the first pages in US1 need it, so it lands early in practice.

**Independent Test**: A new page written from a template passes the checks with no edits to the template's structure. The skill names the guide and adds no writing rule of its own.

**Acceptance Scenarios**:

1. **Given** an author starting a page, **When** they open the writing guide, **Then** it tells them the audience, the kind of page, the template and the navigation position in that order.
2. **Given** the docs skill, **When** an agent loads it, **Then** every rule it applies is a link into the writing guide; the skill holds steps, not rules.
3. **Given** a rule that agent files and human docs both need (a command, a branch rule, a merge tier), **When** it is stated, **Then** it has one home in `docs/` and the agent file links to it, except where the plan records a reason otherwise.

---

### User Story 5 - A developer looks up the engine's API beside the guide (Priority: P3)

From a guide page that names a type or function, a developer opens the generated API documentation for it on the same site.

**Why this priority**: The API docs already exist locally; publishing them beside the guide removes a build step for a reader who just wants to look something up. It is P3 because it hinges on a licence decision (plan, W-007).

**Independent Test**: The published site contains the API documentation for every workspace crate, reachable from the contributor navigation, and it passes the link and network checks.

**Acceptance Scenarios**:

1. **Given** the contributor navigation, **When** a reader opens the API entry, **Then** they see a list of the workspace's crates and can open each.

---

### Edge Cases

- **A page in neither navigation**: the checks fail, naming it. Template files are excluded by rule, not by accident.
- **A link to a section's index page**: it resolves to that section's landing page, not a dead file.
- **A link to something outside `docs/`**: it opens on the repository host. A relative link that leaves the published tree fails the checks.
- **An empty section**: it does not appear in the navigation until it has a real page. No "coming soon" pages.
- **Status drift**: a page other than the status page that states what is built fails review and gets a link instead.
- **A maker-facing change with no guide page yet**: the change adds the page. Before the first release that page is built and checked but unpublished.
- **The API build or the maker part fails to build**: the whole publish fails; a partial site is never published.
- **A forked contributor's change**: it can be built and previewed, never published.

## Requirements *(mandatory)*

### Functional Requirements

**Site and publishing**

- **FR-001**: The project MUST publish one documentation site built from the repository's `docs/` folder, with a landing page that routes each audience to its part.
- **FR-002**: The site MUST have two parts with separate navigation: a contributor guide and a maker guide (user guide and FAQ).
- **FR-003**: The site MUST be rebuilt and republished automatically from the main branch. Publishing MUST NOT run for proposed changes or forks.
- **FR-004**: Every proposed change MUST build the whole site and make the built site available to reviewers for download.
- **FR-005**: The maker part MUST be built and checked on every change but MUST NOT be published before the project's first public release. Publishing it is a maintainer decision.

**Trust**

- **FR-010**: Every page under `docs/` MUST appear in exactly one part's navigation, or be excluded by a stated rule (templates). The checks MUST fail otherwise.
- **FR-011**: The checks MUST fail on any dead link inside the built site, including anchors.
- **FR-012**: Links to repository files that are not published (specs, constitution, agent files, research) MUST point to the file on the repository host.
- **FR-013**: No published page MAY load a font, script, stylesheet or other resource from outside the site, and no page MAY send any request on load. The checks MUST fail on a remote reference in the built output.
- **FR-014**: Building the same commit twice MUST give identical output.
- **FR-015**: Project status (what is built, what is not) MUST be stated on exactly one page. Other documents link to it.

**Contributor guide**

- **FR-020**: The contributor guide MUST be ordered as a path: start here, set up, first change, how the engine works, recipes, how the project is run, reference, decisions.
- **FR-021**: It MUST include a glossary defining domain terms (geometry, sewing) and every process-ID prefix used in the repository.
- **FR-022**: It MUST include a walkthrough that follows one model through every pipeline stage, naming the crate that owns each.
- **FR-023**: Every fact a human contributor needs MUST have its home in `docs/`. Agent files keep agent directives and link to `docs/` for the facts, except where the plan records a reason.
- **FR-024**: It SHOULD include the generated API documentation of the workspace crates (US5), subject to the licence decision in the plan.

**Maker guide**

- **FR-030**: The maker guide MUST be written for a maker, not a programmer: no code, no process IDs, no crate names.
- **FR-031**: It MUST contain an FAQ, a getting-started path to a first printed pattern, task guides, and an explanation of every message the app shows a maker.
- **FR-032**: It MUST describe only behaviour that exists in the code; FR-005 keeps it unpublished until that behaviour is released. A change that alters maker-facing behaviour MUST update the maker guide in the same change.

**Authoring**

- **FR-040**: There MUST be one writing guide that defines the audiences, the page kinds, the writing rules, and how IDs may appear in each part.
- **FR-041**: There MUST be one template per page kind.
- **FR-042**: There MUST be a docs skill that walks an agent through the writing guide's steps and states no rule of its own.

### Key Entities

- **Part**: an audience's section of the site (contributor guide, maker guide), with its own navigation and landing page.
- **Page kind**: tutorial, how-to, explanation, reference (Diátaxis), plus maker task guide and FAQ entry.
- **Template**: the starting structure for one page kind.
- **Writing guide**: the one home for documentation rules.
- **Status page**: the one home for what is built.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Following only the site, a new contributor reaches a passing local run of every gate in three documented commands or fewer.
- **SC-002**: A person new to the repository answers the five orientation questions in US1 within 30 minutes using only the site. Measured once, by a person, before the onboarding part is called done.
- **SC-003**: The built site has zero dead internal links and zero remote resource references, enforced on every change.
- **SC-004**: Every page under `docs/` is in exactly one navigation, enforced on every change.
- **SC-005**: Project status appears on exactly one page.
- **SC-006**: Zero process IDs and zero code identifiers in the maker guide, enforced on every change.
- **SC-007**: Two builds of the same commit are byte-identical.

## Assumptions

- The site lives on the repository host's free static hosting at the project's default address. A custom domain is out of scope.
- The site is English only.
- Diagrams are out of scope until a page needs one; adding them is its own task with its own cost check.
- The maker guide may later be bundled inside the app for offline help; that is out of scope here, but nothing in this feature should prevent it.
- `specs/`, the constitution, `research/` and agent files are linked from the site, never rendered in it. Specs record intent; the site describes what exists.

## Out of scope

- A wiki outside the repository (separate pages not reviewed through pull requests).
- Search engines, analytics, comments, or any hosted service on the site.
- Versioned docs per release; there is no release yet.
- Translating the maker guide.
