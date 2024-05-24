// TODO Fix the way the data is fetched, API has to be the same for every entity

pub mod agents {
    use common::model::{self, Agent};
    use tracing::debug;

    use crate::{error::C2Result, ThreadSafeConnection};

    fn row_to_agent(row: &rusqlite::Row) -> Result<Agent, rusqlite::Error> {
        Ok(Agent {
            id: row.get("id")?,
            name: row.get("name")?,
            identity: row.get("identity")?,
            platform: serde_json::from_str(&row.get::<_, String>("platform")?).unwrap(),
            created_at: row.get("created_at")?,
            last_seen_at: row.get("last_seen_at")?,
        })
    }

    pub fn create(
        conn: ThreadSafeConnection,
        name: &str,
        identity: [u8; 32],
        platform: model::Platform,
    ) -> C2Result<Agent> {
        debug!("Creating agent");
        let conn = conn.lock().unwrap();
        conn.execute(
            "INSERT INTO agents (name, identity, platform) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, identity, serde_json::to_string(&platform)?],
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

    pub fn get_by_id(conn: ThreadSafeConnection, id: i32) -> Option<Agent> {
        debug!("Getting agent {}", id);
        let conn = conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, identity, platform, created_at, last_seen_at FROM agents WHERE id = ?1",
            [id],
            row_to_agent,
        )
        .ok()
    }

    pub fn get_by_identity(conn: ThreadSafeConnection, identity: [u8; 32]) -> Option<Agent> {
        debug!("Getting agent by identity");
        let conn = conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, identity, platform, created_at, last_seen_at FROM agents WHERE identity = ?1",
            [identity],
            row_to_agent,
        )
        .ok()
    }

    pub fn update_name_by_id(conn: ThreadSafeConnection, id: i32, name: &str) -> C2Result<Agent> {
        debug!("Updating agent {} name", id);
        let conn = conn.lock().unwrap();

        conn.execute(
            "UPDATE agents SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, id],
        )?;

        Ok(conn.query_row(
            "SELECT id, name, identity, platform, created_at, last_seen_at FROM agents WHERE id = ?1",
            [id],
            row_to_agent,
        )?)
    }
}

pub mod missions {
    use std::sync::{Arc, Mutex};

    use common::model::Mission;
    use tracing::debug;

    use crate::{error::C2Result, ThreadSafeConnection};

    fn row_to_mission(row: &rusqlite::Row) -> Result<Mission, rusqlite::Error> {
        Ok(Mission {
            id: row.get("id")?,
            agent_id: row.get("agent_id")?,
            task: serde_json::from_str(&row.get::<_, String>("task")?).unwrap(),
            result: row.get("result")?,
            issued_at: row.get("issued_at")?,
            completed_at: row.get("completed_at")?,
        })
    }

    pub fn create(
        conn: ThreadSafeConnection,
        agent_id: i32,
        task: common::model::Task,
    ) -> C2Result<Mission> {
        debug!("Creating mission for agent {}", agent_id);
        let conn = conn.lock().unwrap();

        conn.execute(
            "INSERT INTO missions (agent_id, task) VALUES (?1, ?2)",
            rusqlite::params![agent_id, serde_json::to_string(&task)?],
        )?;

        Ok(conn.query_row(
            "SELECT id, agent_id, task, result, issued_at, completed_at FROM missions WHERE id = last_insert_rowid()",
            [],
            row_to_mission,
        )?)
    }

    pub fn get_next(conn: Arc<Mutex<rusqlite::Connection>>, agent_id: i32) -> Option<Mission> {
        debug!("Getting next mission for agent {}", agent_id);
        conn.lock().unwrap().query_row(
            "SELECT id, agent_id, task, result, issued_at, completed_at FROM missions WHERE agent_id = ?1 AND completed_at IS NULL ORDER BY issued_at ASC LIMIT 1",
            [agent_id],
            row_to_mission,
        )
        .ok()
    }

    pub async fn poll_next(
        conn: Arc<Mutex<rusqlite::Connection>>,
        agent_id: i32,
    ) -> Option<Mission> {
        debug!("Polling next mission for agent {}", agent_id);
        for _ in 0..5 {
            if let Some(mission) = get_next(conn.clone(), agent_id) {
                return Some(mission);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        None
    }

    pub fn get_by_id(conn: ThreadSafeConnection, id: i32) -> C2Result<Option<Mission>> {
        debug!("Getting mission {}", id);
        let conn = conn.lock().unwrap();
        match conn.query_row(
            "SELECT id, agent_id, task, result, issued_at, completed_at FROM missions WHERE id = ?1 LIMIT 1",
            [id],
            row_to_mission,
        ) {
            Ok(mission) => Ok(Some(mission)),
            Err(e) => if rusqlite::Error::QueryReturnedNoRows == e {
                Ok(None)
            } else {
                Err(e.into())
            },
        }
    }

    pub fn complete(conn: ThreadSafeConnection, id: i32, result: &str) -> C2Result<Mission> {
        debug!("Completing mission {}", id);
        let conn = conn.lock().unwrap();

        conn.execute(
            "UPDATE missions SET result = ?1, completed_at = CURRENT_TIMESTAMP WHERE id = ?2",
            rusqlite::params![result, id],
        )?;

        Ok(conn.query_row(
            "SELECT id, agent_id, task, result, issued_at, completed_at FROM missions WHERE id = ?1",
            [id],
            row_to_mission,
        )?)
    }
}
