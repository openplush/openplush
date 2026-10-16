#!/usr/bin/env bash
# Generates the shell's TypeScript protocol types from openplush-wasm's serde DTOs
# with ts-rs (docs/adr/0010-protocol-types.md), into packages/engine-client/src/generated/.
#
# Both modes fail, before anything is written, on:
#   - any `bigint`: a JSON number decodes to a JS number, so a 64-bit integer's type
#     and value would disagree;
#   - an optional field typed `?: T | null`, in any file, or EngineErrorDto not
#     declaring `locations?:` at all: without #[ts(optional)], ts-rs types a skipped
#     Option as `T | null` and only warns.
# --check exports into a temporary directory and fails on any difference, in either
# direction, from the committed files, so a changed, missing or stale file fails.
#
# Usage: scripts/gen-ts-types.sh [--check] [root]   (default root: the repository)
# Tests: scripts/tests/gen-ts-types.test.sh
set -euo pipefail

check=0
root=
for arg in "$@"; do
  case $arg in
    --check) check=1 ;;
    -*)
      echo "gen-ts-types: unknown option '$arg'" >&2
      exit 2
      ;;
    *) root=$arg ;;
  esac
done
root="${root:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$root"

die() {
  echo "gen-ts-types: $*" >&2
  exit 1
}

out=packages/engine-client/src/generated
fresh="$(mktemp -d)"
trap 'rm -rf "$fresh"' EXIT

# Unset and forced on the command line, so neither the environment nor a cargo
# config's [env] can switch the bigint check off: the CLI value outranks a config's
# table form, and cargo refuses to merge it with a config's plain string form.
unset TS_RS_LARGE_INT
TS_RS_EXPORT_DIR="$fresh" cargo --config 'env.TS_RS_LARGE_INT.value="bigint"' \
  --config env.TS_RS_LARGE_INT.force=true test -p openplush-wasm --lib --locked export_bindings ||
  die "the ts-rs export failed"

if hits=$(grep -rnw bigint "$fresh"); then
  die "64-bit integers are banned from the protocol; bigint in:"$'\n'"${hits//$fresh\//}"
fi

if hits=$(grep -rn '?: [^,]*| null' "$fresh"); then
  die "an optional field is also typed | null (is #[ts(optional)] missing?):"$'\n'"${hits//$fresh\//}"
fi

error_dto="$fresh/EngineErrorDto.ts"
[[ -f $error_dto ]] || die "EngineErrorDto.ts was not exported"
grep -q 'locations?:' "$error_dto" ||
  die "EngineErrorDto.ts must declare locations?: (is #[ts(optional)] missing?):"$'\n'"$(cat "$error_dto")"

if ((check)); then
  [[ -d $out ]] || die "$out is missing; run scripts/gen-ts-types.sh"
  if ! changes=$(diff -r "$out" "$fresh"); then
    die "$out differs from a fresh export; run scripts/gen-ts-types.sh and commit the result:"$'\n'"${changes//$fresh/<fresh export>}"
  fi
else
  rm -rf "$out"
  mkdir -p "$out"
  cp -R "$fresh/." "$out"
fi
