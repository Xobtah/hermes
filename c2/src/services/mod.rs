pub mod agents;
pub mod missions;
pub mod releases;

type RusqliteResult<T> = Result<T, rusqlite::Error>;
