use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use common::model;
use tracing::{error, warn};

use crate::{jwt::Claim, services, C2State};

mod agents;
mod missions;

pub const AGENT_NOT_FOUND: &str = "Agent not found.";
pub const MISSION_NOT_FOUND: &str = "Mission not found.";

// TODO Try to propagate errors using impl IntoResponse for better error handling

// TODO HTTPS
pub fn init_router(state: C2State) -> Router<C2State> {
    let logged_router = Router::new()
        // Agents
        .nest(
            "/agents",
            Router::new().route("/", get(agents::get)).nest(
                "/:agent_id",
                Router::new().route("/", put(agents::update).delete(agents::delete)),
            ),
        )
        // Missions
        .nest(
            "/missions",
            Router::new()
                .route("/", post(missions::create))
                .route("/:mission_id", get(missions::get_report)),
        )
        .layer(middleware::from_fn_with_state(state.clone(), is_admin));

    let not_logged_router = Router::new()
        // Admin login
        .route("/", get(admin_login))
        // Crypto
        .route("/crypto/:mission_id", get(get_crypto))
        // Missions
        .nest(
            "/missions",
            Router::new()
                .route("/", get(missions::get_next))
                .route("/:mission_id", put(missions::report)),
        );

    logged_router.merge(not_logged_router)
}

async fn is_admin(
    State(state): State<C2State>,
    header: axum::http::HeaderMap,
    request: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    match header.get("Authorization") {
        Some(header) => {
            let jwt = &header.to_str().unwrap()[7..]; // Bearer
            let claim = match Claim::from_jwt(jwt, state.signing_key.as_bytes()) {
                Ok(claim) => claim,
                Err(e) => {
                    error!("{e}");
                    return (StatusCode::UNAUTHORIZED).into_response();
                }
            };

            if claim.expired() {
                error!("JWT is expired");
                return (StatusCode::UNAUTHORIZED).into_response();
            }
        }
        None => return (StatusCode::UNAUTHORIZED).into_response(),
    }

    next.run(request).await
}

async fn admin_login(
    State(state): State<C2State>,
    crypto_negociation: Json<model::CryptoNegociation>,
) -> impl IntoResponse {
    // This only checks that the admin has access to the signing key to which
    // the server also has access, to make sure requests are sent from
    // localhost. On a security perspective, I don't think it's good. It's just
    // a personnal project for fun though.
    if crypto_negociation.identity != state.signing_key.verifying_key() {
        error!("Failed to authenticate admin");
        return (StatusCode::UNAUTHORIZED).into_response();
    }

    if let Err(e) = crypto_negociation.verify() {
        error!("{e}");
        return (StatusCode::UNAUTHORIZED).into_response();
    }

    match Claim::new(1).into_jwt(state.signing_key.as_bytes()) {
        Ok(jwt) => {
            tracing::info!("Admin logged in");
            (StatusCode::OK, Json(jwt)).into_response()
        }
        Err(e) => {
            error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
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
