//! Probe: what a Rust panic looks like from JS on wasm32-unknown-unknown.

use std::cell::RefCell;
use std::panic;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(message: &str);
}

thread_local! {
    static STATE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Installs a hook that forwards the panic message to the console before the abort.
#[wasm_bindgen(start)]
pub fn start() {
    panic::set_hook(Box::new(|info| console_error(&format!("probe hook: {info}"))));
}

/// Returns a value; proves the instance works.
#[wasm_bindgen]
pub fn ok() -> u32 {
    STATE.with(|s| s.borrow().len() as u32)
}

/// Panics while a `RefCell` is mutably borrowed.
#[wasm_bindgen]
#[allow(clippy::panic)]
pub fn boom_while_borrowed() {
    STATE.with(|s| {
        let mut v = s.borrow_mut();
        v.push(1);
        panic!("deliberate panic while borrowed");
    });
}

/// Panics inside `catch_unwind`; reports whether it was caught.
#[wasm_bindgen]
#[allow(clippy::panic)]
pub fn caught() -> bool {
    panic::catch_unwind(|| panic!("inside catch_unwind")).is_err()
}

/// Requests an allocation that cannot succeed.
#[wasm_bindgen]
pub fn oom() -> usize {
    let mut blocks: Vec<Vec<u8>> = Vec::new();
    loop {
        blocks.push(vec![1; 256 << 20]);
        if blocks.len() > 64 {
            return blocks.len();
        }
    }
}

/// Recurses without bound.
#[wasm_bindgen]
pub fn recurse(n: u32) -> u32 {
    if n == u32::MAX { 0 } else { recurse(n + 1).wrapping_add(std::hint::black_box(1)) }
}
