use rand::Rng;
use rusqlite::Connection;

use crate::db;

const INVITE_CODE_LENGTH: usize = 8;

fn generate_code() -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijkmnpqrstuvwxyz23456789".chars().collect();
    (0..INVITE_CODE_LENGTH)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

pub fn create_invite_code(
    conn: &Connection,
    created_by: Option<&str>,
    max_uses: Option<i64>,
    expires_at: Option<&str>,
) -> rusqlite::Result<String> {
    let code = generate_code();
    db::invites::create_invite(conn, &code, created_by, max_uses, expires_at)?;
    Ok(code)
}
