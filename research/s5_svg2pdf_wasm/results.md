# S5 results (2026-10-15)

## Versions

| Tool / crate | Version |
|---|---|
| rustc / cargo | 1.97.0 (2d8144b78 2026-07-07) / 1.97.0 (c980f4866 2026-06-30) |
| `svg2pdf` | 0.13.0 (latest on crates.io on 2026-10-15), MIT OR Apache-2.0 |
| `usvg` | 0.45.1 (svg2pdf's; latest is 0.48.1) |
| `pdf-writer` | 0.15.0 (direct route; latest), and 0.12.1 (svg2pdf's own), MIT OR Apache-2.0 |
| `miniz_oxide` | 0.8.9 (svg2pdf) and 0.9.1 (via usvg → flate2) |
| With svg2pdf defaults | `fontdb` 0.23.0, `resvg` 0.45.1, `tiny-skia` 0.11.4, `image` 0.25.10, `subsetter` 0.2.6, `ttf-parser` 0.25.1, `rustybuzz` 0.20.1 |
| `wasm-bindgen` / `wasm-bindgen-test` / runner | 0.2.129 / 0.3.79 / 0.2.129 (same as the root `Cargo.lock`) |
| Node | v22.22.0 |
| Python / pypdf | 3.13.16 / 6.17.0 (BSD-3-Clause, no runtime dependencies) |
| cargo-deny | 0.20.2, against the repo's `deny.toml` |
| Host | Linux x86_64, 4 cores |

The oracle first used pdfminer.six (MIT), but it requires `cryptography` → `cffi`, which is MIT-0, not on the allowlist. pypdf replaced it.

## 1. Dependency inventory

`svg2pdf` 0.13.0 features: `default = [image, filters, text]`; `image` → `image` (png, gif, jpeg decoders); `filters` → `tiny-skia` + `resvg/raster-images` (rasterises filtered groups); `text` → `usvg/text`, `resvg/text`, `fontdb`, `subsetter`, `ttf-parser`, `siphasher`. It depends on `usvg` 0.45 with `default-features = false`, but on `fontdb = "0.23"` **with fontdb's default features** (`std`, `fs`, `memmap`, `fontconfig`), so `text` brings filesystem font loading and `memmap2` into the graph, wasm32 included.

| Graph (`cargo tree --target wasm32-unknown-unknown -e normal`) | Crates |
|---|---|
| Harness, svg2pdf `default-features = false` + pdf-writer 0.15 | 37 |
| Same with `--features svg2pdf-default` | 106 |

Not in either graph: `rayon`, `getrandom`, `wasm-bindgen`, `js-sys`, `web-sys`, `time`, `chrono` (boundary rule 1 clean).

Source grep (`svg2pdf` 0.13.0, `pdf-writer` 0.15.0): no `SystemTime`, `Instant`, or RNG. svg2pdf writes `/Producer (svg2pdf)` in the document info, nothing else. pdf-writer writes `/ID` only through `Pdf::set_file_id` and an info dictionary only through `Pdf::document_info`. `svg2pdf::to_pdf` emits exactly one page; `to_chunk` returns a pdf-writer **0.12** `Chunk` (svg2pdf re-exports `usvg`, not `pdf_writer`).

## 2. Policy gates

```
$ cargo deny --manifest-path Cargo.toml --config ../../deny.toml check licenses bans sources
warning[duplicate]: … bitflags, gif, kurbo, miniz_oxide, pdf-writer, png, zune-core, zune-jpeg
bans ok, licenses ok, sources ok

$ cargo deny --manifest-path Cargo.toml --config ../../deny.toml check advisories
error[unmaintained]: `rustybuzz` is unmaintained   ID: RUSTSEC-2026-0206
error[unmaintained]: `ttf-parser` is unmaintained  ID: RUSTSEC-2026-0192
advisories FAILED
```

The repo's `deny.toml` sets `all-features = true`, so the run above covers the `svg2pdf-default` graph. With a scratch copy set to `all-features = false` (the harness as built):

```
warning[duplicate]: found 2 duplicate entries for crate 'miniz_oxide'
warning[duplicate]: found 2 duplicate entries for crate 'pdf-writer'
advisories ok, bans ok, licenses ok, sources ok
```

Both advisories come only from svg2pdf's `text` feature. The direct route alone (`pdf-writer` 0.15 → `bitflags`, `itoa`, `memchr`, `ryu`) is clean.

## 3. Build and run

```
$ cargo test --release
test direct_square_is_drawn_in_mm_under_one_transform ... ok
test direct_is_repeatable_and_matches_native ... ok
test svg2pdf_is_repeatable_and_matches_native ... ok
test result: ok. 3 passed; 0 failed

$ cargo test --release --target wasm32-unknown-unknown
     Running tests/cross_target.rs (target/wasm32-unknown-unknown/release/deps/cross_target-….wasm)
test svg2pdf_is_repeatable_and_matches_native ... ok
test direct_square_is_drawn_in_mm_under_one_transform ... ok
test direct_is_repeatable_and_matches_native ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 filtered out; finished in 0.04s
```

`cargo build --release --target wasm32-unknown-unknown --example probe_svg2pdf --features svg2pdf-default` also builds (1 min 02 s); its tests were not run on wasm32.

## 4. Output (native release, `cargo run --release`)

```
direct              9435 B  hash  2578343843372603017  42.119µs/page
svg2pdf             4921 B  hash 11445119838395906974  1.023415ms/page
svg2pdf_dpi72       4918 B  hash   461375733877581731  1.169629ms/page
```

Three further separate processes gave the same three hashes; with `--features svg2pdf-default` the svg2pdf hash is unchanged (text is still dropped: no fonts). Times vary run to run: direct 42–47 µs, svg2pdf 0.86–1.02 ms per page, mean of 200 runs. The direct PDF is uncompressed; pdf-writer doesn't compress, so T091 would deflate content streams itself (`miniz_oxide`, already in svg2pdf's graph) if size matters.

## 5. Determinism

| Route | Same process, twice | Separate native processes | Native vs wasm32 | Date / ID |
|---|---|---|---|---|
| Direct (pdf-writer 0.15) | identical (asserted) | 4/4 identical | identical (asserted) | none (asserted) |
| svg2pdf, no fonts | identical (asserted) | 4/4 identical | identical (asserted) | none (asserted); `/Producer (svg2pdf)` |
| svg2pdf + DejaVu Sans (scratch) | 3/3 identical, 13,309 B | 2/2 identical (hash 9124801884010855544) | not run | none; subset tag `FCTBWA+DejaVuSans` |

## 6. Scale (`python3 -m unittest -v test_scale`, 5 tests OK)

Measured in page space by the pypdf interpreter (CTM through `q`/`Q`/`cm`/Form XObjects):

| PDF | MediaBox (pt) | Square (pt) | Square (mm) |
|---|---|---|---|
| direct | 1190.5511 × 841.8898 | 141.73228 × 141.73228 | 50.0000 |
| svg2pdf, `PageOptions { dpi: 96 }` | 1190.5513 × 841.8898 | 141.73228 × 141.73228 | 50.0000 |
| svg2pdf, default `dpi: 72` | 1587.4016 × 1122.5197 | 188.97637 × 188.97637 | **66.6667** |

Exact A3 landscape is 1190.5512 × 841.8898 pt. The direct page is one `2.8346457 0 0 -2.8346457 0 841.8898 cm` and the square is `10 237 50 50 re` (asserted in the Rust test). svg2pdf's stream is `0.75 0 0 0.75 0 0 cm` → `1 0 0 -1 0 1122.5197 cm` → `3.7795274 0 0 3.7795274 0.000061035156 0 cm`, then the path in mm.

Labels: pypdf extracts "A3", "Body side", "50 mm" from the direct PDF, and an empty string from the svg2pdf PDF.

## 7. Fonts

| Candidate | Declared crate license | License of the font data | Verdict |
|---|---|---|---|
| `dejavu` 2.37.0 | MIT/Apache-2.0 | Bitstream Vera + DejaVu changes public domain (`name` ID 13; `LICENSE` in the crate) | off the allowlist; `cargo deny` can't see it |
| `damascene-fonts-roboto` 0.1.0 | (MIT OR Apache-2.0) AND Apache-2.0 | Roboto 3.015: "licensed under the SIL Open Font License, Version 1.1" (`name` ID 13), though the crate's `LICENSE-Roboto.txt` says Apache-2.0 | contradictory; OFL is off the allowlist |
| PDF standard-14 Helvetica | n/a | not embedded; supplied by the viewer | used by the direct route |

## 8. WASM size

Probes: `extern "C"` cdylibs, `opt-level = "s"`, LTO, `codegen-units = 1`, `panic = "abort"`, `strip`; no `wasm-opt`.

| Probe | `.wasm` bytes | gzip -9 | Delta over empty |
|---|---|---|---|
| `probe_empty` | 70 | 102 | — |
| `probe_direct` (pdf-writer 0.15) | 92,570 | 43,672 | +92.5 KB (+43.6 KB gz) |
| `probe_svg2pdf` (no default features) | 609,551 | 256,822 | +609.5 KB (+256.7 KB gz) |
| `probe_svg2pdf --features svg2pdf-default` | 2,933,755 | 1,059,490 | +2.93 MB (+1.06 MB gz) |

Both probes include the outline generator (`sin`/`cos`) and float formatting, so the delta between them (≈ 517 KB, 213 KB gz) is the svg2pdf + usvg cost.
