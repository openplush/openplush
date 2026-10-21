#!/usr/bin/env bash
# Publishes the workspace rustdoc in the docs site: documents every workspace crate,
# copies cargo's doc directory to target/site/dev/api/ without rustdoc's .lock, and
# writes api/index.html, which stable rustdoc does not, listing the crates in name order.
#
# Each crate gets its own `cargo doc`, after the workspace crates it depends on.
# rustdoc links an item of another workspace crate only if that crate's doc
# directory exists when it starts, and cargo deletes a crate's directory as it
# starts documenting it, so one parallel `cargo doc --workspace` links differently
# from run to run. The doc directory is cleaned first, so a crate gone from the
# workspace leaves no pages and rustdoc's shared indexes start empty.
#
# Fails, with no api/ left behind, on a workspace crate with no index.html, on any
# dotfile but .lock in rustdoc's output, and on a crate with more than one
# documented target, since the index links one page per crate.
#
# Usage: scripts/assemble-docs-api.sh [root]   (default root: the repository)
# Tests: scripts/tests/assemble-docs-api.test.sh
set -euo pipefail
shopt -s inherit_errexit
export LC_ALL=C

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$root"

die() {
  echo "docs-api: $*" >&2
  exit 1
}

out=target/site/dev/api
rm -rf "$out"

meta=$(cargo metadata --format-version 1 --locked)
doc="$(jq -r .target_directory <<<"$meta")/doc"

# One line per workspace crate, in name order: its package name, its doc directory,
# then the workspace crates it depends on (normal dependencies only: a dev-dependency
# never appears in the docs).
crates=$(jq -r '
  .workspace_members as $ws
  | [.packages[] | select(.id as $id | $ws | index($id))] as $members
  | ($members | map({key: .id, value: .name}) | from_entries) as $name
  | (.resolve.nodes | map({key: .id, value: .deps}) | from_entries) as $deps
  | $members | sort_by(.name)[]
  | [.targets[] | select(.doc) | .name | gsub("-"; "_")] as $dirs
  | if ($dirs | length) != 1 then error("\(.name) has \($dirs | length) documented targets; the index links one per crate") else . end
  | [.name, $dirs[0], ($deps[.id][] | select(any(.dep_kinds[]; .kind == null)) | $name[.pkg] // empty)]
  | join(" ")' <<<"$meta" 2>&1) || die "${crates#jq: error (at *): }"

declare -A dir_of=() deps_of=() documented=()
names=()
while read -r name dir deps; do
  names+=("$name")
  dir_of[$name]=$dir
  deps_of[$name]=$deps
done <<<"$crates"

# ready <name>: every workspace crate it depends on is documented.
ready() {
  local d
  for d in ${deps_of[$1]}; do [[ -n ${documented[$d]:-} ]] || return 1; done
}

run() {
  echo "+ $*"
  "$@"
}

run cargo clean --doc
# Each pass documents the first crate in name order whose dependencies are done.
while ((${#documented[@]} < ${#names[@]})); do
  next=
  for name in "${names[@]}"; do
    [[ -z ${documented[$name]:-} ]] && ready "$name" && { next=$name; break; }
  done
  [[ -n $next ]] || die "the workspace's dependencies form a cycle"
  run cargo doc --no-deps --locked -p "$next"
  documented[$next]=1
done

for name in "${names[@]}"; do
  [[ -f $doc/${dir_of[$name]}/index.html ]] || die "rustdoc wrote no ${dir_of[$name]}/index.html for $name"
done
# Pages publishes every file; one rustdoc starts writing must be looked at, not shipped unseen.
dotfiles=$(cd "$doc" && find . -name '.*' ! -name . ! -path ./.lock | sed 's|^\./||' | sort)
[[ -z $dotfiles ]] || die "rustdoc wrote dotfiles besides .lock:"$'\n'"$dotfiles"

mkdir -p "$out"
cp -R "$doc/." "$out"
rm -f "$out/.lock"

{
  cat <<'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>OpenPlush API reference</title>
<style>
:root { color-scheme: light dark; }
body { font-family: system-ui, sans-serif; line-height: 1.5; max-width: 40rem; margin: 2rem auto; padding: 0 1rem; }
</style>
</head>
<body>
<h1>OpenPlush API reference</h1>
<p>The rustdoc of every workspace crate, generated from the code this site was built from.</p>
<ul>
EOF
  for name in "${names[@]}"; do
    echo "<li><a href=\"${dir_of[$name]}/index.html\">$name</a></li>"
  done
  cat <<'EOF'
</ul>
<p><a href="../index.html">Back to the contributor book</a></p>
</body>
</html>
EOF
} >"$out/index.html"
echo "docs-api: ok (${#names[@]} crates)"
