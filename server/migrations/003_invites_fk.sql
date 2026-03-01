-- Add foreign key constraint to invites.created_by
-- SQLite doesn't support ALTER TABLE ADD CONSTRAINT, so we recreate the table

CREATE TABLE IF NOT EXISTS invites_new (
    code TEXT PRIMARY KEY,
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    max_uses INTEGER,
    uses INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO invites_new (code, created_by, max_uses, uses, expires_at, created_at)
    SELECT code, created_by, max_uses, uses, expires_at, created_at FROM invites;

DROP TABLE invites;
ALTER TABLE invites_new RENAME TO invites;
