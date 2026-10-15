#!/usr/bin/env bash
# Tests for scripts/check-js-licenses.sh: each case builds a throwaway tree with
# installed-package dirs and a stub package manager on PATH, so no Node is needed.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-js-licenses.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# The stub prints the case's canned `licenses list --json` output and answers
# `view <spec> license --json` from the case's views.json. It fails loudly when a
# case has no listing, so "never called" is testable, and fails as the registry
# does for a spec views.json lacks.
mkdir "$tmp/bin"
cat >"$tmp/bin/pnpm" <<'EOF'
#!/usr/bin/env bash
if [[ $# -eq 4 && $1 == view && $3 == license && $4 == --json ]]; then
  [[ -f views.json ]] && jq -e --arg s "$2" 'has($s)' views.json >/dev/null ||
    { echo "stub: ERR_PNPM_FETCH_404 $2" >&2; exit 1; }
  jq -j --arg s "$2" '.[$s]' views.json
  exit
fi
[[ "$*" == "licenses list --json" ]] || { echo "stub: unexpected arguments: $*" >&2; exit 97; }
[[ -f licenses.out ]] || { echo "stub: called, but this case has no listing" >&2; exit 98; }
cat licenses.out
EOF
chmod +x "$tmp/bin/pnpm"
export PATH="$tmp/bin:$PATH"

pass=0
fail=0

# expect <pass|fail> <case name> <root> [<text the output must contain>]
expect() {
  local want=$1 name=$2 root=$3 pattern=${4:-}
  local out status=0
  out=$(cd "$tmp" && "$check" "$root" 2>&1) || status=$?
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

# new_case <name> [<allow list body>]: a tree with an allowlist and a lockfile with
# an empty packages section, but nothing installed yet. The entry under snapshots
# is never installed or in the registry, so reading it as a package fails.
new_case() {
  local root="$tmp/$1"
  mkdir -p "$root"
  printf '[licenses]\nallow = [%s]\n' "${2-\"MIT\", \"Apache-2.0\", \"Zlib\"}" >"$root/deny.toml"
  printf "lockfileVersion: '9.0'\n\nimporters:\n\n  .: {}\n\npackages:\n\nsnapshots:\n\n  ghost@9.9.9: {}\n" \
    >"$root/pnpm-lock.yaml"
  echo "$root"
}

# lock <root> <key>: add a key to the lockfile's packages section, quoted as pnpm
# quotes scoped names.
lock() {
  local key=$2
  [[ $key == @* ]] && key="'$key'"
  awk -v k="$key" '{ print } /^packages:/ { print "  " k ":"; print "    resolution: {integrity: x}" }' \
    "$1/pnpm-lock.yaml" >"$1/lock.tmp"
  mv "$1/lock.tmp" "$1/pnpm-lock.yaml"
}

# registry <root> <spec> <raw view output>: what `pnpm view <spec> license --json` prints.
registry() {
  local views="{}"
  [[ -f $1/views.json ]] && views=$(cat "$1/views.json")
  jq --arg s "$2" --arg o "$3" '.[$s] = $o' <<<"$views" >"$1/views.json"
}

# install <root> <name> <license JSON fragment> [<key pnpm reports>]: lock and
# install a package declaring the fragment (e.g. '"license": "MIT"', or '' for
# none) and list it under the key pnpm would report.
install() {
  local root=$1 name=$2 fragment=$3 key=${4:-MIT} dir
  lock "$root" "$name@1.0.0"
  dir="$root/node_modules/$name"
  mkdir -p "$dir"
  printf '{"name": "%s", "version": "1.0.0"%s}\n' "$name" "${fragment:+, $fragment}" >"$dir/package.json"
  local entry
  entry=$(jq -n --arg n "$name" --arg p "$dir" '{name: $n, versions: ["1.0.0"], paths: [$p]}')
  local listing="{}"
  [[ -f $root/licenses.out ]] && listing=$(cat "$root/licenses.out")
  jq --arg k "$key" --argjson e "$entry" '.[$k] += [$e]' <<<"$listing" >"$root/licenses.out"
}

# one <pass|fail> <case name> <license JSON fragment> [<pattern>]: a single package.
one() {
  local r
  r=$(new_case "$(tr -c 'a-zA-Z0-9\n' - <<<"$2")")
  install "$r" pkg "$3"
  expect "$1" "$2" "$r" "${4:-}"
}

one pass "MIT" '"license": "MIT"' "1 package"
one fail "GPL-3.0 is named" '"license": "GPL-3.0"' "pkg@1.0.0: GPL-3.0"
one pass "(MIT OR GPL-3.0)" '"license": "(MIT OR GPL-3.0)"'
one fail "GPL-2.0 OR LGPL-3.0" '"license": "GPL-2.0 OR LGPL-3.0"' "pkg@1.0.0: GPL-2.0 OR LGPL-3.0"
one pass "MIT AND Zlib" '"license": "MIT AND Zlib"'
one fail "MIT AND GPL-3.0" '"license": "MIT AND GPL-3.0"' "pkg@1.0.0: MIT AND GPL-3.0"
one fail "(MIT OR Apache-2.0) AND Zlib mixes AND and OR" '"license": "(MIT OR Apache-2.0) AND Zlib"' "mixes AND and OR"
one pass "Apache-2.0 WITH LLVM-exception OR MIT" '"license": "Apache-2.0 WITH LLVM-exception OR MIT"'
one fail "Apache-2.0 WITH LLVM-exception alone" '"license": "Apache-2.0 WITH LLVM-exception"' "pkg@1.0.0"
one fail "MIT AND Apache-2.0 WITH LLVM-exception" '"license": "MIT AND Apache-2.0 WITH LLVM-exception"' "pkg@1.0.0"
one fail "a missing license" '' "pkg@1.0.0"
one fail "SEE LICENSE IN" '"license": "SEE LICENSE IN LICENSE.md"' "pkg@1.0.0: SEE LICENSE IN LICENSE.md"
one fail "UNLICENSED" '"license": "UNLICENSED"' "pkg@1.0.0: UNLICENSED"
one fail "a two-entry licenses array" '"licenses": [{"type": "MIT"}, {"type": "Zlib"}]' "pkg@1.0.0"
one pass "a one-entry licenses array" '"licenses": [{"type": "MIT"}]'
one pass "legacy {type: MIT}" '"license": {"type": "MIT"}'
one fail "IDs are case-sensitive: mit" '"license": "mit"' "pkg@1.0.0: mit"

r=$(new_case guess-ignored)
install "$r" pkg '"license": "GPL-3.0"' MIT
expect fail "the declared license wins over the listed guess" "$r" "pkg@1.0.0: GPL-3.0"

r=$(new_case sorted)
install "$r" zeta '"license": "GPL-3.0"'
install "$r" alpha '"license": "MIT"'
install "$r" beta '"license": "AGPL-3.0"' AGPL-3.0
out=$("$check" "$r" 2>&1 || true)
if [[ $(grep -E '^ *[a-z]+@' <<<"$out" | sed -E 's/^ *//; s/@.*//' | paste -sd' ') == "beta zeta" ]]; then
  pass=$((pass + 1)); echo "ok   every violation, sorted"
else
  fail=$((fail + 1)); echo "FAIL every violation, sorted"
  while IFS= read -r line; do echo "     | $line"; done <<<"$out"
fi

r=$(new_case empty-allowlist "")
install "$r" pkg '"license": "MIT"'
expect fail "an empty allowlist" "$r" "allow"

r=$(new_case no-allowlist)
install "$r" pkg '"license": "MIT"'
printf '[licenses]\n' >"$r/deny.toml"
expect fail "a missing allowlist" "$r" "allow"

r=$(new_case no-lockfile)
install "$r" pkg '"license": "MIT"'
rm "$r/pnpm-lock.yaml"
expect fail "a missing lockfile" "$r" "pnpm-lock.yaml"

r=$(new_case no-packages)
printf "lockfileVersion: '9.0'\n\nimporters:\n\n  .: {}\n" >"$r/pnpm-lock.yaml"
registry "$r" "dep@1.0.0" '"MIT"'
expect pass "no packages: passes without listing" "$r" "no npm dependencies"

# Packages pnpm does not list (an os, cpu or libc that does not match this
# machine) are judged by the license the registry says they declare.
r=$(new_case unlisted-gpl)
install "$r" pkg '"license": "MIT"'
lock "$r" "@scope/bin-arm64@1.0.0"
registry "$r" "@scope/bin-arm64@1.0.0" '"GPL-3.0"'
expect fail "an unlisted package the registry says is GPL-3.0" "$r" "@scope/bin-arm64@1.0.0: GPL-3.0"

r=$(new_case unlisted-mit)
install "$r" pkg '"license": "MIT"'
lock "$r" "@scope/bin-arm64@1.0.0"
registry "$r" "@scope/bin-arm64@1.0.0" $'"MIT"\n'
expect pass "an unlisted package the registry says is MIT" "$r" "2 packages"

r=$(new_case unlisted-view-fails)
install "$r" pkg '"license": "MIT"'
lock "$r" "gone@1.0.0"
expect fail "a registry lookup that fails" "$r" "gone@1.0.0"

r=$(new_case unlisted-view-empty)
lock "$r" "bare@1.0.0"
registry "$r" "bare@1.0.0" ""
printf '{}' >"$r/licenses.out"
expect fail "a registry lookup that prints nothing" "$r" "bare@1.0.0"

r=$(new_case unlisted-view-not-json)
lock "$r" "odd@1.0.0"
registry "$r" "odd@1.0.0" "MIT"
printf '{}' >"$r/licenses.out"
expect fail "a registry lookup that prints no JSON" "$r" "odd@1.0.0"

r=$(new_case empty-listing)
lock "$r" "dep@1.0.0"
registry "$r" "dep@1.0.0" '"GPL-3.0"'
printf '{}' >"$r/licenses.out"
expect fail "an empty listing does not pass vacuously" "$r" "dep@1.0.0: GPL-3.0"

r=$(new_case empty-key-listing)
lock "$r" "dep@1.0.0"
registry "$r" "dep@1.0.0" '"GPL-3.0"'
printf '{"MIT": []}' >"$r/licenses.out"
expect fail "a listing of empty keys does not pass vacuously" "$r" "dep@1.0.0: GPL-3.0"

r=$(new_case peer-suffix)
install "$r" pkg '"license": "MIT"'
lock "$r" "pkg@1.0.0(react@18.0.0)"
expect pass "a lockfile key's peer suffix is not part of its version" "$r" "1 package"

r=$(new_case versionless-key)
install "$r" pkg '"license": "MIT"'
lock "$r" "loose"
registry "$r" "loose" '"MIT"'
expect fail "a lockfile key without a version is not looked up as the latest" "$r" "loose: lockfile key is not name@version"

# listing <root> <JSON>: replace the canned listing.
listing() { printf '%s' "$2" >"$1/licenses.out"; }

r=$(new_case no-paths)
lock "$r" "pkg@1.0.0"
listing "$r" '{"MIT": [{"name": "pkg", "versions": ["1.0.0"], "paths": []}]}'
expect fail "a listed package with no paths" "$r" "listed without one path per version"

r=$(new_case fewer-paths)
install "$r" pkg '"license": "MIT"'
lock "$r" "pkg@2.0.0"
listing "$r" "{\"MIT\": [{\"name\": \"pkg\", \"versions\": [\"1.0.0\", \"2.0.0\"], \"paths\": [\"$r/node_modules/pkg\"]}]}"
expect fail "a listed package with fewer paths than versions" "$r" "listed without one path per version"

r=$(new_case package-json-not-object)
install "$r" pkg '"license": "MIT"'
printf '["MIT"]\n' >"$r/node_modules/pkg/package.json"
expect fail "a package.json that is not an object" "$r" "pkg@1.0.0: unreadable package.json"

for shape in '[]' '{"MIT": "pkg"}' '{"MIT": ["pkg"]}'; do
  r=$(new_case "bad-shape-$(tr -c 'a-zA-Z\n' - <<<"$shape")")
  lock "$r" "pkg@1.0.0"
  listing "$r" "$shape"
  expect fail "a listing shaped $shape" "$r" "not an object of lists of package objects"
done

r=$(new_case not-json)
printf 'No licenses in packages found\n' >"$r/licenses.out"
expect fail "non-JSON listing while packages exist" "$r" "JSON"

r=$(new_case no-package-json)
install "$r" pkg '"license": "MIT"'
rm "$r/node_modules/pkg/package.json"
expect fail "an installed package without package.json" "$r" "pkg@1.0.0"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
