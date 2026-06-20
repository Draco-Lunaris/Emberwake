use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

/// Root application component — renders the full HTML document shell.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>"Emberwake"</title>
            </head>
            <body>
                <Router>
                    <main>
                        <Routes fallback=|| "Not found.">
                            <Route path=path!("/") view=HomePage />
                        </Routes>
                    </main>
                </Router>
            </body>
        </html>
    }
}

/// Home page — placeholder for Phase 1.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"Emberwake"</h1>
        <p>"Hello from Emberwake!"</p>
    }
}
