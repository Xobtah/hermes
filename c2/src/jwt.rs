use serde::{Deserialize, Serialize};

use crate::error::C2Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claim {
    exp: i64,
}

impl Claim {
    pub fn new(timeout_in_minutes: i64) -> Self {
        Self {
            exp: (chrono::Utc::now() + chrono::Duration::minutes(timeout_in_minutes)).timestamp(),
        }
    }

    pub fn expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.exp
    }

    pub fn from_jwt(token: &str, signing_key: &[u8]) -> C2Result<Self> {
        Ok(jsonwebtoken::decode::<Claim>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(signing_key),
            &jsonwebtoken::Validation::default(),
        )
        .map(|data| data.claims)?)
    }

    pub fn into_jwt(self, signing_key: &[u8]) -> C2Result<String> {
        Ok(jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &self,
            &jsonwebtoken::EncodingKey::from_secret(signing_key),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim() {
        let claim = Claim::new(1);
        let jwt = claim.into_jwt(b"secret").unwrap();
        let claim = Claim::from_jwt(&jwt, b"secret").unwrap();
        assert!(!claim.expired());
    }

    #[test]
    fn test_expired_claim() {
        let claim = Claim::new(-1);
        let jwt = claim.into_jwt(b"secret").unwrap();
        let claim = Claim::from_jwt(&jwt, b"secret").unwrap();
        assert!(claim.expired());
    }

    #[test]
    fn test_invalid_claim() {
        let claim = Claim::new(1);
        let jwt = claim.into_jwt(b"secret").unwrap();
        match Claim::from_jwt(&jwt, b"wrong").unwrap_err() {
            crate::error::C2Error::JsonWebToken(_) => (),
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_invalid_jwt() {
        match Claim::from_jwt("invalid", b"secret").unwrap_err() {
            crate::error::C2Error::JsonWebToken(_) => (),
            _ => panic!("Wrong error type"),
        }
    }
}
