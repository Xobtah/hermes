use common::model;
use rusqlite::OptionalExtension;
use tracing::debug;

use crate::{error::C2Result, ThreadSafeConnection};

use super::RusqliteResult;

fn row_to_release(row: &rusqlite::Row) -> RusqliteResult<model::Release> {
    Ok(model::Release {
        checksum: row.get("checksum")?,
        platform: serde_json::from_str(&row.get::<_, String>("platform")?).unwrap(),
        bytes: row.get("bytes")?,
        created_at: row.get("created_at")?,
    })
}

pub fn create(
    conn: ThreadSafeConnection,
    checksum: &str,
    platform: model::Platform,
    bytes: &[u8],
) -> C2Result<model::Release> {
    debug!("Creating release");
    let conn = conn.lock().unwrap();
    conn.execute(
        "INSERT INTO releases (checksum, platform, bytes) VALUES (?1, ?2, ?3)",
        rusqlite::params![checksum, serde_json::to_string(&platform)?, bytes],
    )?;

    Ok(conn.query_row(
        "SELECT checksum, platform, bytes, created_at FROM releases WHERE ROWID = last_insert_rowid()",
        [],
        row_to_release,
    )?)
}

pub fn get(conn: ThreadSafeConnection) -> RusqliteResult<Vec<model::Release>> {
    debug!("Getting releases");
    conn.lock()
        .unwrap()
        .prepare("SELECT checksum, platform, bytes, created_at FROM releases")?
        .query_map([], row_to_release)?
        .collect()
}

pub fn get_by_checksum(
    conn: ThreadSafeConnection,
    checksum: &str,
) -> RusqliteResult<Option<model::Release>> {
    debug!("Getting release {checksum}");
    let conn = conn.lock().unwrap();
    conn.query_row(
        "SELECT checksum, platform, bytes, created_at FROM releases WHERE checksum = ?1",
        [checksum],
        row_to_release,
    )
    .optional()
}
