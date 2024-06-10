use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::error;

use crate::{routes::AGENT_NOT_FOUND, services, C2State};

// #[axum::debug_handler]
pub async fn get(State(c2_state): State<C2State>) -> impl IntoResponse {
    match services::agents::get(c2_state.conn.clone()) {
        Ok(agents) => (StatusCode::OK, Json(agents)).into_response(),
        Err(e) => {
            error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn update(
    State(c2_state): State<C2State>,
    Path(agent_id): Path<i32>,
    agent_json: String,
) -> impl IntoResponse {
    let agent_json: serde_json::Value = serde_json::from_str(&agent_json).unwrap();
    match services::agents::get_by_id(c2_state.conn.clone(), agent_id) {
        Ok(Some(agent)) => {
            match services::agents::update_by_id(c2_state.conn.clone(), &agent.merge(agent_json)) {
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

pub async fn delete(
    State(c2_state): State<C2State>,
    Path(agent_id): Path<i32>,
) -> impl IntoResponse {
    match services::agents::delete_by_id(c2_state.conn.clone(), agent_id) {
        Ok(true) => (StatusCode::OK).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, AGENT_NOT_FOUND).into_response(),
        Err(e) => {
            error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}
