#!/usr/bin/env bash
# Create or update the repository's labels from .github/labels.json.
# GitHub does not read label files, so a maintainer runs this once, and again
# after editing the file. Needs the GitHub CLI (gh) authenticated with repo access.
#
# Usage: scripts/sync-labels.sh [--dry-run] [--repo OWNER/NAME]
# The repository is explicit (default openplush/openplush), so running from a
# fork's clone, or outside any clone, can't silently target the wrong repo.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dry=0
repo=openplush/openplush
while (($#)); do
  case $1 in
    --dry-run) dry=1 ;;
    --repo) repo=${2:?--repo needs OWNER/NAME}; shift ;;
    *) echo "sync-labels: unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done
command -v jq >/dev/null || { echo "sync-labels: jq is required" >&2; exit 2; }
((dry)) || command -v gh >/dev/null || { echo "sync-labels: gh (GitHub CLI) is required" >&2; exit 2; }

jq -r '.[] | [.name, .color, .description] | @tsv' "$root/.github/labels.json" |
  while IFS=$'\t' read -r name color description; do
    if ((dry)); then
      printf 'gh label create %q --repo %q --color %q --description %q --force\n' "$name" "$repo" "$color" "$description"
    else
      gh label create "$name" --repo "$repo" --color "$color" --description "$description" --force
    fi
  done
