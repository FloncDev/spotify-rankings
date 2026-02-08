use std::collections::HashSet;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::Serialize;

use crate::{
    AppState,
    spotify::{Artist, Playlist, Spotify, Track},
};

#[derive(Debug, Serialize)]
pub struct Song {
    pub id: i32,
    pub song_id: String,
    pub playlist_id: String,
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
    pub total_matches: i32,
}

#[derive(Debug, Serialize)]
pub struct RatedTrack {
    pub href: String,
    pub id: String,
    pub name: String,
    pub artists: Vec<Artist>,
    pub image_url: Option<String>,
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
    pub total_matches: i32,
}

impl RatedTrack {
    pub fn from_track(track: &Track, song: &Song) -> Self {
        Self {
            href: track.href.clone(),
            id: track.id.clone(),
            name: track.name.clone(),
            artists: track.artists.clone(),
            image_url: track.image_url.clone(),
            rating: song.rating,
            deviation: song.deviation,
            volatility: song.volatility,
            total_matches: song.total_matches,
        }
    }
}

async fn get_playlists(
    State(state): State<AppState>,
    mut spotify: Spotify,
) -> Result<Json<Vec<Playlist>>, StatusCode> {
    let playlists = spotify.get_playlists(&state).await.map_err(|e| {
        tracing::error!("Failed to fetch playlists: {:#?}", e);
        StatusCode::UNAUTHORIZED
    })?;
    Ok(Json(playlists))
}

// Check that a playlist is in database, and if not then add it
// This is basically starting a new "playlist rating session"
async fn check_playlist(
    Path(playlist_id): Path<String>,
    State(state): State<AppState>,
    mut spotify: Spotify,
) -> Result<(), StatusCode> {
    // let tracks = spotify
    //     .get_playlist_tracks(&state, &playlist_id)
    //     .await
    //     .map_err(|err| {
    //         tracing::error!("Failed to fetch playlist tracks: {:#?}", err);
    //         StatusCode::UNAUTHORIZED
    //     })?;
    // Ok(Json(tracks))

    // Get all current songs from database
    let songs = sqlx::query_as!(
        Song,
        "SELECT id, song_id, playlist_id, rating, deviation, volatility, total_matches FROM songs WHERE playlist_id = $1",
        playlist_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch songs: {:#?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let song_ids: HashSet<String> = HashSet::from_iter(songs.iter().map(|s| s.song_id.clone()));

    // Fetch songs from spotify and compare for any changes/insert all new songs
    let tracks = spotify
        .get_playlist_tracks(&state, &playlist_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let track_ids: HashSet<String> = HashSet::from_iter(tracks.iter().map(|t| t.id.clone()));

    let mut new_song_ids = Vec::new();

    for track in tracks {
        // Check if the song is already in the database
        if song_ids.contains(&track.id) {
            continue;
        }

        // If not, insert the new song into the database
        new_song_ids.push(track.id.clone());
    }

    let mut deleted_songs = Vec::new();

    // Check for any deleted songs
    for song in songs {
        if !track_ids.contains(&song.song_id) {
            deleted_songs.push(song.id);
        }
    }

    tracing::info!(
        "New songs: {:?}, Deleted songs: {:?}",
        new_song_ids,
        deleted_songs
    );

    // Update the database with new songs
    // Prepare a vector of playlist_ids matching the new_song_ids
    let playlist_ids: Vec<String> = vec![playlist_id.clone(); new_song_ids.len()];

    sqlx::query!(
        "INSERT INTO songs(song_id, playlist_id) SELECT * FROM UNNEST($1::text[], $2::text[]) ON CONFLICT DO NOTHING",
        &new_song_ids,
        &playlist_ids,
    )
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert new songs: {:#?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Remove deleted songs from the database
    sqlx::query!(
        "DELETE FROM songs WHERE id = ANY($1::integer[])",
        &deleted_songs
    )
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete songs: {:#?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}

pub async fn get_leaderboard(
    Path(playlist_id): Path<String>,
    State(state): State<AppState>,
    mut spotify: Spotify,
) -> Result<Json<Vec<RatedTrack>>, StatusCode> {
    let songs = sqlx::query_as!(
        Song,
        "SELECT id, song_id, playlist_id, rating, deviation, volatility, total_matches FROM songs WHERE playlist_id = $1 ORDER BY rating DESC LIMIT 10",
        playlist_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch leaderboard: {:#?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Fetch songs from spotify
    let spotify_tracks = spotify
        .get_playlist_tracks(&state, &playlist_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Map the songs to RatedTrack
    let songs: Vec<RatedTrack> = songs
        .into_iter()
        .filter_map(|song| {
            spotify_tracks
                .iter()
                .find(|t| t.id == song.song_id)
                .map(|track| RatedTrack::from_track(track, &song))
        })
        .collect();

    Ok(Json(songs))
}

pub fn get_router() -> Router<AppState> {
    Router::new()
        .route("/playlists", get(get_playlists))
        .route("/playlists/{playlist_id}", post(check_playlist))
        .route("/playlists/{playlist_id}/leaderboard", get(get_leaderboard))
}
