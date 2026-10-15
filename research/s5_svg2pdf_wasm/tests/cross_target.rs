//! Same tests natively (`#[test]`) and on wasm32 under Node (`#[wasm_bindgen_test]`).
//! The hashes are what the native release build printed; a pass on wasm32 means
//! the PDF bytes are identical to native.

use s5_svg2pdf_wasm::{fnv1a, page_direct, page_svg2pdf};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const DIRECT_HASH: u64 = 2_578_343_843_372_603_017;
const SVG2PDF_HASH: u64 = 11_445_119_838_395_906_974;

/// Keys that would carry a clock or a random/derived file identity.
const NONDETERMINISTIC_KEYS: [&[u8]; 3] = [b"/CreationDate", b"/ModDate", b"/ID"];

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn assert_no_clock_or_id(pdf: &[u8]) {
    for key in NONDETERMINISTIC_KEYS {
        assert!(!contains(pdf, key), "{}", String::from_utf8_lossy(key));
    }
}

#[test]
fn direct_is_repeatable_and_matches_native() {
    let pdf = page_direct();
    assert_eq!(pdf, page_direct());
    assert_no_clock_or_id(&pdf);
    assert_eq!(fnv1a(&pdf), DIRECT_HASH);
}

#[test]
fn svg2pdf_is_repeatable_and_matches_native() {
    let pdf = page_svg2pdf().expect("svg2pdf converts the page");
    assert_eq!(pdf, page_svg2pdf().expect("svg2pdf converts the page"));
    assert_no_clock_or_id(&pdf);
    assert_eq!(fnv1a(&pdf), SVG2PDF_HASH);
}

/// The whole mm → pt mapping is this one `cm`, and the square is drawn in mm
/// under it: 50 × 2.8346457 = 141.732 pt.
#[test]
fn direct_square_is_drawn_in_mm_under_one_transform() {
    let pdf = page_direct();
    assert!(contains(&pdf, b"q\n2.8346457 0 0 -2.8346457 0 841.8898 cm\n"));
    assert!(contains(&pdf, b"\n10 237 50 50 re\n"));
    assert!(contains(&pdf, b"/BaseFont /Helvetica"));
    assert!(!contains(&pdf, b"/FontFile"), "no font program embedded");
}
