use std::{
    fmt::{self, Display},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::crypto;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agent {
    pub id: i32,
    pub name: String,
    pub identity: crypto::VerifyingKey,
    pub platform: Platform,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "lastSeenAt")]
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
}

impl Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let timeago = timeago::Formatter::new();
        write!(
            f,
            "Agent [{}]: {} ({}) {}",
            self.id,
            self.name,
            self.platform,
            timeago.convert_chrono(self.last_seen_at, chrono::offset::Utc::now())
        )
    }
}

impl Agent {
    pub fn merge(self, value: serde_json::Value) -> Self {
        Agent {
            id: value
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(self.id as i64) as i32,
            name: value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&self.name)
                .to_owned(),
            identity: value
                .get("identity")
                .and_then(|v| serde_json::from_value(v.clone()).ok()) // TODO Errors are hidden
                .unwrap_or(self.identity),
            platform: value
                .get("platform")
                .and_then(serde_json::Value::as_str)
                .and_then(|s| Platform::from_str(s).ok()) // TODO Errors are hidden
                .unwrap_or(self.platform),
            created_at: self.created_at,
            last_seen_at: self.last_seen_at,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum Platform {
    #[default]
    Unix,
    Windows,
}

impl Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Platform::Unix => write!(f, "Unix"),
            Platform::Windows => write!(f, "Windows"),
        }
    }
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Unix" => Ok(Platform::Unix),
            "Windows" => Ok(Platform::Windows),
            _ => Err(format!("Invalid platform '{s}'")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Release {
    pub checksum: String,
    pub verifying_key: crypto::VerifyingKey,
    pub bytes: Vec<u8>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mission {
    #[serde(default)]
    pub id: i32,
    #[serde(rename = "agentId")]
    pub agent_id: i32,
    pub task: Task,
    pub result: Option<String>,
    #[serde(default)]
    pub issued_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl fmt::Display for Mission {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Mission [{}]: {:?}",
            self.id,
            match &self.task {
                Task::Update(release) => format!("Update {} compressed bytes", release.bytes.len()),
                Task::Execute(cmd) => format!("Execute '{cmd}'"),
                Task::Stop => "Stop".to_owned(),
            }
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Task {
    Update(Box<Release>),
    Execute(String),
    Stop,
}

#[derive(Serialize, Deserialize)]
pub struct CryptoNegociation {
    pub identity: crypto::VerifyingKey,
    #[serde(rename = "publicKey")]
    pub public_key: crypto::KeyExchangePublicKey,
    pub signature: crypto::Signature,
}

impl CryptoNegociation {
    pub fn new(signing_key: &mut crypto::SigningKey) -> (crypto::KeyExchangePrivateKey, Self) {
        let (public_key, private_key, signature) =
            crypto::generate_key_exchange_key_pair(signing_key);
        (
            private_key,
            Self {
                identity: signing_key.verifying_key(),
                public_key,
                signature,
            },
        )
    }

    pub fn verify(&self) -> Result<(), crypto::CryptoError> {
        crypto::verify_key_exchange_key_pair(&self.identity, self.public_key, self.signature)
    }
}

#[derive(Serialize, Deserialize)]
pub struct CryptoMessage {
    #[serde(rename = "publicKey")]
    pub public_key: crypto::KeyExchangePublicKey,
    pub nonce: crypto::Nonce,
    #[serde(rename = "encryptedData")]
    pub encrypted_data: Vec<u8>,
    pub signature: crypto::Signature,
}

impl CryptoMessage {
    pub fn new(
        signing_key: &mut crypto::SigningKey,
        public_key: crypto::KeyExchangePublicKey,
        plain_data: &[u8],
    ) -> crypto::CryptoResult<Self> {
        let (public_key, nonce, encrypted_data) = crypto::encrypt(public_key, plain_data)?;
        let signature = crypto::sign(signing_key, &[], public_key, &encrypted_data, nonce);
        Ok(Self {
            public_key,
            nonce,
            encrypted_data,
            signature,
        })
    }

    pub fn verify(&self, verifying_key: &crypto::VerifyingKey) -> Result<(), crypto::CryptoError> {
        crypto::verify(
            verifying_key,
            self.signature,
            &[],
            self.public_key,
            &self.encrypted_data,
            self.nonce,
        )
    }

    pub fn decrypt(
        &self,
        private_key: crypto::KeyExchangePrivateKey,
    ) -> crypto::CryptoResult<Vec<u8>> {
        crypto::decrypt(
            &self.encrypted_data,
            self.public_key,
            private_key,
            self.nonce,
        )
    }
}
