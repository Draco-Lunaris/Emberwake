use app::App;
use leptos::mount::hydrate_body;

/// WASM hydrate entry point — called by cargo-leptos.
pub fn main() {
    hydrate_body(App);
}
