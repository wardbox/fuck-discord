use rusqlite::{params, Connection};

pub fn create_invite(
    conn: &Connection,
    code: &str,
    created_by: &str,
    max_uses: Option<i64>,
    expires_at: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO invites (code, created_by, max_uses, expires_at) VALUES (?1, ?2, ?3, ?4)",
        params![code, created_by, max_uses, expires_at],
    )?;
    Ok(())
}

pub fn validate_and_use_invite(conn: &Connection, code: &str) -> rusqlite::Result<bool> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let affected = conn.execute(
        "UPDATE invites SET uses = uses + 1
         WHERE code = ?1
           AND (max_uses IS NULL OR uses < max_uses)
           AND (expires_at IS NULL OR expires_at > ?2)",
        params![code, now],
    )?;
    Ok(affected > 0)
}

pub fn get_all_invites(
    conn: &Connection,
) -> rusqlite::Result<Vec<(String, String, Option<i64>, i64, Option<String>, String)>> {
    let mut stmt = conn.prepare(
        "SELECT code, created_by, max_uses, uses, expires_at, created_at FROM invites ORDER BY created_at DESC",
    )?;
    let invites = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(invites)
}
