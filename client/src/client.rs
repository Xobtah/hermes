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
