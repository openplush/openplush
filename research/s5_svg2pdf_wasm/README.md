# Spike 001 T005: `svg2pdf` under WASM vs. writing PDF directly

**Status (2026-10-15): closed.** Decision: [ADR 0008](../../docs/adr/0008-pdf-export.md) (Accepted).

This is non-shipping research code. Nothing in `crates/`, `apps/`, or `packages/` imports it. It is a standalone Cargo project with its own `[workspace]` table and `Cargo.lock`, so the root workspace (`members = ["crates/*"]`) and `scripts/check.sh` never see it.

## The question

Task 001 T005 asks whether `svg2pdf` works on `wasm32-unknown-unknown`, or whether 001 T091 (`openplush-export/src/pdf.rs`, tiled PDF) should write PDF directly. PDF is in v0.1 because browsers print SVG at unreliable scale (FR-061), so the deciding properties are:

- the 50 mm scale-check square measures exactly 50 mm in the PDF;
- byte-identical output natively and on wasm32, with no clock, document ID, or randomness (Constitution V; `crates/AGENTS.md`);
- text for labels without system fonts or fetched fonts (Constitution VIII), and without a font whose license is off the allowlist (Constitution IX);
- `cargo deny` and the boundary check pass; WASM size is reasonable.

## Method

| Step | What | Why |
|---|---|---|
| Inventory | `cargo info`, the published manifests, `cargo tree --target wasm32-unknown-unknown` with svg2pdf's default features and with none; grep for clock, IDs, fs, threads | Finds fonts-from-disk, codecs, threads, and timestamps before any build |
| Policy gates | `cargo deny check` against the repo's `deny.toml` (which sets `all-features = true`, so the `svg2pdf-default` feature puts the default graph under the check), and against a scratch copy with `all-features = false` for the minimal graph | Licenses, bans, advisories, sources |
| Page | `src/lib.rs`: one A3-landscape page (two A4 sheets' worth) holding a 400-vertex lobed piece outline about 360 × 260 mm, a 50.00 mm scale-check square, and three labels ("A3", "Body side", "50 mm"). Page space is mm, y down | One representative page, built in code, no file I/O |
| Harness A | `src/via_svg.rs`: the page as SVG (1 user unit = 1 mm, `width="420mm"`), parsed by `usvg`, converted by `svg2pdf::to_pdf`, no default features | The svg2pdf route |
| Harness B | `src/direct.rs`: the same page with `pdf-writer` 0.15: geometry in mm under one `cm` (mm → pt, y flip), text in standard-14 Helvetica, not embedded | The direct route |
| Cross-target test | `tests/cross_target.rs`: each writer twice gives the same bytes, no `/CreationDate`, `/ModDate`, or `/ID`, and an FNV-1a hash equal to the native release build's. Runs as `#[test]` natively and `#[wasm_bindgen_test]` on wasm32 under Node | A wasm32 pass means byte-identical to native |
| Scale oracle | `test_scale.py`: pypdf decodes the content streams; a small interpreter tracks the CTM through `q`/`Q`/`cm` and Form XObjects and measures the square in page space | Independent of both writers |
| Size | `examples/probe_*.rs`: plain `extern "C"` cdylibs (empty, direct, svg2pdf; svg2pdf again with `--features svg2pdf-default`), release profile `opt-level = "s"`, LTO, `codegen-units = 1`, `panic = "abort"`, strip; gzip -9; no `wasm-opt` | Delta over an empty module |
| Font probe | Scratch, not committed: svg2pdf with `text` and DejaVu Sans loaded into `usvg`'s fontdb from bytes, 3 conversions × 2 processes | Whether embedded-font output stays deterministic |

## Run it

From this directory, with the repo toolchain and `wasm-bindgen-test-runner` 0.2.129 (`scripts/check.sh --setup`):

```sh
cargo run --release                                     # writes target/s5/*.pdf; sizes, hashes, time per page
cargo test --release                                    # native oracle
cargo test --release --target wasm32-unknown-unknown    # same tests on wasm32 under Node
python3 -m unittest -v test_scale                       # scale oracle (needs pypdf==6.17.0; run cargo first)
cargo build --release --target wasm32-unknown-unknown --examples   # size probes
cargo deny --manifest-path Cargo.toml --config ../../deny.toml check
```

## Findings

Full output in [`results.md`](./results.md).

1. **Both routes run on wasm32 and match native byte for byte** [solid]. Neither writes a date or an `/ID`; pdf-writer writes them only when asked. svg2pdf writes `/Producer (svg2pdf)`, a constant. Its compression (`miniz_oxide`) is deterministic.
2. **Both get the scale exactly right, but svg2pdf only with a non-default option** [solid]. The square measures 141.73228 pt = 50.0000 mm in both. With svg2pdf's default `PageOptions { dpi: 72 }`, a `width="420mm"` SVG prints **4/3 too large** (the square measures 66.67 mm): usvg resolves `mm` at CSS 96 px/in, and svg2pdf then maps 1 px to 1 pt. No error, no warning. 001 T091 would have to pin `dpi: 96` and test it.
3. **Text is where svg2pdf costs most** [solid]. With no fonts in usvg's fontdb, `<text>` is dropped with only a `log::warn!`, with or without svg2pdf's `text` feature: the PDF has no labels and no error. A silent-wrong-output trap. Giving it a font means embedding one:
   - its `text` feature pulls `fontdb` with default features (`fs`, `memmap`, `fontconfig`: filesystem and `unsafe` mmap code compiled into the engine graph), plus `rustybuzz` and `ttf-parser`, which fail `cargo deny check advisories` (unmaintained, RUSTSEC-2026-0206 and -0192);
   - **no font crate with an allowlisted license was found**. `dejavu` 2.37.0 says `MIT/Apache-2.0` but ships Bitstream Vera-licensed fonts; `damascene-fonts-roboto` 0.1.0 says Apache-2.0, but its Roboto 3.015 names the SIL OFL 1.1 in the font's own `name` table. `cargo deny` sees only the crate's manifest license, so both would pass the gate while shipping an off-allowlist license (Constitution IX needs an ADR for OFL);
   - with a font supplied (scratch probe), embedding is deterministic: a subset Type0/CIDFontType2 font with a hash-derived subset tag, the same 13,309 bytes in every run.

   The direct route needs none of this: Helvetica is a standard-14 font, so nothing is embedded, nothing is fetched, and labels are real text. The cost is WinAnsi only (Latin-1-ish), and the viewer chooses the actual glyphs [solid; FR-064 warning text in other scripts is a [verify]].
4. **Size** [solid]: over a 70-byte empty module, the direct probe adds 92.5 KB (43.7 KB gzip), svg2pdf with no default features 609.5 KB (256.8 KB gzip), and with its defaults 2.93 MB (1.06 MB gzip). The minimal svg2pdf graph also carries two `pdf-writer` (0.12.1 and 0.15.0) and two `miniz_oxide` versions.
5. **Speed** (native, release, opt-level s): direct ≈ 45 µs per page, svg2pdf ≈ 0.9–1.0 ms per page, mostly SVG text formatting and re-parsing [likely]. Both are far below the ~50 ms yield budget.
6. **Tiling favours the direct route** [likely]. `svg2pdf::to_pdf` makes exactly one page. Multi-page output needs `svg2pdf::to_chunk` (one Form XObject per SVG) assembled with `pdf-writer` 0.12, which svg2pdf does not re-export and which is three minor versions behind. So 001 T091 would write pages, page map, registration marks and the warning page with pdf-writer anyway, and use svg2pdf only to turn our own geometry into an SVG string and parse it back. The direct route places each tile with one `cm` and clips with `re W n`.
7. **Precision**: both routes write `f32` numbers. Error at 1 m is about 1e-4 mm [solid by arithmetic]. svg2pdf's chain of transforms (0.75 × 3.7795274, a 0.000061 pt translate residue) leaves the page width 0.0001 pt off; harmless for print.
