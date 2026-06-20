use app::App;
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, file_and_error_handler, generate_route_list};

/// Shell function for error/fallback rendering — wraps App in HTML document.
fn shell(_options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>"Emberwake"</title>
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[tokio::main]
async fn main() {
    tracing::info!("Starting Emberwake server");

    let options = LeptosOptions::builder()
        .output_name("emberwake")
        .site_addr(
            "0.0.0.0:5005"
                .parse::<std::net::SocketAddr>()
                .expect("valid addr"),
        )
        .build();

    let routes = generate_route_list(App);

    let app = axum::Router::new()
        .leptos_routes(&options, routes, App)
        .fallback(file_and_error_handler(shell))
        .with_state(options);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5005")
        .await
        .expect("failed to bind");

    tracing::info!("Listening on http://0.0.0.0:5005");
    axum::serve(listener, app).await.expect("server error");
}
