use common::{client, crypto};

#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    #[error("Crypto error: {0}")]
    Crypto(#[from] crypto::CryptoError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Client error: {0}")]
    Client(#[from] client::ClientError),
}
