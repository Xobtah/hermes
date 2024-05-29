pub struct Item<S: AsRef<str>, T, F>
where
    F: Fn() -> T,
{
    pub name: S,
    pub command: F,
}

impl<S, T, F> Item<S, T, F>
where
    S: AsRef<str>,
    F: Fn() -> T,
{
    pub const fn new(name: S, command: F) -> Self {
        Self { name, command }
    }
}

pub struct Selection<'a, S, T, F>
where
    S: AsRef<str>,
    F: Fn() -> T,
{
    pub actions: &'a [Item<S, T, F>],
}

impl<'a, S, T, F> From<&'a [Item<S, T, F>]> for Selection<'a, S, T, F>
where
    S: AsRef<str>,
    F: Fn() -> T,
{
    fn from(actions: &'a [Item<S, T, F>]) -> Self {
        Self { actions }
    }
}

impl<'a, S, T, F> Selection<'a, S, T, F>
where
    S: AsRef<str> + std::fmt::Display,
    F: Fn() -> T,
{
    pub fn select(&self, prompt: &str) -> Result<Option<T>, dialoguer::Error> {
        Ok(
            dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt(prompt)
                .default(0)
                .items(&self.actions.iter().map(|a| &a.name).collect::<Vec<_>>())
                .interact_opt()?
                .and_then(|i| {
                    self.actions
                        .get(i)
                        .and_then(|action| Some((action.command)()))
                }),
        )
    }
}
