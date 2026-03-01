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
    // Check if invite exists and is valid
    let result = conn.query_row(
        "SELECT max_uses, uses, expires_at FROM invites WHERE code = ?1",
        params![code],
        |row| {
            let max_uses: Option<i64> = row.get(0)?;
            let uses: i64 = row.get(1)?;
            let expires_at: Option<String> = row.get(2)?;
            Ok((max_uses, uses, expires_at))
        },
    );

    match result {
        Ok((max_uses, uses, expires_at)) => {
            // Check if expired
            if let Some(exp) = expires_at {
                let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
                if now > exp {
                    return Ok(false);
                }
            }

            // Check if max uses reached
            if let Some(max) = max_uses {
                if uses >= max {
                    return Ok(false);
                }
            }

            // Increment uses
            conn.execute(
                "UPDATE invites SET uses = uses + 1 WHERE code = ?1",
                params![code],
            )?;

            Ok(true)
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e),
    }
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
