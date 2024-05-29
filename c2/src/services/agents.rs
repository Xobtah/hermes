use std::str::FromStr;

use common::{
    crypto,
    model::{self, Agent},
};
use rusqlite::OptionalExtension;
use tracing::debug;

use crate::{error::C2Result, ThreadSafeConnection};

use super::RusqliteResult;

fn row_to_agent(row: &rusqlite::Row) -> RusqliteResult<Agent> {
    Ok(Agent {
        id: row.get("id")?,
        name: row.get("name")?,
        identity: crypto::VerifyingKey::from_bytes(&row.get::<_, [u8; 32]>("identity")?).unwrap(),
        platform: model::Platform::from_str(&row.get::<_, String>("platform")?).unwrap(),
        created_at: row.get("created_at")?,
        last_seen_at: row.get("last_seen_at")?,
    })
}

pub fn create(
    conn: ThreadSafeConnection,
    name: &str,
    identity: crypto::VerifyingKey,
    platform: model::Platform,
) -> C2Result<Agent> {
    debug!("Creating agent");
    let conn = conn.lock().unwrap();
    conn.execute(
        "INSERT INTO agents (name, identity, platform) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, identity.to_bytes(), platform.to_string()],
    )?;

    Ok(conn.query_row(
            "SELECT id, name, identity, platform, created_at, last_seen_at FROM agents WHERE id = last_insert_rowid()",
            [],
            row_to_agent,
        )?)
}

pub fn get(conn: ThreadSafeConnection) -> C2Result<Vec<Agent>> {
    debug!("Getting agents");
    Ok(conn
        .lock()
        .unwrap()
        .prepare("SELECT id, name, identity, platform, created_at, last_seen_at FROM agents")?
        .query_map([], row_to_agent)?
        .map(Result::unwrap)
        .collect())
}

pub fn get_by_id(conn: ThreadSafeConnection, id: i32) -> RusqliteResult<Option<Agent>> {
    debug!("Getting agent {}", id);
    let conn = conn.lock().unwrap();
    conn.query_row(
        "SELECT id, name, identity, platform, created_at, last_seen_at FROM agents WHERE id = ?1",
        [id],
        row_to_agent,
    )
    .optional()
}

pub fn get_by_identity(
    conn: ThreadSafeConnection,
    identity: crypto::VerifyingKey,
) -> RusqliteResult<Option<Agent>> {
    debug!("Getting agent by identity");
    let conn = conn.lock().unwrap();
    conn.query_row(
            "SELECT id, name, identity, platform, created_at, last_seen_at FROM agents WHERE identity = ?1",
            [identity.to_bytes()],
            row_to_agent,
        ).optional()
}

pub fn update_by_id(conn: ThreadSafeConnection, agent: &model::Agent) -> C2Result<Agent> {
    debug!("Updating agent {} name", agent.id);
    let conn = conn.lock().unwrap();

    conn.execute(
        "UPDATE agents SET name = ?1, identity = ?2, platform = ?3 WHERE id = ?4",
        rusqlite::params![
            agent.name,
            agent.identity.to_bytes(),
            agent.platform.to_string(),
            agent.id
        ],
    )?;

    Ok(conn.query_row(
        "SELECT id, name, identity, platform, created_at, last_seen_at FROM agents WHERE id = ?1",
        [agent.id],
        row_to_agent,
    )?)
}
