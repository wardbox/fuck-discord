use chrono::{Duration, Utc};
use rand::Rng;
use rusqlite::{params, Connection};

const SESSION_DURATION_DAYS: i64 = 30;

fn generate_session_id() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    hex::encode(bytes)
}

pub fn create_session(conn: &Connection, user_id: &str) -> rusqlite::Result<String> {
    let session_id = generate_session_id();
    let expires_at = (Utc::now() + Duration::days(SESSION_DURATION_DAYS))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    conn.execute(
        "INSERT INTO sessions (id, user_id, expires_at) VALUES (?1, ?2, ?3)",
        params![session_id, user_id, expires_at],
    )?;

    Ok(session_id)
}

pub fn validate_session(conn: &Connection, session_id: &str) -> rusqlite::Result<Option<String>> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    match conn.query_row(
        "SELECT user_id FROM sessions WHERE id = ?1 AND expires_at > ?2",
        params![session_id, now],
        |row| row.get::<_, String>(0),
    ) {
        Ok(user_id) => Ok(Some(user_id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn delete_session(conn: &Connection, session_id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
    Ok(())
}

// We need hex encoding for session IDs
mod hex {
    pub fn encode(bytes: Vec<u8>) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}
