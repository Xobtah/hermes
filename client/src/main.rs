use std::fs;

use base64::{prelude::BASE64_STANDARD, Engine as _};
use common::{client, crypto, model};
use error::ClientResult;
use selection::{Item, Selection};

mod error;
mod selection;
mod utils;

const PLATFORMS: &[Item<&str, model::Platform, fn() -> model::Platform>] = &[
    Item::new("Unix", || model::Platform::Unix),
    Item::new("Windows", || model::Platform::Windows),
];

const MAIN_MENU_COMMANDS: &[Item<
    &str,
    ClientResult<Option<Menu>>,
    fn() -> ClientResult<Option<Menu>>,
>] = &[
    Item::new("Select agent", || {
        let agents = client::agents::get()?;
        if agents.is_empty() {
            println!("No agents available");
            return Ok(None);
        }
        Ok(Some(Menu::SelectAgent(client::agents::get()?)))
    }),
    Item::new("Create agent", || {
        let Some(platform) = Selection::from(PLATFORMS).select("Select platform")? else {
            println!("No platform selected");
            return Ok(None);
        };

        client::agents::create(
            utils::prompt("Agent name", None)?,
            crypto::VerifyingKey::from_bytes(
                fs::read(utils::prompt("Agent identity public key file path", None)?)?
                    .as_slice()
                    .try_into()
                    .unwrap(),
            )
            .unwrap(),
            platform,
        )?;
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

const AGENT_COMMANDS: for<'a> fn(
    &'a model::Agent,
) -> [Item<
    &'static str,
    ClientResult<Option<Menu>>,
    Box<dyn 'a + Fn() -> ClientResult<Option<Menu>>>,
>; 4] = |agent| {
    [
        Item::new(
            "Execute command",
            Box::new(move || {
                let mission = client::missions::issue(
                    agent.id,
                    model::Task::Execute(utils::prompt("Command", None)?),
                )?;
                utils::poll_mission_result(mission.id);
                Ok(None)
            }),
        ),
        Item::new(
            "Update agent binary",
            Box::new(move || {
                let (bin_path, vk_path) = match agent.platform {
                    model::Platform::Unix => ("target/release/agent", "target/release/id-pub.key"),
                    model::Platform::Windows => (
                        "target/x86_64-pc-windows-gnu/release/agent.exe",
                        "target/x86_64-pc-windows-gnu/release/id-pub.key",
                    ),
                };

                let mission = client::missions::issue(
                    agent.id,
                    model::Task::Update(model::Release {
                        checksum: common::checksum(bin_path)?,
                        verifying_key: crypto::VerifyingKey::from_bytes(
                            fs::read(vk_path)?.as_slice().try_into().unwrap(),
                        )
                        .unwrap(),
                        bytes: common::compress(&fs::read(bin_path)?),
                        created_at: Default::default(),
                    }),
                )?;
                utils::poll_mission_result(mission.id);
                Ok(None)
            }),
        ),
        Item::new(
            "Update agent data",
            Box::new(|| {
                let agents = client::agents::get()?;

                let Some(agent) = utils::select_agent(&agents)? else {
                    println!("No agent selected");
                    return Ok(None);
                };

                client::agents::update(&model::Agent {
                    name: utils::prompt("Agent name", Some(agent.name.clone()))?,
                    identity: crypto::VerifyingKey::from_bytes(
                        fs::read(utils::prompt("Agent identity public key file path", None)?)?
                            .as_slice()
                            .try_into()
                            .unwrap(),
                    )
                    .unwrap(),
                    ..agent.clone()
                })?;
                Ok(None)
            }),
        ),
        Item::new(
            "Stop agent",
            Box::new(move || {
                let mission = client::missions::issue(agent.id, model::Task::Stop)?;
                utils::poll_mission_result(mission.id);
                Ok(None)
            }),
        ),
    ]
};

enum Menu {
    Main,
    SelectAgent(Vec<model::Agent>),
    Agent(model::Agent),
}

impl Menu {
    // TODO This is bad
    fn select(&self) -> Result<Option<ClientResult<Option<Menu>>>, dialoguer::Error> {
        match self {
            Menu::Main => Selection::from(MAIN_MENU_COMMANDS).select("Select a command"),
            Menu::SelectAgent(agents) => {
                if let Some(agent) = utils::select_agent(&agents)? {
                    Ok(Some(Ok(Some(Menu::Agent(agent)))))
                } else {
                    Ok(None)
                }
            }
            Menu::Agent(agent) => {
                let commands: [Item<
                    &str,
                    Result<Option<Menu>, error::ClientError>,
                    Box<dyn Fn() -> Result<Option<Menu>, error::ClientError>>,
                >; 4] = AGENT_COMMANDS(agent);
                Selection::from(&commands[..]).select(&format!("[{}] Select a mission", agent.name))
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
