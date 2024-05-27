use common::model;

use crate::error::ClientResult;

pub fn prompt(prompt: &str) -> Result<String, dialoguer::Error> {
    dialoguer::Input::new()
        .with_prompt(prompt)
        .interact_text()
        .map(|s: String| s.trim().to_string())
}

pub fn select_agent(agents: &[model::Agent]) -> ClientResult<Option<&model::Agent>> {
    Ok(
        dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select an agent")
            .default(0)
            .items(&agents.iter().map(agent_fmt).collect::<Vec<_>>())
            .interact_opt()?
            .and_then(|i| agents.get(i)),
    )
}

// TODO https://crates.io/crates/timeago
pub fn agent_fmt(agent: &model::Agent) -> String {
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
