use common::client;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Dialoguer error: {0}")]
    Dialoguer(#[from] dialoguer::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ureq error: {0}")]
    Ureq(#[from] ureq::Error),
    #[error("Serde_json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Client error: {0}")]
    Client(#[from] client::ClientError),
}

pub type ClientResult<T> = Result<T, ClientError>;
