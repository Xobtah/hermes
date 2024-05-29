use common::{client, model};

use crate::selection::{Item, Selection};

pub fn prompt<S: Into<String>>(prompt: S, default: Option<String>) -> Result<String, dialoguer::Error> {
    if let Some(default) = default {
        dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(prompt)
            .default(default)
            .interact_text()
            .map(|s: String| s.trim().to_string())
    } else {
        dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(prompt)
            .interact_text()
            .map(|s: String| s.trim().to_string())
    }
}

pub fn select_agent(agents: &[model::Agent]) -> Result<Option<model::Agent>, dialoguer::Error> {
    let select_agent = agents
        .into_iter()
        .map(|agent| Item::new(format!("{agent}"), || agent.clone()))
        .collect::<Vec<_>>();
    Selection::from(&select_agent[..]).select("Select an agent")
}

pub fn poll_mission_result(mission_id: i32) {
    loop {
        match client::missions::get_result(mission_id) {
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
