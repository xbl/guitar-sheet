use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sql: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("http: {0}")]
    Http(String),
    #[error("bad input: {0}")]
    BadInput(String),
    #[error("keyring: {0}")]
    Keyring(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct UserFacingError {
    pub code: &'static str,
    pub message: String,
}

impl From<AppError> for UserFacingError {
    fn from(value: AppError) -> Self {
        UserFacingError {
            code: "app_error",
            message: value.to_string(),
        }
    }
}
