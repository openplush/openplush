//! Writes both PDFs (and the default-dpi svg2pdf one) to `target/pdfs/` for the
//! Python scale oracle, and prints sizes, hashes, and native time per page.

use std::time::Instant;

use svg2pdf_wasm::{fnv1a, page_direct, page_svg2pdf, page_svg2pdf_at_dpi};

const RUNS: u32 = 200;

fn timed(name: &str, f: impl Fn() -> Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let pdf = f();
    let start = Instant::now();
    for _ in 0..RUNS {
        std::hint::black_box(f());
    }
    let per_page = start.elapsed() / RUNS;
    println!(
        "{name:<16} {:>7} B  hash {:>20}  {per_page:?}/page",
        pdf.len(),
        fnv1a(&pdf)
    );
    std::fs::write(format!("target/pdfs/{name}.pdf"), pdf)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("target/pdfs")?;
    timed("direct", page_direct)?;
    timed("svg2pdf", || page_svg2pdf().expect("svg2pdf"))?;
    timed("svg2pdf_dpi72", || {
        page_svg2pdf_at_dpi(72.0).expect("svg2pdf")
    })?;
    Ok(())
}
