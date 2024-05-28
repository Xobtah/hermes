use std::path::Path;

use common::model;

use crate::error::ClientResult;

pub fn prompt<S: Into<String>>(prompt: S) -> Result<String, dialoguer::Error> {
    dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt(prompt)
        .interact_text()
        .map(|s: String| s.trim().to_string())
}

pub fn select_agent(agents: &[model::Agent]) -> ClientResult<Option<&model::Agent>> {
    Ok(
        dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select an agent")
            .default(0)
            .items(
                &agents
                    .iter()
                    .map(|agent| format!("{agent}"))
                    .collect::<Vec<_>>(),
            )
            .interact_opt()?
            .and_then(|i| agents.get(i)),
    )
}

pub fn checksum<P: AsRef<Path>>(path: P) -> ClientResult<String> {
    Ok(sha256::digest(
        std::fs::read(path)?.as_slice(),
    ))
}
