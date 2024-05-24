// TODO Fix the way the data is fetched, API has to be the same for every entity

pub mod agents {
    use common::model::{self, Agent};
    use tracing::debug;

    use crate::error::C2Result;

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
        conn: &rusqlite::Connection,
        name: &str,
        identity: [u8; 32],
        platform: model::Platform,
    ) -> C2Result<Agent> {
        debug!("Creating agent");
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

    pub fn get(conn: &rusqlite::Connection) -> C2Result<Vec<Agent>> {
        debug!("Getting agents");
        Ok(conn
            .prepare("SELECT id, name, identity, platform, created_at, last_seen_at FROM agents")?
            .query_map([], row_to_agent)?
            .map(Result::unwrap)
            .collect())
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: i32) -> Option<Agent> {
        debug!("Getting agent {}", id);
        conn.query_row(
            "SELECT id, name, identity, platform, created_at, last_seen_at FROM agents WHERE id = ?1",
            [id],
            row_to_agent,
        )
        .ok()
    }

    pub fn get_by_identity(conn: &rusqlite::Connection, identity: [u8; 32]) -> Option<Agent> {
        debug!("Getting agent by identity");
        conn.query_row(
            "SELECT id, name, identity, platform, created_at, last_seen_at FROM agents WHERE identity = ?1",
            [identity],
            row_to_agent,
        )
        .ok()
    }

    pub fn update_name_by_id(conn: &rusqlite::Connection, id: i32, name: &str) -> C2Result<Agent> {
        debug!("Updating agent {} name", id);
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
    use common::model::Mission;
    use tracing::debug;

    use crate::error::C2Result;

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
        conn: &rusqlite::Connection,
        agent_id: i32,
        task: common::model::Task,
    ) -> C2Result<Mission> {
        debug!("Creating mission for agent {}", agent_id);
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

    pub fn get_next(conn: &rusqlite::Connection, agent_id: i32) -> Option<Mission> {
        debug!("Getting next mission for agent {}", agent_id);
        conn.query_row(
            "SELECT id, agent_id, task, result, issued_at, completed_at FROM missions WHERE agent_id = ?1 AND completed_at IS NULL ORDER BY issued_at ASC LIMIT 1",
            [agent_id],
            row_to_mission,
        )
        .ok()
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: i32) -> C2Result<Option<Mission>> {
        debug!("Getting mission {}", id);
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

    pub fn complete(conn: &rusqlite::Connection, id: i32, result: &str) -> C2Result<Mission> {
        debug!("Completing mission {}", id);
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
