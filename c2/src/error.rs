use thiserror::Error;

#[derive(Error, Debug)]
pub enum C2Error {
    #[error("rusqlire error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("serde_json error: {0}")]
    SerdeJson(#[from] serde_json::error::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type C2Result<T> = Result<T, C2Error>;
