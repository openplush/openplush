//! Size probe: links only the SVG → `svg2pdf` path.

#[unsafe(no_mangle)]
pub extern "C" fn probe() -> usize {
    svg2pdf_wasm::page_svg2pdf().map_or(0, |pdf| pdf.len())
}
