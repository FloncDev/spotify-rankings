use axum::{Json, Router, extract::State, http::StatusCode, routing::get};

use crate::{
    AppState,
    spotify::{Playlist, Spotify},
};

async fn get_playlists(
    State(state): State<AppState>,
    mut spotify: Spotify,
) -> Result<Json<Vec<Playlist>>, StatusCode> {
    let playlists = spotify
        .get_playlists(&state)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok(Json(playlists))
}

pub fn get_router() -> Router<AppState> {
    Router::new().route("/playlists", get(get_playlists))
}
