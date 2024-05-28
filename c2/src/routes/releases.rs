use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use common::model::Release;
use tracing::error;

use crate::{services, C2State};

use super::RELEASE_NOT_FOUND;

pub async fn create(
    State(c2_state): State<C2State>,
    Json(release): Json<Release>,
) -> impl IntoResponse {
    match services::releases::create(
        c2_state.conn.clone(),
        &release.checksum,
        release.platform,
        &release.bytes,
    ) {
        Ok(release) => (StatusCode::CREATED, Json(release)).into_response(),
        Err(e) => {
            error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn get(State(c2_state): State<C2State>) -> impl IntoResponse {
    match services::releases::get(c2_state.conn.clone()) {
        Ok(releases) => (StatusCode::OK, Json(releases)).into_response(),
        Err(e) => {
            error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn get_by_checksum(
    State(c2_state): State<C2State>,
    Path(release_checksum): Path<String>,
) -> impl IntoResponse {
    match services::releases::get_by_checksum(c2_state.conn.clone(), &release_checksum) {
        Ok(Some(release)) => (StatusCode::OK, Json(release)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, RELEASE_NOT_FOUND).into_response(),
        Err(e) => {
            error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}
