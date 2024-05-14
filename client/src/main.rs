fn prompt(prompt: &str) -> Result<String, anyhow::Error> {
    dialoguer::Input::new()
        .with_prompt(prompt)
        .interact_text()
        .map(|s: String| s.trim().to_string())
        .map_err(|e| anyhow::anyhow!("Prompting failed: {e}"))
}

fn main() -> anyhow::Result<()> {
    // let selection = dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
    //     .with_prompt("Select a subcommand")
    //     .default(0)
    //     .items(&["List agents", "Send command"])
    //     .interact_opt()?;
    // if let Some(selection) = selection {
    //     return Ok(match selection {
    //         // 0 => Self::Encrypt {
    //         //     input: prompt("Input file")?.into(),
    //         //     key: None,
    //         // },
    //         // 1 => Self::Decrypt {
    //         //     input: prompt("Input file")?.into(),
    //         //     key: prompt("Key")?,
    //         // },
    //         _ => unreachable!(),
    //     });
    // } else {
    //     return Err(anyhow::anyhow!("No subcommand selected"));
    // }
    Ok(())
}
