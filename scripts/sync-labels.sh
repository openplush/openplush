#!/usr/bin/env bash
# Create or update the repository's labels from .github/labels.json (T019).
# GitHub does not read label files, so a maintainer runs this once, and again
# after editing the file. Needs the GitHub CLI (gh) authenticated with repo access.
#
# Usage: scripts/sync-labels.sh [--dry-run]
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dry=0
[[ ${1:-} == --dry-run ]] && dry=1
command -v jq >/dev/null || { echo "sync-labels: jq is required" >&2; exit 2; }
((dry)) || command -v gh >/dev/null || { echo "sync-labels: gh (GitHub CLI) is required" >&2; exit 2; }

jq -r '.[] | [.name, .color, .description] | @tsv' "$root/.github/labels.json" |
  while IFS=$'\t' read -r name color description; do
    if ((dry)); then
      printf 'gh label create %q --color %q --description %q --force\n' "$name" "$color" "$description"
    else
      gh label create "$name" --color "$color" --description "$description" --force
    fi
  done
