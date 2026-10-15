"""Scale oracle for spike S5, independent of both PDF writers.

pypdf decodes each content stream; the small interpreter below tracks the
current transformation matrix through `q`/`Q`/`cm` and Form XObjects (`Do`),
so every path point is measured in page space, in points (PDF 1.7 §8.3, §8.10).
Run `cargo run --release` first: it writes the PDFs into `target/s5/`.
"""

import unittest
from pathlib import Path

from pypdf import PdfReader
from pypdf.generic import ContentStream, DictionaryObject

OUT = Path(__file__).parent / "target" / "s5"
PT_PER_MM = 72.0 / 25.4
SQUARE_PT = 50.0 * PT_PER_MM  # 141.732...
A3_LANDSCAPE_PT = (420.0 * PT_PER_MM, 297.0 * PT_PER_MM)
# pdf-writer and usvg both write f32: 1e-3 pt is ~0.35 µm, far below print error.
TOL_PT = 1e-3

type Matrix = tuple[float, float, float, float, float, float]
type Point = tuple[float, float]
IDENTITY: Matrix = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)


def mul(m: Matrix, n: Matrix) -> Matrix:
    """m then n, in PDF's row-vector convention."""
    a, b, c, d, e, f = m
    p, q, r, s, t, u = n
    return (a * p + b * r, a * q + b * s, c * p + d * r, c * q + d * s,
            e * p + f * r + t, e * q + f * s + u)


def apply(m: Matrix, x: float, y: float) -> Point:
    return (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])


def paths(content: object, resources: DictionaryObject, reader: PdfReader,
          ctm: Matrix) -> list[list[Point]]:
    """Every path in page space; `re` and each `m`-started subpath is one path."""
    out: list[list[Point]] = []
    stack: list[Matrix] = []
    current: list[Point] = []
    for operands, op in ContentStream(content, reader).operations:
        nums = [float(v) for v in operands if isinstance(v, (int, float))]
        if op == b"q":
            stack.append(ctm)
        elif op == b"Q":
            ctm = stack.pop()
        elif op == b"cm":
            ctm = mul((nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]), ctm)
        elif op == b"m":
            if current:
                out.append(current)
            current = [apply(ctm, nums[0], nums[1])]
        elif op in (b"l", b"c", b"v", b"y"):
            current.append(apply(ctm, nums[-2], nums[-1]))
        elif op == b"re":
            x, y, w, h = nums
            out.append([apply(ctm, px, py)
                        for px, py in ((x, y), (x + w, y), (x + w, y + h), (x, y + h))])
        elif op in (b"S", b"s", b"f", b"F", b"f*", b"B", b"B*", b"b", b"b*", b"n"):
            if current:
                out.append(current)
            current = []
        elif op == b"Do":
            xobject = resources["/XObject"][operands[0]].get_object()
            if xobject["/Subtype"] == "/Form":
                m = [float(v) for v in xobject.get("/Matrix", list(IDENTITY))]
                inner = mul((m[0], m[1], m[2], m[3], m[4], m[5]), ctm)
                out += paths(xobject, xobject.get("/Resources", resources), reader, inner)
    return out


def load(name: str) -> tuple[PdfReader, float, float, list[list[Point]]]:
    path = OUT / f"{name}.pdf"
    if not path.is_file():
        raise FileNotFoundError(f"{path}: run `cargo run --release` first")
    reader = PdfReader(path)
    if len(reader.pages) != 1:
        raise ValueError(f"{path}: expected one page, got {len(reader.pages)}")
    page = reader.pages[0]
    found = paths(page.get_contents(), page["/Resources"], reader, IDENTITY)
    return reader, float(page.mediabox.width), float(page.mediabox.height), found


def square_sides(found: list[list[Point]]) -> list[Point]:
    """Width and height of every path with exactly four distinct corners."""
    sides: list[Point] = []
    for path in found:
        corners = sorted(set((round(x, 6), round(y, 6)) for x, y in path))
        if len(corners) == 4:
            xs, ys = [c[0] for c in corners], [c[1] for c in corners]
            sides.append((max(xs) - min(xs), max(ys) - min(ys)))
    return sides


class ScaleCheckSquare(unittest.TestCase):
    """Prevents printing a pattern whose 50 mm square does not measure 50 mm."""

    def assert_true_scale(self, name: str) -> None:
        _, width, height, found = load(name)
        self.assertAlmostEqual(width, A3_LANDSCAPE_PT[0], delta=TOL_PT)
        self.assertAlmostEqual(height, A3_LANDSCAPE_PT[1], delta=TOL_PT)
        sides = square_sides(found)
        self.assertEqual(len(sides), 1, f"{name}: squares found {sides}")
        self.assertAlmostEqual(sides[0][0], SQUARE_PT, delta=TOL_PT)
        self.assertAlmostEqual(sides[0][1], SQUARE_PT, delta=TOL_PT)

    def test_direct_square_is_50_mm(self) -> None:
        self.assert_true_scale("direct")

    def test_svg2pdf_square_is_50_mm_at_css_dpi(self) -> None:
        self.assert_true_scale("svg2pdf")

    def test_svg2pdf_default_dpi_prints_four_thirds_too_large(self) -> None:
        """Prevents trusting svg2pdf's default PageOptions (72 dpi) with mm SVGs."""
        sides = square_sides(load("svg2pdf_dpi72")[3])
        self.assertEqual(len(sides), 1)
        self.assertAlmostEqual(sides[0][0], SQUARE_PT * 96.0 / 72.0, delta=TOL_PT)


class Labels(unittest.TestCase):
    """Prevents a page that silently loses its piece labels."""

    def test_direct_has_labels(self) -> None:
        text = load("direct")[0].pages[0].extract_text()
        for label in ("A3", "Body side", "50 mm"):
            self.assertIn(label, text)

    def test_svg2pdf_without_fonts_drops_labels(self) -> None:
        """Records the gap: no font in usvg's fontdb → labels vanish, no error."""
        self.assertEqual(load("svg2pdf")[0].pages[0].extract_text().strip(), "")


if __name__ == "__main__":
    unittest.main()
