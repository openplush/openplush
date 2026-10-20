#!/usr/bin/env bash
# Every link and anchor inside the built docs site resolves. mdBook rewrites
# .md links to .html without checking that the target exists, so the check runs
# on the built HTML, not the Markdown. Links leaving the site are not followed:
# the check stays offline and gives the same answer on every machine.
# The site's directory is also lychee's root, so a root-relative link such as
# mdBook's 404.html <base href="/"> resolves inside the site.
# Usage: scripts/check-docs-links.sh <dir>   (a built site, e.g. target/site/dev)
# Tests: scripts/tests/check-docs-links.test.sh
set -euo pipefail
shopt -s inherit_errexit
export LC_ALL=C

dir=${1:?usage: scripts/check-docs-links.sh <dir>}
[[ -d $dir ]] || { echo "docs-links: $dir is not a directory" >&2; exit 1; }
root=$(cd "$dir" && pwd)

lychee --offline --include-fragments --no-progress --root-dir "$root" "$root" ||
  { echo "docs-links: dead links or anchors under $dir" >&2; exit 1; }
echo "docs-links: ok"
