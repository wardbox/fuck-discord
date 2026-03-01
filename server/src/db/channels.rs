use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub topic: Option<String>,
    pub category: Option<String>,
    pub channel_type: String,
    pub position: i64,
    pub created_at: String,
}

pub fn seed_defaults(conn: &Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM channels", [], |row| row.get(0))?;
    if count == 0 {
        let id = ulid::Ulid::new().to_string();
        conn.execute(
            "INSERT INTO channels (id, name, topic, position) VALUES (?1, 'general', 'General discussion', 0)",
            params![id],
        )?;
        tracing::info!("Created default #general channel");
    }
    Ok(())
}

pub fn create_channel(
    conn: &Connection,
    id: &str,
    name: &str,
    topic: Option<&str>,
    category: Option<&str>,
) -> rusqlite::Result<Channel> {
    let position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM channels",
        [],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO channels (id, name, topic, category, position) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, topic, category, position],
    )?;

    get_channel_by_id(conn, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_channel_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Option<Channel>> {
    match conn.query_row(
        "SELECT id, name, topic, category, channel_type, position, created_at
         FROM channels WHERE id = ?1",
        params![id],
        |row| {
            Ok(Channel {
                id: row.get(0)?,
                name: row.get(1)?,
                topic: row.get(2)?,
                category: row.get(3)?,
                channel_type: row.get(4)?,
                position: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    ) {
        Ok(channel) => Ok(Some(channel)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn update_channel(
    conn: &Connection,
    id: &str,
    name: Option<&str>,
    topic: Option<Option<&str>>,
    category: Option<Option<&str>>,
) -> rusqlite::Result<Option<Channel>> {
    if let Some(name) = name {
        conn.execute(
            "UPDATE channels SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
    }
    if let Some(topic) = topic {
        conn.execute(
            "UPDATE channels SET topic = ?1 WHERE id = ?2",
            params![topic, id],
        )?;
    }
    if let Some(category) = category {
        conn.execute(
            "UPDATE channels SET category = ?1 WHERE id = ?2",
            params![category, id],
        )?;
    }
    get_channel_by_id(conn, id)
}

pub fn delete_channel(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    let affected = conn.execute("DELETE FROM channels WHERE id = ?1", params![id])?;
    Ok(affected > 0)
}

pub fn get_all_channels(conn: &Connection) -> rusqlite::Result<Vec<Channel>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, topic, category, channel_type, position, created_at
         FROM channels ORDER BY position",
    )?;
    let channels = stmt
        .query_map([], |row| {
            Ok(Channel {
                id: row.get(0)?,
                name: row.get(1)?,
                topic: row.get(2)?,
                category: row.get(3)?,
                channel_type: row.get(4)?,
                position: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(channels)
}
