//! The `wasm-bindgen` facade the web worker hosts: the worker protocol's operations
//! over `openplush-pipeline`, and nothing else from the engine.

pub mod protocol;

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(message: &str);
}

/// Forwards a panic's message to the worker console; the default hook's stderr goes nowhere here.
#[wasm_bindgen(start)]
fn start() {
    std::panic::set_hook(Box::new(|info| console_error(&info.to_string())));
}
