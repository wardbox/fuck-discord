use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: String,
    pub created_at: String,
}

/// Internal user with password hash for auth
pub struct UserWithAuth {
    pub user: User,
    pub password_hash: String,
}

pub fn create_user(
    conn: &Connection,
    id: &str,
    username: &str,
    password_hash: &str,
) -> rusqlite::Result<User> {
    conn.execute(
        "INSERT INTO users (id, username, password_hash) VALUES (?1, ?2, ?3)",
        params![id, username, password_hash],
    )?;

    get_user_by_id(conn, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_user_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Option<User>> {
    conn.query_row(
        "SELECT id, username, display_name, avatar_url, status, created_at
         FROM users WHERE id = ?1",
        params![id],
        |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                avatar_url: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
            })
        },
    )
    .optional()
}

pub fn get_user_by_username(conn: &Connection, username: &str) -> rusqlite::Result<Option<UserWithAuth>> {
    conn.query_row(
        "SELECT id, username, display_name, avatar_url, status, created_at, password_hash
         FROM users WHERE username = ?1 COLLATE NOCASE",
        params![username],
        |row| {
            Ok(UserWithAuth {
                user: User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    avatar_url: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                },
                password_hash: row.get(6)?,
            })
        },
    )
    .optional()
}

pub fn update_status(conn: &Connection, user_id: &str, status: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE users SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![status, user_id],
    )?;
    Ok(())
}

pub fn get_all_users(conn: &Connection) -> rusqlite::Result<Vec<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, username, display_name, avatar_url, status, created_at FROM users",
    )?;
    let users = stmt
        .query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                avatar_url: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(users)
}

trait OptionalExt<T> {
    fn optional(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalExt<T> for rusqlite::Result<T> {
    fn optional(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
