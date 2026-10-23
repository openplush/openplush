#!/usr/bin/env bash
# An ADR that another one amends or supersedes says so, both ways, and its status
# agrees: an Accepted record cannot amend one that was never accepted. A finding
# that only the maintainer can settle (a status is a decision) is listed, with its
# reason, in scripts/adr-lifecycle-exceptions.txt rather than fixed by an agent.
#
# Reads docs/adr/NNNN-*.md except 0000-template.md. The status is the front
# matter's `status:` or, failing that, a MADR `Status:` line. The body is read one
# sentence at a time (a line split at ". "); fenced code is skipped. An ADR is named
# as `ADR NNNN`, `ADR-NNNN` or a link to `NNNN-*.md`; a self-reference is ignored.
#   A amends B when a sentence of A has `supersedes`, `amends`, or a leading
#     `Changes`, with no ADR named before the verb, and names B after it
#     ("Supersedes, in ADR 0014: …", "This ADR supersedes … ADR 0014",
#     "Changes ADR 0006 step 7"); or when it names B before `is amended to` or
#     `are amended to` ("ADR 0014's … are amended to …").
#   B claims A amends it when its status is `Superseded by ADR-A`, or a sentence
#     of B names A after `superseded by` or `amended by`, or names A before
#     `supersedes` or `amends` ("[ADR 0018](…) supersedes two clauses").
#   B records A when its status is `Superseded by ADR-A`, or a line of B names A
#     and contains a word starting `supersed` or `amend`.
# Fails, naming both ADRs and the fix, when:
#   1. an Accepted ADR A amends B and B is still Proposed     (key: B A status);
#   2. an Accepted ADR A amends B and B does not record A     (key: B A backlink);
#   3. B claims A amends it and A does not say it amends B    (key: A B backlink);
#   and, with no exception possible, when an ADR has no status, two ADRs share a
#   number, or an amendment names an ADR that does not exist.
# Each key that only the maintainer can settle is a line `<key> <reason>` in the
# exceptions file; a line whose key no longer matches a finding fails as stale.
# Also fails on a Decision Ledger row whose ID is not D- and three digits (ADR 0021).
# Also prints, never failing, the Proposed rows of every specs/*/plan.md Decision
# Ledger (feature, ID, first eight words), sorted: decisions agents cannot make.
#
# Known limits: the reading is textual. A sentence split at an abbreviation, a
# verb this header does not list, or an amendment spread over two sentences is
# missed; review of the ADR is the layer behind it.
# Usage: scripts/check-adr-lifecycle.sh [root]   (default: the repository)
# Tests: scripts/tests/check-adr-lifecycle.test.sh
set -euo pipefail
export LC_ALL=C

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$root"
exceptions=scripts/adr-lifecycle-exceptions.txt

die() {
  echo "adr-lifecycle: $*" >&2
  exit 1
}

# Reads ADR files; prints one record per fact: `S n status`, `F a b` (a amends b),
# `C b a` (b claims a amends it), `M b a` (b records a), or `E message`.
parse_awk='
function refs(s,    out, seen, m, n) {
  out = ""
  while (match(s, /ADR[- ][0-9][0-9][0-9][0-9]|[0-9][0-9][0-9][0-9]-[A-Za-z0-9_-]+\.md/)) {
    m = substr(s, RSTART, RLENGTH)
    s = substr(s, RSTART + RLENGTH)
    n = (m ~ /^ADR/) ? substr(m, 5, 4) : substr(m, 1, 4)
    if (n != self && !(n in seen)) { seen[n] = 1; out = out " " n }
  }
  return out
}
function emit(kind, list,    k, a, i) {
  k = split(list, a, " ")
  for (i = 1; i <= k; i++) print kind, self, a[i]
}
function sentence(s,    lc, p) {
  lc = tolower(s)
  if (match(lc, /(superseded|amended) by/)) emit("C", refs(substr(s, RSTART)))
  if (match(lc, /(is|are) amended to/)) emit("F", refs(substr(s, 1, RSTART - 1)))
  p = 0
  if (match(lc, /(^|[^a-z])(supersedes|amends)([^a-z]|$)/)) p = RSTART
  else if (s ~ /^[*> 0-9.-]*Changes /) p = 1
  if (!p) return
  if (refs(substr(s, 1, p)) == "") emit("F", refs(substr(s, p)))
  else emit("C", refs(substr(s, 1, p)))
}
function flush() {
  if (self == "") return
  if (status == "") { print "E " file ": no status; add `status:` to its front matter"; return }
  print "S", self, status
  if (match(status, /^Superseded by ADR[- ]?[0-9][0-9][0-9][0-9]/)) print "C", self, substr(status, RLENGTH - 3, 4)
}
FNR == 1 {
  flush()
  file = FILENAME; self = FILENAME; sub(/.*\//, "", self); self = substr(self, 1, 4)
  status = ""; fm = ($0 == "---"); fence = 0
  if (fm) next
}
fm { if ($0 == "---") fm = 0; else if (sub(/^status:[ \t]*/, "")) status = $0; next }
/^```/ { fence = !fence; next }
fence { next }
status == "" && /^[*-]?[ \t]*[Ss]tatus:/ { sub(/^[*-]?[ \t]*[Ss]tatus:[ \t]*/, ""); status = $0; next }
{
  if (tolower($0) ~ /supersed|amend/) emit("M", refs($0))
  n = split($0, parts, /\. /)
  for (i = 1; i <= n; i++) sentence(parts[i])
}
END { flush() }
'

# Reads parse_awk records; prints `E message`, `V key message` (tab-separated)
# and `N count` (amendment pairs seen).
judge_awk='
BEGIN { OFS = "\t" }
$1 == "E" { sub(/^E /, ""); print "E", $0; next }
$1 == "S" {
  st = $0; sub(/^S [0-9]+ /, "", st)
  if ($2 in status) print "E", "two ADRs are numbered " $2 "; renumber the newer one"
  status[$2] = st; next
}
$1 == "F" { fwd[$2 SUBSEP $3] = 1; pair[$2 SUBSEP $3] = 1; next }
$1 == "C" { claim[$2 SUBSEP $3] = 1; pair[$3 SUBSEP $2] = 1; next }
$1 == "M" { ment[$2 SUBSEP $3] = 1; next }
function exists(n, by) {
  if (n in status) return 1
  print "E", "ADR " by " names ADR " n " in an amendment, and no docs/adr/" n "-*.md exists; fix the number"
  return 0
}
END {
  for (k in fwd) {
    split(k, p, SUBSEP); a = p[1]; b = p[2]
    if (!exists(b, a) || status[a] !~ /^Accepted/) continue
    if (status[b] ~ /^Proposed/)
      print "V", b " " a " status", "ADR " a " (Accepted) amends or supersedes ADR " b ", which is still Proposed; the maintainer settles ADR " b "'"'"'s status (accept it, or mark it Superseded by ADR-" a ")"
    if (!ment[b SUBSEP a] && status[b] !~ ("^Superseded by ADR[- ]?" a))
      print "V", b " " a " backlink", "ADR " a " (Accepted) amends or supersedes ADR " b ", which does not record it; add a note to ADR " b " that names ADR " a " (\"amended by\" or \"superseded by\"), or set its status to Superseded by ADR-" a
  }
  for (k in claim) {
    split(k, p, SUBSEP); b = p[1]; a = p[2]
    if (exists(a, b) && !fwd[a SUBSEP b])
      print "V", a " " b " backlink", "ADR " b " says ADR " a " amends or supersedes it, and ADR " a " does not say so; name ADR " b " in ADR " a " (\"Supersedes, in ADR " b ": …\"), or correct ADR " b
  }
  n = 0; for (k in pair) n++
  print "N", n
}
'

# Reads one plan.md; prints `  <feature> <ID> <first eight words>` for each
# Proposed row of its Decision Ledger table, or, with -v ids=1, each row whose ID
# is not D- and three digits (ADR 0021).
ledger_awk='
/^## / { inl = ($0 ~ /^## Decision Ledger/); idc = 0; next }
!inl || !/^\|/ { next }
{
  n = split($0, c, "|")
  for (i = 2; i < n; i++) gsub(/^[ \t]+|[ \t]+$/, "", c[i])
  if (!idc) {
    for (i = 2; i < n; i++) { if (c[i] == "ID") idc = i; if (c[i] == "Decision") dc = i; if (c[i] == "Status") sc = i }
    next
  }
  if (c[idc] ~ /^:?-+:?$/) next
  if (ids) {
    if (c[idc] !~ /^D-[0-9][0-9][0-9]$/) print FILENAME ": ledger row " c[idc] ": a decision ID is D- and three digits in every feature (ADR 0021)"
    next
  }
  if (c[sc] !~ /^Proposed/) next
  text = c[dc]; gsub(/\*\*/, "", text)
  w = split(text, words, /[ \t]+/); out = words[1]
  for (i = 2; i <= w && i <= 8; i++) out = out " " words[i]
  if (w > 8) out = out "…"
  print "  " feature " " c[idc] " " out
}
'

# print_proposed: the Proposed ledger rows of every plan, sorted, to stdout.
print_proposed() {
  local f rows
  rows=$(for f in specs/*/plan.md; do
    [[ ! -f $f ]] || awk -v feature="$(basename "$(dirname "$f")")" "$ledger_awk" "$f"
  done | sort)
  echo "adr-lifecycle: $(grep -c . <<<"$rows" || true) Proposed decision rows in specs/*/plan.md, which only the maintainer accepts:"
  [[ -z $rows ]] || printf '%s\n' "$rows"
}

# read_exceptions: the exceptions file's keys, sorted, into $listed; bad lines go
# to $errors.
read_exceptions() {
  local line a b kind reason n=0
  while IFS= read -r line || [[ -n $line ]]; do
    n=$((n + 1))
    [[ $line =~ ^[[:space:]]*(#|$) ]] && continue
    read -r a b kind reason <<<"$line"
    if [[ ! $a =~ ^[0-9]{4}$ || ! ${b:-} =~ ^[0-9]{4}$ ]]; then
      errors+=("$exceptions:$n: expected '<ADR> <ADR> <kind> <reason>': $line")
    elif [[ ${kind:-} != status && ${kind:-} != backlink ]]; then
      errors+=("$exceptions:$n: kind must be status or backlink: $line")
    elif [[ -z ${reason:-} ]]; then
      errors+=("$exceptions:$n: entry has no reason: $line")
    else
      listed+="$a $b $kind"$'\n'
    fi
  done <"$exceptions"
  listed=$(sort -u <<<"$listed" | grep . || true)
}

[[ -d docs/adr ]] || die "docs/adr is missing"
[[ -f $exceptions ]] || die "$exceptions is missing"
adrs=()
for f in docs/adr/[0-9][0-9][0-9][0-9]-*.md; do
  [[ -f $f && $f != docs/adr/0000-template.md ]] && adrs+=("$f")
done

print_proposed
results=""
((${#adrs[@]} == 0)) || results=$(awk "$parse_awk" "${adrs[@]}" | awk "$judge_awk" | sort)
errors=()
while IFS= read -r msg; do
  [[ -n $msg ]] && errors+=("$msg")
done < <(for f in specs/*/plan.md; do [[ ! -f $f ]] || awk -v ids=1 "$ledger_awk" "$f"; done)
while IFS=$'\t' read -r tag msg; do
  [[ $tag == E ]] && errors+=("$msg")
done <<<"$results"
found=$(awk -F'\t' '$1 == "V" { print $2 }' <<<"$results" | sort -u)
listed=""
read_exceptions

while IFS= read -r key; do
  [[ -n $key ]] && errors+=("$(awk -F'\t' -v k="$key" '$1 == "V" && $2 == k { print $3; exit }' <<<"$results"); or list \"$key <reason>\" in $exceptions")
done < <(comm -23 <(printf '%s\n' "$found") <(printf '%s\n' "$listed"))
while IFS= read -r key; do
  [[ -n $key ]] && errors+=("$exceptions: stale entry, no matching finding: $key; remove it")
done < <(comm -13 <(printf '%s\n' "$found") <(printf '%s\n' "$listed"))

if ((${#errors[@]})); then
  printf 'adr-lifecycle: %s\n' "${errors[@]}" >&2
  exit 1
fi
pairs=$(awk -F'\t' '$1 == "N" { print $2 }' <<<"$results")
echo "adr-lifecycle: ok (${#adrs[@]} ADRs, ${pairs:-0} amendments, $(grep -c . <<<"$listed" || true) excepted)"
