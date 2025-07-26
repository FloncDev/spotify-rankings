use axum::{Router, routing::get};
use sqlx::postgres::PgPoolOptions;

macro_rules! var {
    ($key:expr) => {
        dotenvy::var($key).expect(&format!("Environment variable {} not set", $key))
    };
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    dotenvy::dotenv().expect("Failed to load .env file");

    let client = reqwest::Client::new();
    let client_id = var!("SPOTIFY_CLIENT_ID");
    let client_secret = var!("SPOTIFY_CLIENT_SECRET");
    let redirect_uri = var!("SPOTIFY_REDIRECT_URI");

    let pool = PgPoolOptions::new()
        .connect(&var!("DATABASE_URL"))
        .await
        .expect("Failed to connect to the database");

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(spotify_rankings::routes::get_router())
        .with_state(spotify_rankings::AppState {
            client,
            client_id,
            client_secret,
            redirect_uri,
            pool,
        });

    tracing::info!("Starting server...");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to :3000");

    tracing::info!("Server running at http://127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
