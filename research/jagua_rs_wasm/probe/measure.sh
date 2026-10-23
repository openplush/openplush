#!/usr/bin/env bash
# Builds the probe with and without jagua-rs at opt-level z and s, runs
# wasm-bindgen (--target nodejs), calls the export under Node, and prints
# raw and gzip -9 sizes. Run from this directory.
set -euo pipefail
out="${1:-target/sizes}"
mkdir -p "$out"
for opt in z s; do
  for variant in baseline jagua; do
    features=()
    [[ $variant == jagua ]] && features=(--features jagua)
    CARGO_PROFILE_RELEASE_OPT_LEVEL="$opt" cargo build --quiet --release \
      --target wasm32-unknown-unknown "${features[@]}"
    dir="$out/$variant-$opt"
    rm -rf "$dir"
    wasm-bindgen --target nodejs --out-dir "$dir" \
      target/wasm32-unknown-unknown/release/jagua_size_probe.wasm
    raw=$(stat -c %s target/wasm32-unknown-unknown/release/jagua_size_probe.wasm)
    bg=$(stat -c %s "$dir/jagua_size_probe_bg.wasm")
    gz=$(gzip -9 -c "$dir/jagua_size_probe_bg.wasm" | wc -c)
    js=$(stat -c %s "$dir/jagua_size_probe.js")
    ran=$(node -e "const m=require('./$dir/jagua_size_probe.js');console.log(m.nest_length(false), m.nest_length(true))")
    printf '%-8s opt=%s  cargo=%8d  bindgen=%8d  gzip=%7d  js=%6d  run=%s\n' \
      "$variant" "$opt" "$raw" "$bg" "$gz" "$js" "$ran"
  done
done
