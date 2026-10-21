#!/usr/bin/env bash
# Tests for scripts/check-attribution.sh: each case builds commits in a throwaway
# repository and asserts whether the check accepts a range and a PR body.
#
# What matters: an agent author or committer, an agent co-author trailer, a
# session line and a footer linking Claude Code each fail, naming the commit or
# the PR body; prose naming the same words passes; a person as co-author passes;
# commits at or before base are never read, however they are authored; a merge
# commit in the range is read like any other; an empty range passes.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-attribution.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Keep the caller's git config (signing, hooks, templates) out of the throwaway repository.
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=Maintainer GIT_AUTHOR_EMAIL=maintainer@example.com
export GIT_COMMITTER_NAME=Maintainer GIT_COMMITTER_EMAIL=maintainer@example.com

agent_email=noreply@anthropic.com
repo="$tmp/repo"
git init -q -b main "$repo"

pass=0
fail=0

# expect <pass|fail|error> <case> <base> <head> [<text the output must contain>]
# The PR body is $body. fail is exit 1 (attribution found), error is exit 2 (bad arguments).
expect() {
  local want=$1 name=$2 pattern=${5:-} out status=0
  out=$(cd "$repo" && PR_BODY=$body "$check" "$3" "$4" 2>&1) || status=$?
  report "$want" "$name" "$pattern" "$status" "$out"
}

# report <want> <case> <pattern> <status> <output>: count and print one case.
report() {
  local want=$1 name=$2 pattern=$3 status=$4 out=$5 code
  case $want in pass) code=0 ;; fail) code=1 ;; error) code=2 ;; esac
  if [[ $status -eq $code && ( -z $pattern || $out == *"$pattern"* ) ]]; then
    pass=$((pass + 1))
    echo "ok   $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name (wanted $want, exit $status)"
    while IFS= read -r line; do echo "     | $line"; done <<<"$out"
  fi
}

# commit <message> [<author name> <author email>]: an empty commit on the current branch.
commit() {
  GIT_AUTHOR_NAME=${2:-$GIT_AUTHOR_NAME} GIT_AUTHOR_EMAIL=${3:-$GIT_AUTHOR_EMAIL} \
    git -C "$repo" commit -q --allow-empty -m "$1"
}

# branch <name>: a new branch at base, checked out.
branch() { git -C "$repo" checkout -q -b "$1" "$base"; }

tip() { git -C "$repo" rev-parse HEAD; }
short() { git -C "$repo" log -1 --format=%h "$1"; }

body=""

# Agent-attributed history before base: the check must never read it.
commit "feat: agent work merged before the check" Claude "$agent_email"
commit "feat: paired work merged before the check

Co-Authored-By: Claude <$agent_email>
Claude-Session: https://claude.ai/code/session_0123"
commit "chore: base"
base=$(tip)

expect pass "an empty range" "$base" "$base"

branch clean
commit "feat: a person's work"
commit "fix: a person's fix

Co-Authored-By: Jane Example <jane@example.com>
Co-Authored-By: Claudette Example <claudette@example.com>"
clean=$(tip)
expect pass "a clean range, with agent commits in base's history" "$base" "$clean"
expect pass "a person as co-author, even one whose name starts with Claude" "$base" "$clean"
expect pass "the range is base..head, never base's history" "$base" "$clean" "attribution: ok"

branch author
commit "feat: agent authored" Claude "$agent_email"
c=$(tip)
expect fail "an agent author" "$base" "$c" "$(short "$c") feat: agent authored: agent author Claude <$agent_email>"

branch author-email
commit "feat: agent email" Someone "$agent_email"
c=$(tip)
expect fail "an agent author email under another name" "$base" "$c" "$(short "$c") feat: agent email: agent author"

branch author-name
commit "feat: agent name" Claude claude@example.com
c=$(tip)
expect fail "the agent author name with another email" "$base" "$c" "$(short "$c") feat: agent name: agent author"

branch committer
GIT_COMMITTER_EMAIL=$agent_email commit "feat: agent committed"
c=$(tip)
expect fail "an agent committer" "$base" "$c" "$(short "$c") feat: agent committed: agent committer"

branch coauthor
commit "feat: paired with the agent

Co-Authored-By: Claude Opus <$agent_email>"
c=$(tip)
expect fail "a Claude co-author trailer" "$base" "$c" "$(short "$c") feat: paired with the agent: agent co-author: Co-Authored-By: Claude Opus"

branch coauthor-email
commit "feat: paired by email

co-authored-by: Assistant <assistant@anthropic.com>"
c=$(tip)
expect fail "a co-author at anthropic.com, any case" "$base" "$c" "$(short "$c") feat: paired by email: agent co-author"

branch session
commit "feat: from a session

Claude-Session: https://claude.ai/code/session_0456"
c=$(tip)
expect fail "a session line" "$base" "$c" "$(short "$c") feat: from a session: session line: Claude-Session:"

branch commit-footer
commit "feat: footer in the message

🤖 Generated with [Claude Code](https://claude.com/claude-code)"
c=$(tip)
expect fail "a footer in a commit message" "$base" "$c" "$(short "$c") feat: footer in the message: agent footer"

branch two
commit "feat: first agent commit" Claude "$agent_email"
first=$(tip)
commit "feat: a person's commit between"
commit "feat: second agent commit

Claude-Session: https://claude.ai/code/session_0789"
second=$(tip)
expect fail "every attributed commit is named: the first" "$base" "$second" "$(short "$first") feat: first agent commit"
expect fail "every attributed commit is named: the second" "$base" "$second" "$(short "$second") feat: second agent commit"

branch side
commit "feat: side work"
branch merged
git -C "$repo" merge -q --no-ff -m "Merge branch 'side'

Co-Authored-By: Claude <$agent_email>" side
c=$(tip)
expect fail "a merge commit in the range is read" "$base" "$c" "$(short "$c") Merge branch 'side': agent co-author"

body="## Intent

- Adds a change.

---
_Generated by [Claude Code](https://claude.ai/code)_"
expect fail "a generated-by footer in the PR body" "$base" "$clean" "PR body: agent footer: _Generated by [Claude Code]"

body="Adds a change.

🤖 Generated with [Claude Code](https://claude.com/claude-code)"
expect fail "a generated-with footer in the PR body" "$base" "$clean" "PR body: agent footer"

body="Adds a change.

https://claude.ai/code/session_0abc"
expect fail "a session link in the PR body" "$base" "$clean" "PR body: session link"

body="The check fails on a Generated with Claude Code footer, a Co-Authored-By: Claude
trailer and a Claude-Session line, and names noreply@anthropic.com in prose."
expect pass "prose naming the footer, the trailer and the session line" "$base" "$clean"

body=""
expect error "a base that is not a commit" no-such-ref "$clean" "not a commit"
expect error "a head that is not a commit" "$base" no-such-ref "not a commit"

status=0
out=$(cd "$repo" && unset PR_BODY && "$check" "$base" "$clean" 2>&1) || status=$?
report pass "no PR body given: the commits only, and it says so" "commits only" "$status" "$out"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
