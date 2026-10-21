#!/usr/bin/env bash
# Tests for scripts/assemble-docs-api.sh: each case is a throwaway workspace with a
# stub cargo on PATH that answers `cargo metadata` from the case's metadata.json and
# "documents" a crate by writing a few files, so nothing is built.
#
# What matters: every workspace crate, and nothing else, is listed in name order;
# a crate is documented after the workspace crates it depends on; the output has no
# dotfile and no page from an earlier run; two runs give identical bytes; and a
# missing crate or a leftover dotfile fails, naming it, with no api/ written.
set -euo pipefail
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
assemble="$here/../assemble-docs-api.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# The stub runs in the case's root, as the script cds there. `doc` appends the
# package to doc-calls, writes nothing for a package listed in skip-docs, writes
# the path named in the file dotfile (if any) under the doc directory, and fails
# when the file fail-doc exists.
mkdir "$tmp/bin"
cat >"$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
doc="$PWD/cargo-target/doc"
case "$*" in
  "metadata --format-version 1 --locked") cat metadata.json ;;
  "clean --doc") rm -rf "$doc" ;;
  "doc --no-deps --locked -p "*)
    name=$5
    echo "$name" >>doc-calls
    [[ ! -e fail-doc ]] || { echo "stub: rustdoc failed" >&2; exit 101; }
    grep -qxF "$name" skip-docs 2>/dev/null && exit 0
    dir=$(jq -r --arg n "$name" '.packages[] | select(.name == $n) | .targets[] | select(.doc) | .name | gsub("-"; "_")' metadata.json)
    mkdir -p "$doc/$dir" "$doc/static.files" "$doc/src/$dir"
    : >"$doc/.lock"
    echo "<h1>$dir</h1>" >"$doc/$dir/index.html"
    echo "<pre>$dir</pre>" >"$doc/src/$dir/lib.rs.html"
    echo "body{}" >"$doc/static.files/rustdoc.css"
    echo "$dir" >>"$doc/crates.js"
    if [[ -f dotfile ]]; then
      mkdir -p "$(dirname "$doc/$(cat dotfile)")"
      : >"$doc/$(cat dotfile)"
    fi
    ;;
  *) echo "stub: unexpected arguments: $*" >&2; exit 97 ;;
esac
EOF
chmod +x "$tmp/bin/cargo"
export PATH="$tmp/bin:$PATH"

api=target/site/dev/api
pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

# expect <pass|fail> <case name> <root> [<text the output must contain>]
expect() {
  local want=$1 name=$2 root=$3 pattern=${4:-}
  local out status=0
  out=$("$assemble" "$root" 2>&1) || status=$?
  if [[ ($want == pass && $status -eq 0) || ($want == fail && $status -ne 0) ]] &&
    [[ -z $pattern || $out == *"$pattern"* ]]; then
    ok "$name"
  else
    bad "$name (wanted $want, exit $status)" "$out"
  fi
}

# check_that <case name> <command...>: a follow-up assertion on a case's tree.
check_that() {
  local name=$1
  shift
  if "$@"; then ok "$name"; else bad "$name"; fi
}

# metadata <root> <spec>...: write the case's metadata.json. A spec is
# name[:kind[:deps]], kind "lib" (the default), "bin" or "lib+bin", deps comma-separated
# workspace packages, each a normal dependency, or a dev-dependency with a "dev-" prefix.
# glam, a dependency of every lib and no workspace member, is always there.
metadata() {
  local root=$1 specs
  shift
  specs=$(printf '%s\n' "$@" | jq -R 'split(":") | {name: .[0], kind: (.[1] // "lib"), deps: ((.[2] // "") | split(",") | map(select(. != "")))}' | jq -s .)
  jq -n --arg root "$root" --argjson specs "$specs" '
    def id($n): "path+file:///ws/\($n)#0.0.0";
    def target($n; $k): {name: $n, kind: [$k], doc: true};
    {
      packages: ([$specs[] | {id: id(.name), name: .name, targets: (
          if .kind == "bin" then [target(.name; "bin")]
          elif .kind == "lib+bin" then [target(.name | gsub("-"; "_"); "lib"), target("\(.name)-tool"; "bin")]
          else [target(.name | gsub("-"; "_"); "lib")] end
          + [{name: "it", kind: ["test"], doc: false}])}]
        + [{id: "registry+glam#0.30.0", name: "glam", targets: [target("glam"; "lib")]}]),
      workspace_members: [$specs[] | id(.name)],
      resolve: {nodes: ([$specs[] | {id: id(.name), deps: (
          [.deps[] | if startswith("dev-") then
              {pkg: id(ltrimstr("dev-")), dep_kinds: [{kind: "dev"}]}
            else {pkg: id(.), dep_kinds: [{kind: null}]} end]
          + [{pkg: "registry+glam#0.30.0", dep_kinds: [{kind: null}]}])}]
        + [{id: "registry+glam#0.30.0", deps: []}])},
      target_directory: "\($root)/cargo-target"
    }' >"$root/metadata.json"
}

# new_case <name>: a workspace of three crates: openplush-geom; openplush-seams,
# which depends on it; and openplush-cli, a binary.
new_case() {
  local root="$tmp/$1"
  mkdir -p "$root"
  metadata "$root" openplush-seams::openplush-geom openplush-geom openplush-cli:bin
  echo "$root"
}

# hrefs <index.html>: the index's links, in page order.
hrefs() { grep -o 'href="[^"]*"' "$1" | sed 's/^href="//; s/"$//'; }

# links_resolve <api dir>: every link in its index.html that stays inside api/ is a file.
links_resolve() {
  local h
  [[ -f $1/index.html ]] || return 1
  for h in $(hrefs "$1/index.html" | grep -v '^\.\./'); do [[ -f $1/$h ]] || return 1; done
}

# lacks <extended regex> <file>...: every file exists and none matches.
lacks() {
  local pattern=$1 f
  shift
  for f in "$@"; do [[ -f $f ]] && ! grep -Eq "$pattern" "$f" || return 1; done
}

root=$(new_case complete)
expect pass "a workspace is assembled" "$root"
check_that "the index lists every workspace crate in name order, and nothing else" \
  diff <(hrefs "$root/$api/index.html" | grep '/index.html$' | grep -v '^\.\./') \
  <(printf '%s\n' openplush_cli/index.html openplush_geom/index.html openplush_seams/index.html)
check_that "the index names each crate by its package name" \
  grep -q '>openplush-cli<' "$root/$api/index.html"
check_that "every link in the index into api/ resolves" links_resolve "$root/$api"
check_that "rustdoc's shared files are copied" \
  test -f "$root/$api/crates.js" -a -f "$root/$api/static.files/rustdoc.css" -a -f "$root/$api/src/openplush_geom/lib.rs.html"
check_that "rustdoc's .lock is dropped" test ! -e "$root/$api/.lock"
check_that "a dependency outside the workspace is neither documented nor listed" \
  lacks glam "$root/doc-calls" "$root/$api/index.html"
check_that "the index has no date or time" \
  lacks '[0-9]{4}-[0-9]{2}-[0-9]{2}|[0-9]{2}:[0-9]{2}' "$root/$api/index.html"

root=$(new_case twice)
expect pass "a first run" "$root"
[[ -d $root/$api ]] && cp -R "$root/$api" "$tmp/twice-1"
expect pass "a second run" "$root"
check_that "two runs give identical bytes" diff -r "$tmp/twice-1" "$root/$api"

root="$tmp/order"
mkdir -p "$root"
metadata "$root" openplush-a::openplush-z openplush-m openplush-z::dev-openplush-a
expect pass "a dev-dependency cycle is no cycle" "$root"
check_that "a crate is documented after the workspace crates it depends on, then in name order" \
  diff "$root/doc-calls" <(printf '%s\n' openplush-m openplush-z openplush-a)

root=$(new_case stale-api)
mkdir -p "$root/$api/openplush_gone"
echo old >"$root/$api/openplush_gone/index.html"
expect pass "a run over an earlier api/" "$root"
check_that "a page from an earlier run is gone" test ! -e "$root/$api/openplush_gone"

root=$(new_case stale-doc)
mkdir -p "$root/cargo-target/doc/openplush_gone"
echo old >"$root/cargo-target/doc/openplush_gone/index.html"
expect pass "a run over cargo's earlier doc output" "$root"
check_that "a crate no longer in the workspace is not published" test ! -e "$root/$api/openplush_gone"

root=$(new_case missing)
echo openplush-seams >"$root/skip-docs"
mkdir -p "$root/$api"
echo old >"$root/$api/index.html"
expect fail "a crate rustdoc wrote nothing for fails" "$root" "openplush-seams"
check_that "a missing crate leaves no api/" test ! -e "$root/$api"

for dotfile in .nojekyll openplush_geom/.hidden; do
  root=$(new_case "dotfile-${dotfile//\//-}")
  echo "$dotfile" >"$root/dotfile"
  expect fail "a leftover dotfile fails: $dotfile" "$root" "$dotfile"
  check_that "a leftover dotfile leaves no api/: $dotfile" test ! -e "$root/$api"
done

root=$(new_case doc-fails)
touch "$root/fail-doc"
expect fail "a failing cargo doc fails" "$root" "rustdoc failed"
check_that "a failing cargo doc leaves no api/" test ! -e "$root/$api"

root="$tmp/two-targets"
mkdir -p "$root"
metadata "$root" openplush-geom openplush-tool:lib+bin
expect fail "a crate with two documented targets fails" "$root" "openplush-tool"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
