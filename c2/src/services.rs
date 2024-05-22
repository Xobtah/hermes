pub mod agents {
    use common::api::Agent;
    use tracing::debug;

    use crate::error::C2Result;

    pub fn create(conn: &rusqlite::Connection, name: &str, identity: [u8; 32]) -> C2Result<Agent> {
        debug!("Creating agent");
        conn.execute(
            "INSERT INTO agents (name, identity) VALUES (?1, ?2)",
            rusqlite::params![name, identity],
        )?;

        Ok(conn.query_row(
            "SELECT id, name, identity FROM agents WHERE id = last_insert_rowid()",
            [],
            |row| {
                Ok(Agent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    identity: row.get(2)?,
                })
            },
        )?)
    }

    pub fn get(conn: &rusqlite::Connection) -> C2Result<Vec<Agent>> {
        debug!("Getting agents");
        Ok(conn
            .prepare("SELECT id, name, identity FROM agents")?
            .query_map([], |row| {
                Ok(Agent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    identity: row.get(2)?,
                })
            })?
            .map(Result::unwrap)
            .collect())
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: i32) -> Option<Agent> {
        debug!("Getting agent {}", id);
        conn.query_row(
            "SELECT id, name, identity FROM agents WHERE id = ?1",
            [id],
            |row| {
                Ok(Agent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    identity: row.get(2)?,
                })
            },
        )
        .ok()
    }

    pub fn get_by_identity(conn: &rusqlite::Connection, identity: [u8; 32]) -> Option<Agent> {
        debug!("Getting agent by identity");
        conn.query_row(
            "SELECT id, name, identity FROM agents WHERE identity = ?1",
            [identity],
            |row| {
                Ok(Agent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    identity: row.get(2)?,
                })
            },
        )
        .ok()
    }
}

pub mod missions {
    use common::{api::Mission, crypto};
    use tracing::debug;

    use crate::error::C2Result;

    pub fn create(
        conn: &rusqlite::Connection,
        agent_id: i32,
        task: common::api::Task,
        (public_key, private_key): (crypto::KeyExchangePublicKey, crypto::KeyExchangePrivateKey),
    ) -> C2Result<Mission> {
        debug!("Creating mission for agent {}", agent_id);
        conn.execute(
            "INSERT INTO missions (agent_id, task, public_key, private_key) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![agent_id, serde_json::to_string(&task)?, public_key, private_key],
        )?;

        Ok(conn.query_row(
            "SELECT id, agent_id, task, result, public_key, issued_at, completed_at FROM missions WHERE id = last_insert_rowid()",
            [],
            |row| {
                Ok(Mission {
                    id: row.get("id")?,
                    agent_id: row.get("agent_id")?,
                    task: serde_json::from_str(&row.get::<_, String>("task")?).unwrap(),
                    result: row.get("result")?,
                    public_key: row.get("public_key")?,
                    issued_at: row.get("issued_at")?,
                    completed_at: row.get("completed_at")?,
                })
            },
        )?)
    }

    pub fn get_next(conn: &rusqlite::Connection, agent_id: i32) -> Option<Mission> {
        debug!("Getting next mission for agent {}", agent_id);
        conn.query_row(
            "SELECT id, agent_id, task, result, public_key, issued_at, completed_at FROM missions WHERE agent_id = ?1 AND completed_at IS NULL ORDER BY issued_at ASC LIMIT 1",
            [agent_id],
            |row| {
                Ok(Mission {
                    id: row.get("id")?,
                    agent_id: row.get("agent_id")?,
                    task: serde_json::from_str(&row.get::<_, String>("task")?).unwrap(),
                    result: row.get("result")?,
                    public_key: row.get("public_key")?,
                    issued_at: row.get("issued_at")?,
                    completed_at: row.get("completed_at")?,
                })
            },
        )
        .ok()
    }

    pub fn get_by_id(
        conn: &rusqlite::Connection,
        id: i32,
    ) -> C2Result<Option<(Mission, crypto::KeyExchangePrivateKey)>> {
        debug!("Getting mission {}", id);
        match conn.query_row(
            "SELECT id, agent_id, task, result, public_key, private_key, issued_at, completed_at FROM missions WHERE id = ?1 LIMIT 1",
            [id],
            |row| {
                Ok((Mission {
                    id: row.get("id")?,
                    agent_id: row.get("agent_id")?,
                    task: serde_json::from_str(&row.get::<_, String>("task")?).unwrap(),
                    result: row.get("result")?,
                    public_key: row.get("public_key").unwrap_or_default(),
                    issued_at: row.get("issued_at")?,
                    completed_at: row.get("completed_at")?,
                }, row.get("private_key").unwrap_or_default()))
            },
        ) {
            Ok((mission, private_key)) => Ok(Some((mission, private_key))),
            Err(e) => if rusqlite::Error::QueryReturnedNoRows == e {
                Ok(None)
            } else {
                Err(e.into())
            },
        }
    }

    pub fn complete(conn: &rusqlite::Connection, id: i32, result: &str) -> C2Result<Mission> {
        debug!("Completing mission {}", id);
        conn.execute(
            "UPDATE missions SET result = ?1, public_key = NULL, private_key = NULL, completed_at = CURRENT_TIMESTAMP WHERE id = ?2",
            rusqlite::params![result, id],
        )?;

        Ok(conn.query_row(
            "SELECT id, agent_id, task, result, public_key, issued_at, completed_at FROM missions WHERE id = ?1",
            [id],
            |row| {
                Ok(Mission {
                    id: row.get("id")?,
                    agent_id: row.get("agent_id")?,
                    task: serde_json::from_str(&row.get::<_, String>("task")?).unwrap(),
                    result: row.get("result")?,
                    public_key: crypto::KeyExchangePublicKey::default(),
                    issued_at: row.get("issued_at")?,
                    completed_at: row.get("completed_at")?,
                })
            },
        )?)
    }
}
