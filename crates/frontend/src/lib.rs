use app::App;
use leptos::mount::hydrate_body;
use wasm_bindgen::prelude::*;

/// WASM hydrate entry point — called by the Leptos hydration script.
/// The hydration script calls `mod.default(...)` to init WASM, then `mod.hydrate()`.
#[wasm_bindgen]
pub fn hydrate() {
    hydrate_body(App);
}
