#!/usr/bin/env bash
# Tests for scripts/gen-ts-types.sh: each case is a throwaway tree with a stub
# cargo on PATH that "exports" the case's fresh/ files, so nothing is built.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
gen="$here/../gen-ts-types.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# The stub copies fresh/ into TS_RS_EXPORT_DIR, as ts-rs's export tests would. It
# fails unless the arguments force TS_RS_LARGE_INT to bigint over any cargo config,
# fails if TS_RS_LARGE_INT reaches it from the environment, and fails as cargo does when fresh/ has a
# file named FAIL.
mkdir "$tmp/bin"
cat >"$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
[[ "$*" == "--config env.TS_RS_LARGE_INT.value=\"bigint\" --config env.TS_RS_LARGE_INT.force=true test -p openplush-wasm --lib --locked export_bindings" ]] ||
  { echo "stub: unexpected arguments: $*" >&2; exit 97; }
[[ -z ${TS_RS_LARGE_INT+set} ]] || { echo "stub: TS_RS_LARGE_INT is set" >&2; exit 98; }
[[ -n ${TS_RS_EXPORT_DIR:-} && $TS_RS_EXPORT_DIR == /* ]] || { echo "stub: no absolute export dir" >&2; exit 99; }
[[ ! -e fresh/FAIL ]] || { echo "stub: test failed" >&2; exit 101; }
mkdir -p "$TS_RS_EXPORT_DIR"
cp -R fresh/. "$TS_RS_EXPORT_DIR"
EOF
chmod +x "$tmp/bin/cargo"
export PATH="$tmp/bin:$PATH"

out_dir=packages/engine-client/src/generated
good_error='export type EngineErrorDto = { code: string, message: string, locations?: Locations, proposals?: Array<ProposalDto>, };'
pass=0
fail=0

# expect <pass|fail> <case name> <root> <args> [<text the output must contain>]
expect() {
  local want=$1 name=$2 root=$3 args=$4 pattern=${5:-}
  local out status=0 argv=()
  read -ra argv <<<"$args"
  out=$("$gen" "${argv[@]}" "$root" 2>&1) || status=$?
  if [[ ($want == pass && $status -eq 0) || ($want == fail && $status -ne 0) ]] &&
    [[ -z $pattern || $out == *"$pattern"* ]]; then
    pass=$((pass + 1))
    echo "ok   $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name (wanted $want, exit $status)"
    while IFS= read -r line; do echo "     | $line"; done <<<"$out"
  fi
}

# check_that <case name> <command...>: a follow-up assertion on a case's tree.
check_that() {
  local name=$1
  shift
  if "$@"; then
    pass=$((pass + 1))
    echo "ok   $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name"
  fi
}

# new_case <name>: fresh/ holds a valid export (EngineErrorDto.ts and Progress.ts),
# and the committed directory holds the same files.
new_case() {
  local root="$tmp/$1"
  mkdir -p "$root/fresh" "$root/$out_dir"
  echo "$good_error" >"$root/fresh/EngineErrorDto.ts"
  echo 'export type Progress = { "kind": "done" };' >"$root/fresh/Progress.ts"
  cp "$root/fresh/"* "$root/$out_dir/"
  echo "$root"
}

root=$(new_case write)
echo stale >"$root/$out_dir/Renamed.ts"
echo 'export type Progress = { "kind": "working" };' >"$root/fresh/Progress.ts"
expect pass "write replaces the committed files" "$root" ""
check_that "write updates a changed file" diff -q "$root/fresh/Progress.ts" "$root/$out_dir/Progress.ts"
check_that "write removes a stale file" test ! -e "$root/$out_dir/Renamed.ts"

root=$(new_case write-creates)
rm -rf "${root:?}/packages"
expect pass "write creates the output directory" "$root" ""
check_that "write creates every file" diff -rq "$root/fresh" "$root/$out_dir"

root=$(new_case check-clean)
expect pass "check passes when nothing changed" "$root" "--check"

root=$(new_case check-changed)
echo 'export type Progress = { "kind": "working" };' >"$root/fresh/Progress.ts"
expect fail "check fails on a changed file" "$root" "--check" "Progress.ts"
check_that "check leaves the committed files alone" \
  grep -q '"done"' "$root/$out_dir/Progress.ts"

root=$(new_case check-missing)
rm "$root/$out_dir/Progress.ts"
expect fail "check fails on a file not yet committed" "$root" "--check" "Progress.ts"

root=$(new_case check-stale)
echo stale >"$root/$out_dir/Renamed.ts"
expect fail "check fails on a stale committed file" "$root" "--check" "Renamed.ts"

for mode in "" "--check"; do
  label=${mode:-write}
  root=$(new_case "bigint-$label")
  echo 'export type Count = { n: bigint };' >"$root/fresh/Count.ts"
  cp "$root/fresh/Count.ts" "$root/$out_dir/"
  expect fail "$label fails on a bigint" "$root" "$mode" "bigint"

  root=$(new_case "nullable-$label")
  echo "${good_error/locations?: Locations/locations: Locations | null}" >"$root/fresh/EngineErrorDto.ts"
  cp "$root/fresh/EngineErrorDto.ts" "$root/$out_dir/"
  expect fail "$label fails when locations is not optional" "$root" "$mode" "locations?:"

  root=$(new_case "optional-null-$label")
  echo "${good_error/locations?: Locations/locations?: Locations | null}" >"$root/fresh/EngineErrorDto.ts"
  cp "$root/fresh/EngineErrorDto.ts" "$root/$out_dir/"
  expect fail "$label fails when optional locations is also nullable" "$root" "$mode" "locations?:"

  root=$(new_case "no-error-dto-$label")
  rm "$root/fresh/EngineErrorDto.ts" "$root/$out_dir/EngineErrorDto.ts"
  expect fail "$label fails when EngineErrorDto is not exported" "$root" "$mode" "EngineErrorDto.ts"

  root=$(new_case "cargo-fails-$label")
  touch "$root/fresh/FAIL"
  expect fail "$label fails when the export fails" "$root" "$mode" "test failed"
done

root=$(new_case large-int)
TS_RS_LARGE_INT=number expect pass "TS_RS_LARGE_INT is unset before the export" "$root" "--check"

root=$(new_case bigint-write-untouched)
echo 'export type Count = { n: bigint };' >"$root/fresh/Count.ts"
expect fail "write with a bigint fails" "$root" "" "bigint"
check_that "write with a bigint writes nothing" test ! -e "$root/$out_dir/Count.ts"

expect fail "unknown option fails" "$(new_case bad-option)" "--chek" "unknown option"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
