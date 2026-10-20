# Quickstart: validating the docs site

**Feature**: `004-docs-site` | **Date**: 2026-10-20

How to prove the feature works once it is built. Commands follow [plan.md](./plan.md).

## Scenarios

1. **The site builds and every check passes** (US2, SC-003, SC-004)
   `scripts/check.sh docs`
   Expect: `target/site/` holds `index.html`, `dev/`, `dev/api/` and `guide/`; the navigation, network, plain-language and link checks pass.

2. **Each check bites** (US2)
   Add `docs/how-to/stray.md` without a SUMMARY entry, a link to `../specs/x.md`, and `<script src="https://example.com/x.js">` in a page. Expect `scripts/check.sh docs` to fail three times, naming each.

3. **Builds are deterministic** (SC-007)
   Build twice into two directories; `diff -r` reports nothing.

4. **A PR gets a preview, not a deploy** (FR-003, FR-004)
   Open a PR touching `docs/`. Expect a downloadable site artifact and no Pages deployment.

5. **`main` publishes the contributor part only** (FR-005)
   After merge, `https://openplush.github.io/openplush/dev/` shows the change; `/guide/` returns 404 until the first release.

6. **A newcomer gets in** (US1, SC-001, SC-002)
   From the landing page, follow "Start here" in a clean container to a passing `scripts/check.sh`, in three commands or fewer. A person new to the repo answers the five US1 questions within 30 minutes.

7. **An author follows the guide** (US4)
   Write a new how-to from `docs/_templates/how-to.md` following `docs/how-to/write-docs.md`; it passes the checks with the template's headings intact. An agent loading the `docs` skill reaches the same result.
