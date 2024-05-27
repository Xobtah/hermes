use common::model;

use crate::error::ClientResult;

pub fn list_agents() -> ClientResult<Vec<model::Agent>> {
    let agents: Vec<model::Agent> = ureq::get("http://localhost:3000/agents")
        .call()?
        .into_json()?;
    Ok(agents)
}

pub fn issue_mission(agent_id: i32, task: model::Task) -> ClientResult<model::Mission> {
    // let f = fs::File::open("target/debug/agent")?;
    // let metadata = f.metadata()?;
    // println!("metadata len: {:?}", metadata.len());
    let mission: model::Mission = ureq::post("http://localhost:3000/missions")
        // .set("Content-Length", &metadata.len().to_string())
        .send_json(serde_json::to_value(&model::Mission {
            id: Default::default(),
            agent_id,
            task,
            result: None,
            issued_at: Default::default(),
            completed_at: None,
        })?)?
        .into_json()?;
    Ok(mission)
}

pub fn get_mission_result(mission_id: i32) -> ClientResult<Option<String>> {
    let response = ureq::get(&format!("http://localhost:3000/missions/{mission_id}")).call()?;
    if response.status() == 204 {
        Ok(None)
    } else {
        let result: Option<String> = response.into_json()?;
        Ok(result)
    }
}

pub mod agents {
    use common::{crypto, model};

    use crate::error::ClientResult;

    pub fn create(
        name: String,
        identity: crypto::VerifyingKey,
        platform: model::Platform,
    ) -> ClientResult<()> {
        let response = ureq::post("http://localhost:3000/agents").send_json(model::Agent {
            id: Default::default(),
            name,
            identity: identity.to_bytes(),
            platform,
            created_at: Default::default(),
            last_seen_at: Default::default(),
        })?;

        if response.status() == 201 {
            println!("Agent created");
        } else {
            eprintln!("Failed to create agent");
        }
        Ok(())
    }

    pub fn update(agent: &model::Agent) -> ClientResult<()> {
        if ureq::put(&format!("http://localhost:3000/agents/{}", agent.id))
            .send_json(agent)?
            .status()
            == 200
        {
            println!("Agent name updated");
        } else {
            eprintln!("Failed to update agent name");
        }
        Ok(())
    }
}
