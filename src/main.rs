use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    tracing::info!("Starting server...");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to :3000");

    tracing::info!("Server running at http://127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
