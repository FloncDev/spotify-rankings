use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use rand::prelude::*;
use rand_distr::weighted::WeightedIndex;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use skillratings::{
    Outcomes,
    glicko2::{Glicko2Rating, glicko2},
};

use crate::{
    AppState,
    routes::playlists::{RatedTrack, Song},
    spotify::Spotify,
};

#[derive(Debug, Serialize)]
struct Match {
    song_a: RatedTrack,
    song_b: RatedTrack,
}

#[derive(Debug, Deserialize)]
struct MatchResult {
    song_a: String,
    song_b: String,
    winner: String,
}

const EPSILON: f64 = 0.0001;

async fn matchmaking(
    State(state): State<AppState>,
    Path(playlist_id): Path<String>,
    mut spotify: Spotify,
) -> Result<Json<Match>, StatusCode> {
    let songs = sqlx::query_as!(
        Song,
        "SELECT id, song_id, playlist_id, rating, deviation, volatility, total_matches FROM songs WHERE playlist_id = $1 ORDER BY rating DESC",
        playlist_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if songs.len() < 2 {
        return Err(StatusCode::BAD_REQUEST); // Not enough songs to make a match
    }

    // Do all random operations first to avoid Send issues
    let (song_a_idx, song_b_idx) = {
        let mut rng = rand::rng();

        // For now, song A will just be a random song from the playlist
        // TODO: Pick song based on total_matches and RD
        let song_a_idx = rng.random_range(0..songs.len());
        let song_a = &songs[song_a_idx];

        // For song B, a weight will need to be calculated based on the rating and deviation
        // Songs close to song A's rating will have a higher chance of being selected
        let mut weights = Vec::new();
        let mut candidates = Vec::new();

        for (idx, song) in songs.iter().enumerate() {
            if idx == song_a_idx {
                continue; // Skip the same song
            }

            let dist = (song.rating - song_a.rating).powi(2);
            let weight = EPSILON + (-dist / (2.0 * song_a.deviation.powi(2))).exp();
            weights.push(weight);
            candidates.push(idx);
        }

        // Print all songs and weights for debugging
        for (weight, candidate) in weights.iter().zip(&candidates) {
            tracing::info!("Weight for song {}: {}", songs[*candidate].song_id, weight);
        }

        let dist = WeightedIndex::new(&weights).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let candidate_idx = dist.sample(&mut rng);
        let song_b_idx = candidates[candidate_idx];

        (song_a_idx, song_b_idx)
    };

    let song_a = &songs[song_a_idx];
    let song_b = &songs[song_b_idx];

    tracing::info!(
        "Selected songs: A({}) and B({})",
        song_a.song_id,
        song_b.song_id,
    );

    let songs = spotify
        .get_tracks(&state, &[song_a.song_id.clone(), song_b.song_id.clone()])
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let rated_song_a = RatedTrack::from_track(&songs[0], song_a);
    let rated_song_b = RatedTrack::from_track(&songs[1], song_b);

    Ok(Json(Match {
        song_a: rated_song_a,
        song_b: rated_song_b,
    }))
}

async fn matchmaking_result(
    State(state): State<AppState>,
    _: Spotify,
    Json(result): Json<MatchResult>,
) -> Result<(), StatusCode> {
    // Update the ratings based on the result
    tracing::info!(
        "Match result: A({}) vs B({}), winner: {}",
        result.song_a,
        result.song_b,
        result.winner
    );

    // Create glicko2 players for both songs
    let player_a = sqlx::query_as!(
        Glicko2Rating,
        "SELECT rating, deviation, volatility FROM songs WHERE song_id = $1",
        result.song_a
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let player_b = sqlx::query_as!(
        Glicko2Rating,
        "SELECT rating, deviation, volatility FROM songs WHERE song_id = $1",
        result.song_b
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let outcome = if result.winner == result.song_a {
        Outcomes::WIN
    } else if result.winner == result.song_b {
        Outcomes::LOSS
    } else {
        return Err(StatusCode::BAD_REQUEST); // Invalid winner
    };

    let config = skillratings::glicko2::Glicko2Config::default();
    let (new_player_a, new_player_b) = glicko2(&player_a, &player_b, &outcome, &config);

    // Update the database with the new ratings
    sqlx::query!(
        "UPDATE songs SET rating = $1, deviation = $2, volatility = $3, total_matches = total_matches + 1 WHERE song_id = $4",
        new_player_a.rating,
        new_player_a.deviation,
        new_player_a.volatility,
        result.song_a
    )
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query!(
        "UPDATE songs SET rating = $1, deviation = $2, volatility = $3, total_matches = total_matches + 1 WHERE song_id = $4",
        new_player_b.rating,
        new_player_b.deviation,
        new_player_b.volatility,
        result.song_b
    )
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

pub fn get_router() -> Router<AppState> {
    Router::new()
        .route("/playlists/{playlist_id}/matchmaking", get(matchmaking))
        .route(
            "/playlists/{playlist_id}/matchmaking",
            axum::routing::post(matchmaking_result),
        )
}
