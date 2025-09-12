#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    DatabaseError(#[from] sqlx::Error),
}
