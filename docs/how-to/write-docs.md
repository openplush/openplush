# How to write a documentation page

<!-- index: audience, kind, template, navigation, and the writing rules. -->

This page takes you from "I need to document this" to a page that passes review. Follow the steps in order: each one narrows the next. It is also the one home for the writing rules; the templates carry structure, and the agents' docs skill carries these steps and links back here.

## 1. Pick the audience

Every page is for one of two readers, and the two never share a page.

- **Contributor**: someone changing OpenPlush, in Rust, TypeScript or the docs. Their pages live anywhere under `docs/` except `docs/user/`.
- **Maker**: someone using OpenPlush to sew a plush. They do not program and do not need to know how the engine is built. Their pages live under `docs/user/`.

If a fact matters to both, it gets two pages: the maker page explains what it means for their pattern, and the contributor page holds the exact string, code or type and links to the maker page.

## 2. Pick the kind

A page does one job. If you find yourself writing two, split the page.

| Kind | For | Lives in |
|---|---|---|
| Tutorial | A newcomer learning by doing, start to finish, with a result at the end | `docs/tutorials/` |
| How-to | Someone who knows the basics and has one goal | `docs/how-to/` |
| Explanation | Someone who wants to understand why things are the way they are | `docs/explanation/` |
| Reference | Someone looking up an exact fact: a flag, a message, a type | `docs/reference/` |
| Maker task | A maker reaching one goal in the app | `docs/user/` |
| FAQ entry | A question every new maker asks, with a short answer | `docs/user/faq.md` |

A decision record is not a page kind here: it is an ADR, written from [`docs/adr/0000-template.md`](../adr/0000-template.md). The table in [`docs/adr/README.md`](../adr/README.md) lists it from its H1 and the `status:` in its front matter, generated when the book is built, so a new ADR needs no row there.

## 3. Copy the template

Copy the template for your kind from [`docs/_templates/`](../_templates/) into the folder from step 2 and rename it. Keep its headings and their order, and replace each one-line instruction with your text. If you have nothing for a heading, delete it rather than leave it empty.

A tutorial, how-to, explanation or reference page keeps the template's index line, the first line after its H1:

```markdown
# How to write a documentation page

<!-- index: audience, kind, template, navigation, and the writing rules. -->
```

It is one line on what the reader gets from the page, and it renders nowhere on the page itself: the folder's `README.md` lists the page as its H1, a colon, then that line. A page in one of those folders with no H1 or no index line fails the build, naming the file.

## 4. Place it in the navigation

Add the page to its book's `SUMMARY.md`, next to the pages a reader meets just before and after it. A page in no `SUMMARY.md` fails the build; so does a page in both books. Templates are the one exception and never appear in navigation.

The list in the folder's `README.md` follows `SUMMARY.md`'s order and is generated when the book is built, so you never edit it ([generated index lists](../reference/checks.md#generated-index-lists-docs)).

## 5. Write

Follow the rules below, then read the page once as its audience would.

## 6. Check

Run `scripts/check.sh docs`. It builds both books and fails on a page missing from navigation, a dead link or anchor, or a reference to anything outside the site.

## The rules

Each rule has its reason; if the reason doesn't apply, raise it in review rather than quietly breaking the rule.

**Every page**

- **One home per fact.** State a fact on one page and link to it everywhere else. Two copies drift, and a reader can't tell which one is right.
- **No empty sections.** A heading with nothing under it, or "coming soon", promises a page that doesn't exist. Leave the heading out until there is something to say.
- **Status lives on the status page only.** What is built and what isn't changes every week; one page can be kept current, twenty can't. Link to it.
- **Absolute dates** (YYYY-MM-DD), never "recently" or "next release".
- **Links to files that are not published** (specs, the constitution, agent files, research) use the file's GitHub URL, so they work on the site and on GitHub alike.
- **Same PR.** A change to public or maker-facing behaviour updates its page in the same pull request. A page that lags the code is wrong from the day it merges.

**Contributor pages**

- **Open with the page's purpose**: one or two sentences on what the reader gets from it, before any detail.
- **Process IDs only as links**, or in one closing "Decisions" line. `001 D-013` in running text means nothing to a newcomer; [why seam lengths are pinned](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/plan.md) does, and still leads to the decision. Agents write their status for people under the same rule ([`AGENTS.md`, Comments and prose](https://github.com/openplush/openplush/blob/main/AGENTS.md#comments-and-prose)).
- **Every domain term in the glossary** (`docs/reference/glossary.md`). Link the term the first time a page uses it.
- **Explain, don't instruct agents.** Directives for agents belong in agent files; a contributor page explains the fact and the reasoning behind it.

**Maker pages**

- **Maker words.** Fabric, seam, pattern piece, print; not mesh, chart, half-edge.
- **No code, no process IDs, no crate names.** A maker never needs them, and each one tells them the page wasn't written for them.
- **Only what the app does today** on `main`. The maker guide is published at the first public release; until then it is built and checked, so it is ready.
