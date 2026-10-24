# openplush-export

<!-- index: the true-scale SVG writer and the tiled PDF writer, what each page carries, and their errors. -->

What `openplush-export` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

### SVG

`svg::write(sheet, pieces, summary)` writes one `Sheet` as UTF-8 SVG bytes at true scale: `width` and `height` in mm and a `viewBox` of the same numbers, so 1 user unit is 1 mm. Each placement draws its piece's cut line (class `cut`, solid 0.5 mm) then its sewing line (class `sew`, dashed 0.3 mm), through `Placement::apply`; y is flipped about the sheet's length because sheets are y-up and SVG is y-down, so the print is never mirrored. Coordinates are written to 0.001 mm, so the same input gives the same bytes. The `summary` rows follow as the same lines the PDF cover prints (`N. code: location`, then the explanation and `Fix: remedy`, indented), one `text` element each in the generic `monospace` family at 3.5 mm on a 5 mm pitch, starting 10 mm under the sheet and 10 mm in from its left edge, wrapped to the sheet's width less those margins and never to fewer than 60 columns. The document grows down to hold them, and widens when a sheet is narrower than 60 columns; the pieces stay where they are. With no rows the document is the sheet alone. `&`, `<` and `>` are escaped. Errors: `UnknownPiece`, `DuplicatePiece`, and `Unprintable` for summary text outside Latin-1.

`ExportError`: `UnknownPiece { piece }` (a placement whose piece was not given), `DuplicatePiece { piece }`, and `Unprintable { text, character }` (summary text the output cannot print).

### PDF

`pdf::write(sheet, pieces, tiling, summary)` writes the same sheet as a tiled PDF, every page the paper's size (`Paper::size_mm`). Its catalog sets `/ViewerPreferences << /PrintScaling /None >>`, which asks a viewer's print dialog not to scale the pages; viewers may ignore it, so the cover says so too.

Page 1 is a cover: the same labelled scale-check squares at the printable area's top-left corner, labelled "Page map" instead of a tile label, with two lines of 3 mm text right of the squares under that label ("Print at Actual size, 100%, not Fit to page." and "Check that the square at left measures 50 mm before you cut."), and below them, 5 mm down, a map of the sheet scaled to fit the rest of the printable area from its top-left corner: every placed copy's cut line in 60% gray (0.2 mm), the sheet's outline (0.4 mm), and each tile's window (0.2 mm) with its label (4 mm text) 2 mm in from the window's top-left corner.

When `summary` (`SummaryRow::new(code, location, explanation, remedy)`, the pipeline's rendering of its unresolved warnings) is not empty, the cover's bottom carries it in Courier (3 mm text, 4 mm line pitch, wrapped by characters at word breaks to the printable width, continuing lines indented 3 characters): "Unresolved warnings: N", then per row `N. code: location`, the explanation and `Fix: remedy`; the map shrinks to the room left above it, 5 mm higher, but always keeps half its room. Lines that do not fit continue on pages after the last tile, each headed "Unresolved warnings, continued", and the cover's last line says "Continued on page P.", so tile page numbers never depend on the warnings. Text is printed in WinAnsi, so printable ASCII and Latin-1; any other character but whitespace is refused with `ExportError::Unprintable { text, character }`, and runs of whitespace print as one space.

Then one page per `Page` of a `PageTiling` made from that sheet: the page's sheet window is mapped onto its printable area by one `cm` at 72 / 25.4 pt per mm, with no flip since sheets and PDF are both y-up, and clipped to it; every placed copy's cut line (solid 0.5 mm) then sewing line (dashed 3 mm on, 2 mm off, 0.3 mm) is drawn on every page, followed by a crosshair for each of `PageTiling::marks()` inside the page's window (arms 5 mm each way, 0.2 mm lines), so each mark prints on both pages that share it, then the 50 mm scale-check square at `Page::scale_square_min()` with the 25.4 mm square in its top-left corner (0.3 mm lines), labelled "50 mm" and "1 in" inside (3 mm text), and the page's label (`Page::label()`, such as "B3", 5 mm text) 5 mm right of the square, its baseline 5 mm below the square's top.

Text is in standard fonts with WinAnsi encoding, which the viewer supplies, so no font is embedded: labels and scale text in Helvetica, all ASCII; the warning summary in Courier. No date, producer, info dictionary or file ID is written and nothing is compressed, so the same input gives the same bytes. Its errors are `svg::write`'s, and `Unprintable` for summary text.

## Depends on

`openplush-geom`, `openplush-layout`, `openplush-pattern`, `pdf-writer` (exactly 0.15.0), `thiserror`
