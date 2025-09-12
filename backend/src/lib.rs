pub mod error;
pub mod routes;
pub mod spotify;

#[derive(Clone)]
pub struct AppState {
    pub client: reqwest::Client,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub pool: sqlx::Pool<sqlx::Postgres>,
}
