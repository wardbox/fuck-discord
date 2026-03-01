pub mod channels;
pub mod invites;
pub mod messages;
pub mod reactions;
pub mod users;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::state::DbPool;

pub fn create_pool(database_path: &str) -> anyhow::Result<DbPool> {
    let manager = SqliteConnectionManager::file(database_path)
        .with_init(|conn| {
            // Use pragma_update_and_check and handle PRAGMAs that may
            // not return rows. The `extra_check` feature (from bundled-full)
            // makes execute_batch/pragma_update error on result-returning PRAGMAs.
            let set_pragma = |conn: &Connection, name: &str, value: &dyn rusqlite::types::ToSql| {
                conn.pragma_update_and_check(None, name, value, |_| Ok(()))
                    .or_else(|e| match e {
                        rusqlite::Error::QueryReturnedNoRows => Ok(()),
                        other => Err(other),
                    })
            };
            set_pragma(conn, "busy_timeout", &5000)?;
            set_pragma(conn, "synchronous", &"NORMAL")?;
            set_pragma(conn, "foreign_keys", &"ON")?;
            set_pragma(conn, "cache_size", &(-64000))?;
            Ok(())
        });
    let pool = Pool::builder().max_size(8).build(manager)?;

    // WAL mode is database-level, only needs to be set once
    {
        let conn = pool.get()?;
        let _: String = conn
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    }

    Ok(pool)
}

pub fn run_migrations(pool: &DbPool) -> rusqlite::Result<()> {
    let conn = pool.get().map_err(|e| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some(format!("Pool error: {e}")),
        )
    })?;
    migrate(&conn)
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );",
    )?;

    let version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    if version < 1 {
        conn.execute_batch(include_str!("../../migrations/001_initial.sql"))?;
        conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (1)", [])?;
    }

    if version < 2 {
        conn.execute_batch(include_str!("../../migrations/002_reactions.sql"))?;
        conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (2)", [])?;
    }

    if version < 3 {
        conn.execute_batch(include_str!("../../migrations/003_invites_fk.sql"))?;
        conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (3)", [])?;
    }

    if version < 4 {
        conn.execute_batch(include_str!("../../migrations/004_invites_nullable_created_by.sql"))?;
        conn.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (4)", [])?;
    }

    let current = version.max(4);
    tracing::info!("Database at schema version {current}");
    Ok(())
}
