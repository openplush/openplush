#!/usr/bin/env bash
# Tests for scripts/gen-status.py: each case builds a throwaway specs/ tree and
# asserts what the generator writes.
#
# What matters: a tick shows in the next run's counts, the output is
# byte-identical run to run, and a line that looks like a task
# but is not one, or a tasks.md with no tasks, fails rather than being miscounted.
set -euo pipefail
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
gen="$here/../gen-status.py"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

# expect <pass|fail> <case name> <root> [<text the failure output must contain>]
expect() {
  local want=$1 name=$2 root=$3 pattern=${4:-}
  local out status=0
  out=$("$gen" "$root" 2>&1) || status=$?
  if [[ $want == pass && $status -eq 0 ]] ||
    [[ $want == fail && $status -eq 1 && ( -z $pattern || $out == *"$pattern"* ) ]]; then
    ok "$name"
  else
    bad "$name (wanted $want, exit $status)" "$out"
  fi
}

# Task IDs, split so the hygiene check does not read these fixtures as spec references.
T1="T""001" T2="T""002" T2a="T""002a" T3="T""003" T9="T""009"

# tasks <root> <feature> <lines>...: write a feature's tasks.md around the lines.
tasks() {
  local file="$1/specs/$2/tasks.md"
  shift 2
  mkdir -p "$(dirname "$file")"
  printf '%s\n' "# Tasks" "" "## Phase 1" "" "$@" "" "Prose that mentions - [x] mid-line is not a task." >"$file"
}

# new_case <name>: three features, created in an order that is not sorted either
# way round (directory listings often return creation order or its reverse), with
# the include generated.
new_case() {
  local root="$tmp/$1"
  mkdir -p "$root/docs/start"
  tasks "$root" 002-first-feature "- [x] $T1 done" "- [ ] $T2 open" "- [ ] $T3 open"
  tasks "$root" 010-later-feature "- [ ] $T1 [US1] open" "- [x] $T2 done" "- [x] $T2a done"
  tasks "$root" 001-earliest-feature "- [x] $T1 done"
  "$gen" "$root" >/dev/null
  echo "$root"
}

inc=docs/start/status-counts.inc
gh=https://github.com/openplush/openplush/blob/main/specs

r=$(new_case table)
want="| Feature | Tasks ticked |
|---|---|
| [001 earliest feature]($gh/001-earliest-feature/tasks.md) | 1 of 1 |
| [002 first feature]($gh/002-first-feature/tasks.md) | 1 of 3 |
| [010 later feature]($gh/010-later-feature/tasks.md) | 2 of 3 |"
got=$(grep -v '^<!--' "$r/$inc" | sed '/^$/d')
if [[ $got == "$want" ]]; then ok "one row per feature, in directory order, ticked of all"; else bad "one row per feature, in directory order, ticked of all" "$got"; fi

# Directory listings come in creation or hash order, so a few names may list sorted by chance.
r="$tmp/order"
mkdir -p "$r/docs/start"
for feature in 007-g 003-c 010-j 001-a 012-l 005-e 002-b 011-k; do tasks "$r" "$feature" "- [x] $T1 done"; done
"$gen" "$r" >/dev/null
got=$(grep -o 'specs/[^/]*/tasks.md)' "$r/$inc" | tr '\n' ' ')
want="specs/001-a/tasks.md) specs/002-b/tasks.md) specs/003-c/tasks.md) specs/005-e/tasks.md) specs/007-g/tasks.md) specs/010-j/tasks.md) specs/011-k/tasks.md) specs/012-l/tasks.md) "
if [[ $got == "$want" ]]; then ok "rows in sorted directory order"; else bad "rows in sorted directory order" "$got"; fi

r=$(new_case stale)
sed -i "s/^- \\[ \\] $T3/- [x] $T3/" "$r/specs/002-first-feature/tasks.md"
"$gen" "$r" >/dev/null
if grep -qF "002-first-feature/tasks.md) | 2 of 3 |" "$r/$inc"; then ok "a tick shows in the next run"; else bad "a tick shows in the next run" "$(cat "$r/$inc")"; fi

r=$(new_case twice)
cp "$r/$inc" "$tmp/first.inc"
"$gen" "$r" >/dev/null
if cmp -s "$tmp/first.inc" "$r/$inc"; then ok "two runs are byte-identical"; else bad "two runs are byte-identical" "$(diff "$tmp/first.inc" "$r/$inc")"; fi
if ! grep -q $'\r' "$r/$inc" && [[ $(tail -c 1 "$r/$inc" | od -An -c | tr -d ' ') == '\n' ]]; then
  ok "LF endings and a trailing newline"
else
  bad "LF endings and a trailing newline" "$(od -c "$r/$inc" | tail -n 3)"
fi

r=$(new_case empty)
tasks "$r" 003-no-tasks
expect fail "a tasks.md with no task lines fails" "$r" "specs/003-no-tasks/tasks.md"
r=$(new_case no-specs)
rm -r "$r/specs"
expect fail "no tasks.md at all fails" "$r" "no specs/*/tasks.md"

# Each looks like a task to a reader, so counting or skipping it silently would be wrong.
for line in "- [X] $T9 upper-case" "  - [x] $T9 indented" "* [ ] $T9 other bullet" "- [x] no task ID" \
  "- [x]$T9 no space" "- [~] $T9 other mark" "- [] $T9 empty box" "- [ ] t009 lower-case ID" \
  "- [x ] $T9 stray space" "- [ x] $T9 leading space"; do
  r=$(new_case "malformed-${line//[^a-zA-Z0-9]/_}")
  cp "$r/$inc" "$tmp/before.inc"
  tasks "$r" 002-first-feature "- [x] $T1 done" "$line"
  expect fail "a malformed task line fails: '$line'" "$r" "specs/002-first-feature/tasks.md:6"
  if cmp -s "$tmp/before.inc" "$r/$inc"; then ok "a failed run writes nothing: '$line'"; else bad "a failed run writes nothing: '$line'"; fi
done

# Every feature numbers its tasks T (ADR 0021); an earlier letter still reads as a task, so it fails.
for line in "- [x] H012 an earlier letter" "- [ ] U042 another earlier letter"; do
  r=$(new_case "old-letter-${line//[^a-zA-Z0-9]/_}")
  tasks "$r" 002-first-feature "- [x] $T1 done" "$line"
  expect fail "a task line with an earlier letter fails: '$line'" "$r" "specs/002-first-feature/tasks.md:6: a task ID is T"
done

r=$(new_case link-bullet)
tasks "$r" 002-first-feature "- [x] $T1 done" "- [the plan](plan.md) is a link, not a box"
expect pass "a bullet that starts with a link is not a task box" "$r"

# kind <root> <feature> <value>: give a feature's tasks.md a front matter `kind:` line.
kind() {
  local file="$1/specs/$2/tasks.md"
  printf '%s\n' "---" "description: \"Task list\"" "kind: $3" "---" "" | cat - "$file" >"$file.new"
  mv "$file.new" "$file"
}

# Process work (the factory, the gate) is no maker-visible progress, so it is listed apart.
r=$(new_case process)
tasks "$r" 005-process-work "- [x] $T1 done" "- [ ] $T2 open"
kind "$r" 005-process-work process
"$gen" "$r" >/dev/null
want="| Feature | Tasks ticked |
|---|---|
| [001 earliest feature]($gh/001-earliest-feature/tasks.md) | 1 of 1 |
| [002 first feature]($gh/002-first-feature/tasks.md) | 1 of 3 |
| [010 later feature]($gh/010-later-feature/tasks.md) | 2 of 3 |
Repository process, with no change a maker sees:
| Feature | Tasks ticked |
|---|---|
| [005 process work]($gh/005-process-work/tasks.md) | 1 of 2 |"
got=$(grep -v '^<!--' "$r/$inc" | sed '/^$/d')
if [[ $got == "$want" ]]; then ok "a kind: process feature is listed apart, after the product features"; else bad "a kind: process feature is listed apart, after the product features" "$got"; fi

r=$(new_case product-kind)
kind "$r" 002-first-feature product
expect pass "kind: product is the default, spelled out" "$r"
if ! grep -q "Repository process" "$r/$inc"; then ok "no process heading without a process feature"; else bad "no process heading without a process feature" "$(cat "$r/$inc")"; fi

r=$(new_case unknown-kind)
cp "$r/$inc" "$tmp/before.inc"
kind "$r" 002-first-feature tooling
expect fail "an unknown kind fails, naming the file" "$r" "specs/002-first-feature/tasks.md: unknown kind 'tooling'"
if cmp -s "$tmp/before.inc" "$r/$inc"; then ok "a failed kind writes nothing"; else bad "a failed kind writes nothing"; fi

status=0
out=$("$gen" --bogus "$tmp" 2>&1) || status=$?
if [[ $status -eq 2 && $out == *"unknown option"* ]]; then ok "an unknown option exits 2"; else bad "an unknown option exits 2 (got $status)" "$out"; fi

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
