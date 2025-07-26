use axum::Router;

use crate::AppState;

pub mod auth;
pub mod playlists;

pub fn get_router() -> Router<AppState> {
    Router::new()
        .merge(auth::get_router())
        .merge(playlists::get_router())
}
