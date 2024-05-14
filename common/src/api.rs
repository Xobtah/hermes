use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Registration {
    pub identity: crate::crypto::VerifyingKey,
    #[serde(rename = "publicKey")]
    pub public_key: crate::crypto::KeyExchangePublicKey,
    pub signature: crate::crypto::Signature,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    #[serde(rename = "publicKey")]
    pub public_key: crate::crypto::KeyExchangePublicKey,
    pub nonce: crate::crypto::Nonce,
    pub encrypted_data: Vec<u8>,
    pub signature: crate::crypto::Signature,
}
