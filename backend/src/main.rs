use axum::{
    Router,
    http::{Method, header},
    routing::get,
};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;

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

    let cors = CorsLayer::new()
        .allow_origin(axum::http::HeaderValue::from_static(
            "http://localhost:5173",
        ))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(spotify_rankings::routes::get_router())
        .with_state(spotify_rankings::AppState {
            client,
            client_id,
            client_secret,
            redirect_uri,
            pool,
        })
        .layer(cors);

    tracing::info!("Starting server...");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to :3000");

    tracing::info!("Server running at http://127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
