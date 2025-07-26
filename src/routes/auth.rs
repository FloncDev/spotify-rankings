use axum::{
    Router,
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::get,
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{
    AppState,
    spotify::{Spotify, SpotifyResponse},
};

#[derive(Debug, Deserialize)]
struct Callback {
    code: String,
}

async fn login(State(state): State<AppState>) -> impl IntoResponse {
    Redirect::to(&format!(
        "https://accounts.spotify.com/authorize?response_type=code&client_id={}&redirect_uri={}&scope=playlist-read-private,playlist-read-collaborative,streaming",
        state.client_id, state.redirect_uri
    ))
}

async fn callback(
    State(state): State<AppState>,
    Query(params): Query<Callback>,
) -> Result<CookieJar, (StatusCode, String)> {
    tracing::info!("Received callback with code: {}", params.code);

    let response = state
        .client
        .post("https://accounts.spotify.com/api/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .basic_auth(&state.client_id, Some(&state.client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &params.code),
            ("redirect_uri", &state.redirect_uri),
        ])
        .send()
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to send request".into(),
            )
        })?
        .json::<SpotifyResponse>()
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse response".into(),
            )
        })?;

    let spotify = Spotify::from_response(response, &state)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create Spotify client".into(),
            )
        })?;

    let token = rand::random::<u64>().to_string();

    // Get the id from the database
    let user_id = sqlx::query_scalar!(
        "SELECT id FROM users WHERE spotify_id = $1",
        spotify.spotify_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch user id".into(),
        )
    })?;

    // Create a new session in the database session table
    sqlx::query!(
        "INSERT INTO sessions (user_id, token, created_at) VALUES ($1, $2, NOW())",
        user_id,
        token
    )
    .execute(&state.pool)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create session".into(),
        )
    })?;

    let jar = CookieJar::new();
    let jar = jar.add(
        Cookie::build(("session_token", token))
            .secure(true)
            .http_only(true),
    );

    Ok(jar)
}

pub fn get_router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login))
        .route("/callback", get(callback))
}
