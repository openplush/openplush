//! Size probe: links only the direct `pdf-writer` path.

#[unsafe(no_mangle)]
pub extern "C" fn probe() -> usize {
    s5_svg2pdf_wasm::page_direct().len()
}
