use axum::{
    extract::{FromRequestParts, State},
    http::StatusCode,
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SpotifyResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct SpotifyProfile {
    id: String,
}

pub struct Spotify {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<Utc>,
    pub spotify_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Playlist {
    pub href: String,
    pub id: String,
    pub name: String,
    pub image_url: String,
}

#[derive(Debug, Deserialize)]
struct Image {
    url: String,
}

#[derive(Debug, Deserialize)]
struct PlaylistResponse {
    href: String,
    id: String,
    name: String,
    images: Vec<Image>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Track {
    pub href: String,
    pub id: String,
    pub name: String,
    pub artists: Vec<Artist>,
    pub image_url: String,
}

#[derive(Debug, Deserialize)]
struct PlaylistTrackObject {
    track: TrackResponse,
}

#[derive(Debug, Deserialize)]
struct TracksResponse {
    tracks: Vec<TrackResponse>,
}

#[derive(Debug, Deserialize)]
struct TrackResponse {
    href: String,
    id: String,
    name: String,
    artists: Vec<Artist>,
    album: Album,
}

#[derive(Debug, Deserialize)]
struct Album {
    images: Vec<Image>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Artist {
    pub name: String,
    pub href: String,
}

#[derive(Debug)]
pub enum SpotifyError {
    InvalidToken,
    BadOauthRequest,
    RateLimited,
    Other(StatusCode),
}

impl Spotify {
    pub async fn from_response(
        response: SpotifyResponse,
        state: &AppState,
    ) -> Result<Self, StatusCode> {
        let expires_at = Utc::now() + chrono::Duration::seconds(response.expires_in as i64);

        // Get id
        let spotify_id = state
            .client
            .get("https://api.spotify.com/v1/me")
            .bearer_auth(&response.access_token)
            .send()
            .await
            .map_err(|err| {
                tracing::error!("Failed to fetch Spotify profile: {}", err);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .json::<SpotifyProfile>()
            .await
            .map_err(|err| {
                tracing::error!("Failed to parse Spotify profile: {}", err);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .id;

        // Insert into database, if id already exists, update the tokens
        sqlx::query!(
            "INSERT INTO users (spotify_id, access_token, refresh_token, expires_at) VALUES ($1, $2, $3, $4)
             ON CONFLICT (spotify_id)
             DO UPDATE SET access_token = EXCLUDED.access_token, refresh_token = EXCLUDED.refresh_token, expires_at = EXCLUDED.expires_at",
            spotify_id,
            response.access_token,
            response.refresh_token,
            expires_at
        )
        .execute(&state.pool)
        .await
        .map_err(|err| {
            tracing::error!("Failed to insert/update user: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        Ok(Self {
            access_token: response.access_token,
            refresh_token: response.refresh_token.unwrap(),
            expires_at,
            spotify_id,
        })
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    pub async fn refresh(&mut self, state: &AppState) -> Result<(), StatusCode> {
        if !self.is_expired() {
            return Ok(());
        }

        let response: SpotifyResponse = state
            .client
            .post("https://accounts.spotify.com/api/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .basic_auth(&state.client_id, Some(&state.client_secret))
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &self.refresh_token),
            ])
            .send()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .json()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        self.access_token = response.access_token.clone();
        let refresh_token_to_store = if let Some(refresh_token) = response.refresh_token {
            self.refresh_token = refresh_token.clone();
            Some(refresh_token)
        } else {
            Some(self.refresh_token.clone())
        };
        self.expires_at = Utc::now() + chrono::Duration::seconds(response.expires_in as i64);

        sqlx::query!(
            "UPDATE users SET access_token = $1, refresh_token = $2, expires_at = NOW() + INTERVAL '1 second' * $3 WHERE spotify_id = $4",
            response.access_token,
            refresh_token_to_store,
            response.expires_in as i64,
            self.spotify_id
        )
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(())
    }

    // GET function with token refresh
    async fn get<T: serde::de::DeserializeOwned>(
        &mut self,
        state: &AppState,
        url: &str,
    ) -> Result<T, SpotifyError> {
        if self.is_expired() {
            self.refresh(state).await.map_err(|e| {
                tracing::error!("Failed to refresh Spotify token: {:#?}", e);
                SpotifyError::BadOauthRequest
            })?;
        }

        let response = state
            .client
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to send Spotify API request: {:#?}", e);
                SpotifyError::Other(StatusCode::INTERNAL_SERVER_ERROR)
            })?
            .error_for_status()
            .map_err(|e| {
                tracing::error!("Spotify API request failed: {:#?}", e);
                if e.status() == Some(StatusCode::UNAUTHORIZED) {
                    SpotifyError::InvalidToken
                } else if e.status() == Some(StatusCode::TOO_MANY_REQUESTS) {
                    SpotifyError::RateLimited
                } else {
                    SpotifyError::Other(e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
                }
            })?
            .json::<T>()
            .await
            .map_err(|e| {
                tracing::error!("Failed to parse Spotify API response: {:#?}", e);
                SpotifyError::Other(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        Ok(response)
    }

    pub async fn get_playlists(&mut self, state: &AppState) -> Result<Vec<Playlist>, SpotifyError> {
        let mut playlists = Vec::new();
        let mut next_url = Some("https://api.spotify.com/v1/me/playlists".to_string());
        while let Some(url) = next_url {
            let response: PaginatedResponse<PlaylistResponse> = self.get(state, &url).await?;

            playlists.extend(response.items.into_iter().map(|item| {
                Playlist {
                    href: item.href,
                    id: item.id,
                    name: item.name,
                    image_url: item
                        .images
                        .into_iter()
                        .next()
                        .map(|img| img.url)
                        .unwrap_or_default(),
                }
            }));
            next_url = response.next;
        }
        Ok(playlists)
    }

    pub async fn get_playlist_tracks(
        &mut self,
        state: &AppState,
        playlist_id: &str,
    ) -> Result<Vec<Track>, SpotifyError> {
        let mut tracks = Vec::new();
        let mut next_url = Some(format!(
            "https://api.spotify.com/v1/playlists/{}/tracks",
            playlist_id
        ));
        while let Some(url) = next_url {
            let response: PaginatedResponse<PlaylistTrackObject> = self.get(state, &url).await?;

            tracks.extend(response.items.into_iter().map(|item| {
                Track {
                    href: item.track.href,
                    id: item.track.id,
                    name: item.track.name,
                    artists: item.track.artists,
                    image_url: item
                        .track
                        .album
                        .images
                        .into_iter()
                        .next()
                        .map_or_else(String::new, |img| img.url),
                }
            }));
            next_url = response.next;
        }
        Ok(tracks)
    }

    pub async fn get_tracks(
        &mut self,
        state: &AppState,
        track_ids: &[String],
    ) -> Result<Vec<Track>, SpotifyError> {
        if track_ids.is_empty() {
            return Ok(Vec::new());
        }

        let ids = track_ids.join(",");
        let url = format!("https://api.spotify.com/v1/tracks?ids={}", ids);

        let response: TracksResponse = self.get(state, &url).await?;

        Ok(response
            .tracks
            .into_iter()
            .map(|track| Track {
                href: track.href,
                id: track.id,
                name: track.name,
                artists: track.artists,
                image_url: track
                    .album
                    .images
                    .into_iter()
                    .next()
                    .map_or_else(String::new, |img| img.url),
            })
            .collect())
    }
}

impl<S> FromRequestParts<S> for Spotify
where
    S: Send + Sync,
    AppState: axum::extract::FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let State(app_state): State<AppState> =
            State::from_request_parts(parts, state).await.unwrap();

        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| (StatusCode::BAD_REQUEST, "Failed to extract session cookie"))?;

        let session_token = jar
            .get("session_token")
            .ok_or((StatusCode::BAD_REQUEST, "Session token not found"))?
            .value();

        sqlx::query_as!(
            Spotify,
            "SELECT u.spotify_id, u.access_token, u.refresh_token, u.expires_at
             FROM users u
             JOIN sessions s ON u.id = s.user_id
             WHERE s.token = $1",
            session_token
        )
        .fetch_one(&app_state.pool)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token"))
    }
}

#[derive(Debug, Deserialize)]
struct PaginatedResponse<T> {
    items: Vec<T>,
    next: Option<String>,
}
