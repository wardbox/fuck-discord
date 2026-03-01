use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::Deserialize;

use crate::auth::middleware::AuthUser;
use crate::db;
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct MessageQuery {
    pub before: Option<String>,
    pub limit: Option<i64>,
}

pub async fn get_messages(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(channel_id): Path<String>,
    Query(query): Query<MessageQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let conn = state.db.get()?;
    let limit = query.limit.unwrap_or(50).min(100);
    let messages =
        db::messages::get_channel_messages(&conn, &channel_id, query.before.as_deref(), limit)?;

    // Fetch reactions for all messages
    let message_ids: Vec<String> = messages.iter().map(|m| m.id.clone()).collect();
    let reactions_map = db::reactions::get_reactions_for_messages(&conn, &message_ids)?;

    // Build response with reactions attached
    let result: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            let mut val = serde_json::to_value(m).unwrap();
            if let Some(reactions) = reactions_map.get(&m.id) {
                val["reactions"] = serde_json::to_value(reactions).unwrap();
            } else {
                val["reactions"] = serde_json::json!([]);
            }
            val
        })
        .collect();

    Ok(Json(serde_json::json!(result)))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub channel_id: Option<String>,
    pub limit: Option<i64>,
}

pub async fn search(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<Vec<db::messages::Message>>> {
    if query.q.trim().is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "Search query cannot be empty".to_string(),
        ));
    }

    let conn = state.db.get()?;
    let limit = query.limit.unwrap_or(25).min(100);

    // Transform query for FTS5 prefix matching: "test" → "\"test\"*"
    // This makes "test" match "test", "testing", "tests", etc.
    let fts_query = query
        .q
        .trim()
        .split_whitespace()
        .map(|word| {
            let clean: String = word
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            format!("\"{}\"*", clean)
        })
        .collect::<Vec<_>>()
        .join(" ");

    let messages =
        db::messages::search_messages(&conn, &fts_query, query.channel_id.as_deref(), limit)?;
    Ok(Json(messages))
}
