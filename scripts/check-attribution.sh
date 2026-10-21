#!/usr/bin/env bash
# Attribution check (AGENTS.md, Commits and PRs): a pull request's own commits
# carry the contributor's identity and no agent attribution, and its description
# carries no agent footer. Fails on, in base..head (merge commits included):
#   - an author or committer email noreply@anthropic.com, or the author name Claude;
#   - a Co-Authored-By: line naming Claude (as a word) or an anthropic.com address;
#   - a Claude-Session: line;
#   - a "Generated with/by [Claude Code](...)" footer or a claude.ai/code/session_
#     link, in a commit message or the PR body.
# Footers are matched by their link and trailers by their line start, so prose
# that names them passes. Commits at or before base are never read: history merged
# before this check stays as it is.
#
# Usage: PR_BODY=<description> scripts/check-attribution.sh <base> <head>
#   The PR body comes from the PR_BODY environment variable; unset, only the
#   commits are checked (a local run has no PR), and the output says so.
#   Exit 0 clean, 1 attribution found, 2 bad arguments.
# Tests: scripts/tests/check-attribution.test.sh
set -euo pipefail
export LC_ALL=C
shopt -s nocasematch

die() {
  echo "attribution: $*" >&2
  exit 2
}

(($# == 2)) || die "usage: PR_BODY=<description> scripts/check-attribution.sh <base> <head>"
base=$(git rev-parse --verify --quiet "$1^{commit}") || die "base '$1' is not a commit"
head=$(git rev-parse --verify --quiet "$2^{commit}") || die "head '$2' is not a commit"

agent_email=noreply@anthropic.com
agent_name=claude
# Claude as a word, so a person named Claudette is not an agent.
claude_word='(^|[^[:alnum:]])claude([^[:alnum:]]|$)'
coauthor_re="^[[:space:]]*co-authored-by:(.*${claude_word}|.*anthropic\\.com)"
session_line_re='^[[:space:]]*claude-session:'
footer_re='generated (with|by) \[claude code\]\(https://claude\.(ai/code|com/claude-code)'
session_link_re='claude\.ai/code/session_'

findings=()

# line_hit <line>: what agent attribution the line carries, or nothing.
line_hit() {
  if [[ $1 =~ $coauthor_re ]]; then
    echo "agent co-author"
  elif [[ $1 =~ $session_line_re ]]; then
    echo "session line"
  elif [[ $1 =~ $footer_re ]]; then
    echo "agent footer"
  elif [[ $1 =~ $session_link_re ]]; then
    echo "session link"
  fi
}

# check_text <where>: record each attributed line of stdin under <where>.
check_text() {
  local line what
  while IFS= read -r line || [[ -n $line ]]; do
    what=$(line_hit "$line")
    line=${line#"${line%%[![:space:]]*}"}
    [[ -z $what ]] || findings+=("$1: $what: ${line%"${line##*[![:space:]]}"}")
  done
}

# check_commit <sha>: its author, committer and message.
check_commit() {
  local an ae ce where
  { IFS= read -r an && IFS= read -r ae && IFS= read -r ce && IFS= read -r where; } \
    < <(git log -1 --format='%an%n%ae%n%ce%n%h %s' "$1")
  if [[ ${an,,} == "$agent_name" || ${ae,,} == "$agent_email" ]]; then
    findings+=("$where: agent author $an <$ae>")
  fi
  [[ ${ce,,} != "$agent_email" ]] || findings+=("$where: agent committer $ce")
  check_text "$where" < <(git log -1 --format=%B "$1")
}

count=0
while IFS= read -r sha; do
  check_commit "$sha"
  count=$((count + 1))
done < <(git rev-list --topo-order --reverse "$base..$head")

if [[ -v PR_BODY ]]; then
  check_text "PR body" <<<"$PR_BODY"
  scope="$count commit(s) and the PR body"
else
  scope="$count commit(s); commits only, PR_BODY is unset"
fi

if ((${#findings[@]})); then
  printf 'attribution: %s\n' "${findings[@]}" >&2
  echo "attribution: commits carry your own identity and no agent attribution (AGENTS.md, Commits and PRs); checked $scope" >&2
  exit 1
fi
echo "attribution: ok ($scope)"
