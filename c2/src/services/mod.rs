pub mod agents;
pub mod missions;

type RusqliteResult<T> = Result<T, rusqlite::Error>;
