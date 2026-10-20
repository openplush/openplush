---
status: Accepted
date: 2026-10-15
deciders: Lavender (maintainer), accepted 2026-10-18; proposed by the implementing agent from spike S5, T005
---

# Write the tiled PDF directly with `pdf-writer`; label with standard-14 Helvetica, embed no fonts

## Context and problem statement

FR-061 puts PDF print export in v0.1 because browsers print SVG at unreliable scale: tiled Letter and A4 with overlap, registration marks, a page map, and a 50 mm scale-check square on every page, plus the FR-064 warning summary on the first page (D-053 later moved the page map and the summary to a cover page before the tiles). T091 (`crates/openplush-export/src/pdf.rs`) writes it. Research R2 listed `svg2pdf` for this with WASM support `[verify]`.

T005 (spike S5) asks whether `svg2pdf` works under WASM or whether T091 should write PDF directly. Evidence: [`research/s5_svg2pdf_wasm/`](https://github.com/openplush/openplush/blob/main/research/s5_svg2pdf_wasm/README.md).

## Decision drivers

* FR-061: the printed scale must be exact. The scale-check square is the user's only defence against a wrong one.
* Constitution V: byte-identical output, native and WASM. `crates/AGENTS.md`: no clock, no OS randomness. PDFs often carry creation dates and random file IDs.
* Constitution VIII: no fonts fetched at runtime. `crates/AGENTS.md`: no filesystem, so no system fonts either.
* Constitution IX: allowlisted licenses, **including the license of any embedded font data**, which `cargo deny` can't see.
* Root AGENTS.md: no silent wrong output.
* WASM bundle size, the gates (`cargo deny`, `scripts/check-boundary.sh`), and how easily T091 can tile.

## Considered options

1. `svg2pdf` with its default features: render each page as SVG, convert, embed a font for labels.
2. `svg2pdf` without default features (no text, images, or filters) for geometry, assembled into pages with `pdf-writer`; labels drawn with `pdf-writer`.
3. Write the PDF directly with `pdf-writer` 0.15: geometry in mm under one `cm` per page, text in the standard-14 Helvetica, nothing embedded.
4. A higher-level PDF crate (`krilla` 0.8).

## Decision outcome

Chosen option: **3**. It matches option 2 on scale and determinism, costs about a sixth of the WASM size, has no font problem, and is the API T091 would need for pages, marks and the page map anyway.

What S5 found:

* **Both routes run on wasm32 and match native byte for byte; neither writes a date or a file ID** [solid]. pdf-writer writes `/ID` and an info dictionary only when asked. svg2pdf writes a constant `/Producer (svg2pdf)`, and its deflate output is deterministic.
* **Both measure the 50 mm square at exactly 141.73228 pt** [solid], checked by an independent pypdf content-stream interpreter. svg2pdf gets there only with `PageOptions { dpi: 96.0 }`. Its default (72) prints a `mm`-sized SVG **4/3 too large** (the square measures 66.67 mm), with no error and no warning, because usvg resolves `mm` at the CSS 96 px/in.
* **svg2pdf silently drops text when it has no font** [solid], with or without its `text` feature: the labels vanish and only a `log::warn!` remains. Giving it a font needs the `text` feature, which pulls `fontdb` with its default `fs`, `memmap` and `fontconfig` features into the engine graph. It also pulls `rustybuzz` and `ttf-parser`, which fail `cargo deny check advisories` (unmaintained; RUSTSEC-2026-0206, -0192). And it needs a font to embed.
* **No embeddable font with an allowlisted license was found as a crate** [solid for the two checked]. `dejavu` 2.37.0 (crate: MIT/Apache-2.0) ships Bitstream Vera-licensed fonts. `damascene-fonts-roboto` 0.1.0 (crate: Apache-2.0) ships Roboto 3.015, whose own `name` table says SIL OFL 1.1. Both pass `cargo deny`, which reads only the crate manifest.
* **Size over an empty module** [solid]: direct +92.5 KB (+43.6 KB gzip); svg2pdf without defaults +609.5 KB (+256.7 KB gzip); with defaults +2.93 MB (+1.06 MB gzip).
* **Tiling**: `svg2pdf::to_pdf` makes one page. Multi-page output needs `to_chunk` plus `pdf-writer` 0.12, which svg2pdf doesn't re-export and which sits beside 0.15 in the graph [solid]. Under option 2, T091 would write the document structure with pdf-writer anyway and round-trip its own geometry through an SVG string [likely].
* Native time per A3 page: direct ≈ 45 µs, svg2pdf ≈ 0.9–1.0 ms [solid, one page shape].

Why not the others:

* **Option 1** fails `cargo deny` advisories, brings filesystem font code into an engine crate, and has no font to embed under Constitution IX without an ADR adding OFL. It is also 1 MB gzipped.
* **Option 2** works, but it carries a scale trap (the dpi default) and a silent-text-loss trap that T091's tests would have to guard forever. It adds about 213 KB gzipped and two duplicate crate versions. All it saves is writing `m`/`l`/`re` operators, which pattern outlines (polylines in mm) don't need SVG for.
* **Option 4** was not evaluated. `krilla` is MIT OR Apache-2.0 and has an optional `rayon` feature; how it handles dates, IDs, and font embedding is a [verify]. It would face the same font-license question as option 1 [likely].

### Consequences

* Good: one transform maps mm to points (`cm` with 72/25.4, y flipped). The FR-061 square is drawn in mm under it, so the scale is right by construction, and a test can assert the `cm` and the `re` literally.
* Good: a small dependency (`pdf-writer` → `bitflags`, `itoa`, `memchr`, `ryu`), with a clean `cargo deny` result. No font data, no `fs`, nothing fetched (Constitution VIII).
* Good, for tiling: each tile is the same content under a different `cm` translate, clipped to the printable area (`re W n`). The overlap, registration marks, page map, and warning page are plain operators on the same `Content` [likely].
* Bad, **for text**: Helvetica with WinAnsiEncoding covers ASCII and most Latin-1, and nothing else [solid]. The viewer supplies the glyphs, so metrics (and wrapping of the FR-064 warning page) are approximate unless T091 carries the standard Helvetica AFM widths [likely]. T091 must return a **typed error** for a label character outside WinAnsi, never drop or substitute it. Non-Latin labels need an embedded font, which needs an ADR adding OFL-1.1 to the allowlist, or a font under an allowlisted license, then subsetting [speculative on demand].
* Bad: we write PDF structure ourselves. pdf-writer checks syntax, not semantics; a PDF/A or validator check is a [verify] for T091.
* Neutral: pdf-writer doesn't compress. Deflating content streams needs a deflate crate (`miniz_oxide` is MIT OR Zlib OR Apache-2.0 and was deterministic here) [likely]. The A3 page was 9.4 KB uncompressed.
* Neutral: the file has no `/ID`. PDF 1.7 requires one only for encrypted files; PDF 2.0 requires it always [verify]. If wanted, derive it from a hash of the content, never from a clock or RNG.
* Both routes write `f32` numbers: about 1e-4 mm error at 1 m [solid by arithmetic]. Fine for print.

### Confirmation

* T091's tests: the scale-check square on every page measures 50 mm in page space (an independent content-stream oracle, as in S5's `test_scale.py`); same input twice gives the same bytes; native bytes equal wasm32 bytes; no `/CreationDate`, `/ModDate`, or random `/ID`; an unencodable label is a typed error.
* `cargo deny` on the workspace stays green with `pdf-writer` added. Any later font embedding comes with an ADR covering the font's own license.

## Pros and cons of the options

### 1. svg2pdf, default features

* Good, because SVG export (FR-060) and PDF would share one renderer, and embedded fonts give real text in any script.
* Bad, because it fails `cargo deny` advisories, adds `fs`/`memmap` font loading to the engine graph, has no allowlisted font available, and adds 1 MB gzipped.

### 2. svg2pdf without defaults, plus pdf-writer for pages and text

* Good, because it is gate-clean, deterministic, and exact at `dpi: 96`.
* Bad, because of the default-dpi 4/3 trap, silent text loss, +213 KB gzipped over option 3, two `pdf-writer` versions, and an SVG serialise/parse round trip per page.

### 3. Direct pdf-writer, standard-14 Helvetica (chosen)

* Good, because it is the smallest, simplest, and deterministic, with scale by construction, real text with no font data, and a direct fit for tiling.
* Bad, because labels are limited to WinAnsi and text metrics are the viewer's.

### 4. krilla

* Good, because it is a higher-level API from the same authors, with font embedding and subsetting.
* Bad, because it is unevaluated, with the same font-license gap as option 1 [likely].

## More information

* Evidence: [`research/s5_svg2pdf_wasm/README.md`](https://github.com/openplush/openplush/blob/main/research/s5_svg2pdf_wasm/README.md) and [`results.md`](https://github.com/openplush/openplush/blob/main/research/s5_svg2pdf_wasm/results.md): commands, versions, hashes, sizes, and the font license table.
* Follow-ups for the lead's docs PR: research.md R2 `svg2pdf` row (WASM `[verify]` → result, role → not used); research R6 S5 result; tasks.md T005 and T091 ("approach chosen by S5"); plan.md Primary dependencies, if it names `svg2pdf`.
* Revisit when labels or the warning page need non-Latin text, or a font under an allowlisted license (or an OFL ADR) becomes available.
* PDF 1.7 (ISO 32000-1) §8.3 coordinate systems, §9.6.2.2 standard Type 1 fonts, Annex D WinAnsiEncoding.
