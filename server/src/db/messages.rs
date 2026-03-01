use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub author_id: String,
    pub author_username: String,
    pub content: String,
    pub edited_at: Option<String>,
    pub created_at: String,
}

fn row_to_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    Ok(Message {
        id: row.get(0)?,
        channel_id: row.get(1)?,
        author_id: row.get(2)?,
        author_username: row.get(3)?,
        content: row.get(4)?,
        edited_at: row.get(5)?,
        created_at: row.get(6)?,
    })
}

const SELECT_MESSAGE: &str =
    "SELECT m.id, m.channel_id, m.author_id, u.username, m.content, m.edited_at, m.created_at
     FROM messages m JOIN users u ON m.author_id = u.id";

pub fn create_message(
    conn: &Connection,
    id: &str,
    channel_id: &str,
    author_id: &str,
    content: &str,
) -> rusqlite::Result<Message> {
    conn.execute(
        "INSERT INTO messages (id, channel_id, author_id, content) VALUES (?1, ?2, ?3, ?4)",
        params![id, channel_id, author_id, content],
    )?;

    // Insert into FTS index
    let rowid: i64 = conn.query_row(
        "SELECT rowid FROM messages WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO messages_fts(rowid, content) VALUES (?1, ?2)",
        params![rowid, content],
    )?;

    get_message_by_id(conn, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_message_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Option<Message>> {
    let sql = format!("{SELECT_MESSAGE} WHERE m.id = ?1");
    match conn.query_row(&sql, params![id], row_to_message) {
        Ok(msg) => Ok(Some(msg)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_channel_messages(
    conn: &Connection,
    channel_id: &str,
    before: Option<&str>,
    limit: i64,
) -> rusqlite::Result<Vec<Message>> {
    let mut messages = if let Some(before_id) = before {
        let sql = format!(
            "{SELECT_MESSAGE} WHERE m.channel_id = ?1 AND m.id < ?2 ORDER BY m.id DESC LIMIT ?3"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![channel_id, before_id, limit], row_to_message)?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let sql = format!(
            "{SELECT_MESSAGE} WHERE m.channel_id = ?1 ORDER BY m.id DESC LIMIT ?2"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![channel_id, limit], row_to_message)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    // Reverse to get chronological order
    messages.reverse();
    Ok(messages)
}

pub fn search_messages(
    conn: &Connection,
    query: &str,
    channel_id: Option<&str>,
    limit: i64,
) -> rusqlite::Result<Vec<Message>> {
    let messages = if let Some(ch_id) = channel_id {
        let sql = format!(
            "{SELECT_MESSAGE}
             JOIN messages_fts ON messages_fts.rowid = m.rowid
             WHERE messages_fts MATCH ?1 AND m.channel_id = ?2
             ORDER BY rank LIMIT ?3"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![query, ch_id, limit], row_to_message)?;
        rows.collect::<Result<Vec<_>, _>>()?
    } else {
        let sql = format!(
            "{SELECT_MESSAGE}
             JOIN messages_fts ON messages_fts.rowid = m.rowid
             WHERE messages_fts MATCH ?1
             ORDER BY rank LIMIT ?2"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![query, limit], row_to_message)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    Ok(messages)
}

pub fn edit_message(
    conn: &Connection,
    message_id: &str,
    content: &str,
) -> rusqlite::Result<Option<Message>> {
    let rows = conn.execute(
        "UPDATE messages SET content = ?1, edited_at = datetime('now') WHERE id = ?2",
        params![content, message_id],
    )?;
    if rows == 0 {
        return Ok(None);
    }

    // Update FTS
    let rowid: i64 = conn.query_row(
        "SELECT rowid FROM messages WHERE id = ?1",
        params![message_id],
        |row| row.get(0),
    )?;
    // Delete old FTS entry and insert new one
    conn.execute(
        "DELETE FROM messages_fts WHERE rowid = ?1",
        params![rowid],
    ).ok();
    conn.execute(
        "INSERT INTO messages_fts(rowid, content) VALUES (?1, ?2)",
        params![rowid, content],
    )?;

    get_message_by_id(conn, message_id)
}

pub fn delete_message(conn: &Connection, message_id: &str) -> rusqlite::Result<bool> {
    // Delete from FTS first
    if let Ok(rowid) = conn.query_row::<i64, _, _>(
        "SELECT rowid FROM messages WHERE id = ?1",
        params![message_id],
        |row| row.get(0),
    ) {
        conn.execute(
            "DELETE FROM messages_fts WHERE rowid = ?1",
            params![rowid],
        ).ok();
    }

    let rows = conn.execute("DELETE FROM messages WHERE id = ?1", params![message_id])?;
    Ok(rows > 0)
}
