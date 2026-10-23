#!/usr/bin/env bash
# Tests for scripts/check-adr-lifecycle.sh: each case builds a throwaway tree of ADRs,
# an exceptions file and plans, and asserts whether the check accepts or rejects it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-adr-lifecycle.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0
out=""

# expect <pass|fail> <case name> <root> [<text the output must contain>...]
expect() {
  local want=$1 name=$2 root=$3 status=0 p missing=""
  shift 3
  out=$("$check" "$root" 2>&1) || status=$?
  for p in "$@"; do [[ $out == *"$p"* ]] || missing+=" [$p]"; done
  if [[ -z $missing ]] && { [[ $want == pass && $status -eq 0 ]] || [[ $want == fail && $status -ne 0 ]]; }; then
    pass=$((pass + 1))
    echo "ok   $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name (wanted $want, exit $status${missing:+, output lacks$missing})"
    while IFS= read -r line; do echo "     | $line"; done <<<"$out"
  fi
}

# new_case <name>: a tree with an ADR folder, its template and index, and an
# exceptions file holding only a comment.
new_case() {
  local root="$tmp/$1"
  mkdir -p "$root/docs/adr" "$root/scripts"
  printf -- '---\n# Proposed | Accepted | Superseded by ADR-NNNN\nstatus: Proposed\n---\n\n# Title\n\nSupersedes ADR 0001.\n' \
    >"$root/docs/adr/0000-template.md"
  printf '# ADRs\n\n[0001](0001-old.md) is amended by [0002](0002-new.md); 0002 supersedes ADR 0009.\n' \
    >"$root/docs/adr/README.md"
  printf '# adr other kind reason\n\n' >"$root/scripts/adr-lifecycle-exceptions.txt"
  echo "$root"
}

# adr <root> <NNNN-slug> <status> <body>: write one ADR in the template's format.
adr() { printf -- '---\nstatus: %s\ndate: 2026-10-20\ndeciders: maintainer\n---\n\n# %s\n\n%s\n' "$3" "$2" "$4" >"$1/docs/adr/$2.md"; }

# except <root> <entry>: append an exceptions entry.
except() { printf '%s\n' "$2" >>"$1/scripts/adr-lifecycle-exceptions.txt"; }

# plan <root> <feature> <ledger rows...>: a plan whose Decision Ledger holds the rows.
plan() {
  local f="$1/specs/$2/plan.md"
  shift 2
  mkdir -p "$(dirname "$f")"
  printf '# Plan\n\n| Field | Value |\n|---|---|\n| Status | Proposed |\n\n## Decision Ledger\n\n| Date | ID | Decision | Status |\n|---|---|---|---|\n' >"$f"
  printf '%s\n' "$@" >>"$f"
  printf '\n## Next\n\n| A | B | C | Status |\n|---|---|---|---|\n| x | Z-999 | not a ledger row | Proposed |\n' >>"$f"
}

# Decision IDs, split so the hygiene check does not read these fixtures as spec references.
d101="D-""101" d102="D-""102" d201="D-""201" d202="D-""202"

new_amend='Supersedes, in ADR 0001: the clause on budgets.'
old_record='> Amended 2026-10-20: the clause on budgets is superseded by [ADR 0002](0002-new.md).'

r=$(new_case clean)
adr "$r" 0001-old Accepted "$old_record"
adr "$r" 0002-new Accepted "$new_amend"
expect pass "an amendment recorded on both sides" "$r" "adr-lifecycle: ok"

r=$(new_case no-links)
adr "$r" 0001-old Proposed "Nothing here names another record."
expect pass "ADRs with no amendments" "$r"

r=$(new_case amended-but-proposed)
adr "$r" 0001-old Proposed "$old_record"
adr "$r" 0002-new Accepted "$new_amend"
expect fail "an Accepted ADR amends a Proposed one" "$r" "ADR 0002" "ADR 0001" "Proposed" "0001 0002 status"

r=$(new_case excepted)
adr "$r" 0001-old Proposed "$old_record"
adr "$r" 0002-new Accepted "$new_amend"
except "$r" "0001 0002 status only the maintainer settles 0001's status"
expect pass "an exception with its reason" "$r" "1 excepted"

r=$(new_case exception-no-reason)
adr "$r" 0001-old Proposed "$old_record"
adr "$r" 0002-new Accepted "$new_amend"
except "$r" "0001 0002 status"
expect fail "an exception without a reason" "$r" "reason"

r=$(new_case exception-bad-kind)
adr "$r" 0001-old Proposed "$old_record"
adr "$r" 0002-new Accepted "$new_amend"
except "$r" "0001 0002 lifecycle the maintainer settles it"
expect fail "an exception of an unknown kind" "$r" "status or backlink"

r=$(new_case stale)
adr "$r" 0001-old Accepted "$old_record"
adr "$r" 0002-new Accepted "$new_amend"
except "$r" "0001 0002 status only the maintainer settles 0001's status"
expect fail "a stale exception" "$r" "stale" "0001 0002 status"

r=$(new_case no-exceptions-file)
rm "$r/scripts/adr-lifecycle-exceptions.txt"
expect fail "a missing exceptions file" "$r" "adr-lifecycle-exceptions.txt"

r=$(new_case proposed-amender)
adr "$r" 0001-old Proposed "$old_record"
adr "$r" 0002-new Proposed "$new_amend"
expect pass "a Proposed ADR amending a Proposed one is not checked yet" "$r"

r=$(new_case missing-backlink)
adr "$r" 0001-old Accepted "Nothing here names the newer record."
adr "$r" 0002-new Accepted "$new_amend"
expect fail "a missing backlink" "$r" "ADR 0001" "ADR 0002" "0001 0002 backlink"

r=$(new_case mention-only)
adr "$r" 0001-old Accepted "See [ADR 0002](0002-new.md) for the tiers."
adr "$r" 0002-new Accepted "$new_amend"
expect fail "a mention that does not record the amendment" "$r" "0001 0002 backlink"

r=$(new_case backlink-excepted)
adr "$r" 0001-old Accepted "Nothing here names the newer record."
adr "$r" 0002-new Accepted "$new_amend"
except "$r" "0001 0002 backlink the record predates the amendment note"
expect pass "a missing backlink with an exception" "$r"

r=$(new_case missing-forward-link)
adr "$r" 0001-old Accepted "$old_record"
adr "$r" 0002-new Accepted "Nothing here names the older record."
expect fail "an ADR claims an amendment its amender does not name" "$r" "ADR 0002" "ADR 0001" "0002 0001 backlink"

r=$(new_case superseded-status)
adr "$r" 0001-old "Superseded by ADR-0002" "The original decision."
adr "$r" 0002-new Accepted "Supersedes [ADR 0001](0001-old.md) whole."
expect pass "a Superseded by status is the backlink" "$r"

r=$(new_case superseded-by-other)
adr "$r" 0001-old "Superseded by ADR-0003" "The original decision."
adr "$r" 0002-new Accepted "Supersedes [ADR 0001](0001-old.md) whole."
adr "$r" 0003-third Accepted "Supersedes ADR 0001 whole."
expect fail "a Superseded by status names another ADR" "$r" "0001 0002 backlink"

# The phrasings the ADRs use, each with its backlink.
r=$(new_case phrasings)
adr "$r" 0001-a Accepted "The budget clause is superseded by [ADR 0002](0002-b.md)."
adr "$r" 0002-b Accepted "This ADR supersedes the budget clause of ADR 0001. The review rule is amended by [ADR 0004](0004-d.md)."
adr "$r" 0003-c Accepted "[ADR 0004](0004-d.md) supersedes everything here about merging. Also see ADR 0001."
adr "$r" 0004-d Accepted "* Supersedes, in ADR 0002: the review rule. Supersedes, in ADR 0003: merging.
* ADR 0005's rule on labels is amended to the tiers.
* Changes ADR 0006 step 7."
adr "$r" 0005-e Accepted "Its rule on labels was amended in [ADR 0004](0004-d.md)."
adr "$r" 0006-f Accepted "7. Never catch panics ([ADR 0004](0004-d.md)). *Amended 2026-10-15: this step said otherwise.*"
expect pass "every phrasing the ADRs use, each recorded" "$r"
rm "$r/docs/adr/0006-f.md"
adr "$r" 0006-f Accepted "7. Never catch panics."
expect fail "a Changes line is an amendment" "$r" "0006 0004 backlink"
rm "$r/docs/adr/0005-e.md"
adr "$r" 0005-e Accepted "Its rule on labels."
expect fail "an 'is amended to' line is an amendment" "$r" "0005 0004 backlink"

r=$(new_case subject-not-amender)
adr "$r" 0001-a Proposed "[ADR 0002](0002-b.md) supersedes the merge clause; ADR 0003 stands."
adr "$r" 0002-b Accepted "Supersedes, in ADR 0001: the merge clause."
adr "$r" 0003-c Accepted "Unrelated."
except "$r" "0001 0002 status awaits the maintainer"
expect pass "an ADR named before 'supersedes' is the amender, not the amended" "$r"

r=$(new_case unknown-adr)
adr "$r" 0002-new Accepted "Supersedes ADR 0009."
expect fail "an amendment of an ADR that does not exist" "$r" "ADR 0009"

r=$(new_case no-status)
printf '# Old\n\nNo front matter.\n' >"$r/docs/adr/0001-old.md"
expect fail "an ADR without a status" "$r" "0001-old.md" "status"

r=$(new_case status-line)
printf '# Old\n\n* Status: Accepted\n\nThe budget clause is superseded by [ADR 0002](0002-new.md).\n' >"$r/docs/adr/0001-old.md"
adr "$r" 0002-new Accepted "$new_amend"
expect pass "a MADR status line outside front matter" "$r"

r=$(new_case ledger)
adr "$r" 0001-old Accepted "Nothing."
plan "$r" 002-b "| 2026-10-20 | $d201 | **Second feature decision** with several more words than eight in it | Proposed |" \
  "| 2026-10-20 | $d202 | Settled one | Accepted |"
plan "$r" 001-a "| 2026-10-20 | $d102 | \`Later\` row | Proposed; part resolved |" \
  "| 2026-10-20 | $d101 | First row | Proposed |"
expect pass "the Proposed ledger rows are listed and never fail" "$r" "3 Proposed" "$d201 Second feature decision with several more words than…"
listing=$(grep -E '^  [0-9]' <<<"$out" | tr '\n' '|')
want="  001-a $d101 First row|  001-a $d102 \`Later\` row|  002-b $d201 Second feature decision with several more words than…|"
if [[ $listing == "$want" ]]; then
  pass=$((pass + 1))
  echo "ok   the ledger listing is sorted and skips Accepted rows and other tables"
else
  fail=$((fail + 1))
  echo "FAIL the ledger listing is sorted and skips Accepted rows and other tables"
  echo "     | got:  $listing"
  echo "     | want: $want"
fi
first=$out
expect pass "the ledger listing is byte-identical on a second run" "$r"
[[ $first == "$out" ]] && { pass=$((pass + 1)); echo "ok   two runs print the same bytes"; } ||
  { fail=$((fail + 1)); echo "FAIL two runs print the same bytes"; }

r=$(new_case ledger-with-failure)
adr "$r" 0001-old Accepted "Nothing here names the newer record."
adr "$r" 0002-new Accepted "$new_amend"
plan "$r" 001-a "| 2026-10-20 | $d101 | First row | Proposed |"
expect fail "the listing still prints when the lint fails" "$r" "$d101 First row" "0001 0002 backlink"

# Every feature numbers its decisions D- (ADR 0021); a row in another form fails, naming it.
r=$(new_case ledger-old-letter)
adr "$r" 0001-old Accepted "Nothing."
plan "$r" 002-b "| 2026-10-20 | $d201 | Fine | Accepted |" '| 2026-10-20 | H-003 | An earlier letter | Accepted |'
expect fail "a ledger row whose ID is not D- fails" "$r" "specs/002-b/plan.md: ledger row H-003: a decision ID is D-"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
