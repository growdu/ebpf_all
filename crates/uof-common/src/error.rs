use thiserror::Error;

pub type Result<T> = std::result::Result<T, UofError>;

#[derive(Debug, Error)]
pub enum UofError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("internal error: {0}")]
    Internal(String),
}

