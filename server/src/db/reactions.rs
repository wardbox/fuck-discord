use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Reaction {
    pub emoji: String,
    pub count: i64,
    pub users: Vec<String>,
}

pub fn add_reaction(
    conn: &Connection,
    message_id: &str,
    user_id: &str,
    emoji: &str,
) -> rusqlite::Result<bool> {
    let affected = conn.execute(
        "INSERT OR IGNORE INTO reactions (message_id, user_id, emoji) VALUES (?1, ?2, ?3)",
        params![message_id, user_id, emoji],
    )?;
    Ok(affected > 0)
}

pub fn remove_reaction(
    conn: &Connection,
    message_id: &str,
    user_id: &str,
    emoji: &str,
) -> rusqlite::Result<bool> {
    let affected = conn.execute(
        "DELETE FROM reactions WHERE message_id = ?1 AND user_id = ?2 AND emoji = ?3",
        params![message_id, user_id, emoji],
    )?;
    Ok(affected > 0)
}

pub fn get_reactions(conn: &Connection, message_id: &str) -> rusqlite::Result<Vec<Reaction>> {
    let mut stmt = conn.prepare(
        "SELECT emoji, COUNT(*) as count, GROUP_CONCAT(user_id) as users
         FROM reactions WHERE message_id = ?1
         GROUP BY emoji ORDER BY MIN(created_at)",
    )?;
    let rows = stmt.query_map(params![message_id], |row| {
        let users_str: String = row.get(2)?;
        Ok(Reaction {
            emoji: row.get(0)?,
            count: row.get(1)?,
            users: users_str.split(',').map(String::from).collect(),
        })
    })?;
    rows.collect()
}

pub fn get_reactions_for_messages(
    conn: &Connection,
    message_ids: &[String],
) -> rusqlite::Result<std::collections::HashMap<String, Vec<Reaction>>> {
    if message_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let placeholders: Vec<String> = (1..=message_ids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT message_id, emoji, COUNT(*) as count, GROUP_CONCAT(user_id) as users
         FROM reactions WHERE message_id IN ({})
         GROUP BY message_id, emoji ORDER BY MIN(created_at)",
        placeholders.join(",")
    );

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = message_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        let message_id: String = row.get(0)?;
        let users_str: String = row.get(3)?;
        Ok((
            message_id,
            Reaction {
                emoji: row.get(1)?,
                count: row.get(2)?,
                users: users_str.split(',').map(String::from).collect(),
            },
        ))
    })?;

    let mut map: std::collections::HashMap<String, Vec<Reaction>> =
        std::collections::HashMap::new();
    for row in rows {
        let (msg_id, reaction) = row?;
        map.entry(msg_id).or_default().push(reaction);
    }
    Ok(map)
}
