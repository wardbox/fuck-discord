use axum::{extract::State, Extension, Json};
use serde::Deserialize;

use crate::auth::middleware::AuthUser;
use crate::db;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub async fn get_me(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> AppResult<Json<db::users::User>> {
    let conn = state.db.get()?;
    let user =
        db::users::get_user_by_id(&conn, &auth_user.0)?.ok_or(AppError::NotFound)?;
    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub status: Option<String>,
}

pub async fn update_me(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<UpdateUserRequest>,
) -> AppResult<Json<db::users::User>> {
    let conn = state.db.get()?;

    if let Some(display_name) = &req.display_name {
        let trimmed = display_name.trim();
        if trimmed.is_empty() || trimmed.len() > 64 {
            return Err(AppError::BadRequest(
                "Display name must be 1-64 characters".to_string(),
            ));
        }
        conn.execute(
            "UPDATE users SET display_name = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![trimmed, auth_user.0],
        )?;
    }

    if let Some(status) = &req.status {
        let valid = ["online", "idle", "dnd", "offline"];
        if !valid.contains(&status.as_str()) {
            return Err(AppError::BadRequest("Invalid status".to_string()));
        }
        db::users::update_status(&conn, &auth_user.0, status)?;
    }

    let user =
        db::users::get_user_by_id(&conn, &auth_user.0)?.ok_or(AppError::NotFound)?;
    Ok(Json(user))
}
