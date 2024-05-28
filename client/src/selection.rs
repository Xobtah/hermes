// TODO Instead of &'a str use <S: Into<String>>
pub struct Item<'a, T, F>
where
    F: Fn() -> T,
{
    pub name: &'a str,
    pub action: F,
}

impl<'a, T, F> Item<'a, T, F>
where
    F: Fn() -> T,
{
    pub const fn new(name: &'a str, action: F) -> Self {
        Self { name, action }
    }
}

pub struct Selection<'a, T, F>
where
    F: Fn() -> T,
{
    pub actions: &'a [Item<'a, T, F>],
}

impl<'a, T, F> From<&'a [Item<'a, T, F>]> for Selection<'a, T, F>
where
    F: Fn() -> T,
{
    fn from(value: &'a [Item<'a, T, F>]) -> Self {
        Self { actions: value }
    }
}

impl<'a, T, F> Selection<'a, T, F>
where
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
                        .and_then(|action| Some((action.action)()))
                }),
        )
    }
}
