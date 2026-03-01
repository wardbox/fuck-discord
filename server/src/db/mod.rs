pub mod channels;
pub mod invites;
pub mod messages;
pub mod reactions;
pub mod users;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::state::DbPool;

pub fn create_pool(database_path: &str) -> Result<DbPool, r2d2::Error> {
    let manager = SqliteConnectionManager::file(database_path);
    let pool = Pool::builder().max_size(8).build(manager)?;

    // Configure the database with a single connection first
    {
        let conn = pool.get().expect("Failed to get initial connection");
        // Use pragma_query_value for pragmas that return results
        // journal_mode returns the new mode, so use query_row
        let _: String = conn
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
            .expect("Failed to set WAL mode");
    }

    Ok(pool)
}

fn configure_connection(conn: &Connection) {
    let _ = conn.pragma_update(None, "busy_timeout", "5000");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    let _ = conn.pragma_update(None, "cache_size", "-64000");
}

pub fn run_migrations(pool: &DbPool) -> rusqlite::Result<()> {
    let conn = pool.get().map_err(|e| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some(format!("Pool error: {e}")),
        )
    })?;
    configure_connection(&conn);
    migrate(&conn)
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );",
    )?;

    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if version < 1 {
        conn.execute_batch(include_str!("../../migrations/001_initial.sql"))?;
        conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
    }

    if version < 2 {
        conn.execute_batch(include_str!("../../migrations/002_reactions.sql"))?;
        conn.execute("INSERT INTO schema_version (version) VALUES (2)", [])?;
    }

    let current = version.max(2);
    tracing::info!("Database at schema version {current}");
    Ok(())
}
