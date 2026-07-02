use thiserror::Error;

pub(crate) type NativeResult<T> = Result<T, NativeError>;

#[derive(Debug, Error)]
pub(crate) enum NativeError {
    #[error("schema error: {0}")]
    Schema(String),
    #[error("index open error: {0}")]
    Open(String),
    #[error("write error: {0}")]
    Write(String),
    #[error("search error: {0}")]
    Search(String),
    #[error("invalid native handle: {0}")]
    InvalidHandle(i64),
    #[error("native panic")]
    Panic,
    #[error("native state error: {0}")]
    State(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Tantivy(#[from] tantivy::TantivyError),
}
