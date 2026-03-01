use axum::{
    extract::State,
    http::header::SET_COOKIE,
    Extension, Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::{self, middleware::AuthUser};
use crate::db;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub invite_code: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: db::users::User,
    pub session_id: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<axum::response::Response> {
    // Validate input
    let username = req.username.trim();
    if username.len() < 2 || username.len() > 32 {
        return Err(AppError::BadRequest(
            "Username must be 2-32 characters".to_string(),
        ));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    let conn = state.db.get()?;

    // Validate invite
    if !db::invites::validate_and_use_invite(&conn, &req.invite_code)? {
        return Err(AppError::BadRequest("Invalid or expired invite code".to_string()));
    }

    // Check username uniqueness
    if db::users::get_user_by_username(&conn, username)?.is_some() {
        return Err(AppError::Conflict("Username already taken".to_string()));
    }

    // Hash password
    let password_hash = auth::password::hash_password(&req.password)
        .map_err(|e| AppError::Internal(format!("Password hashing failed: {e}")))?;

    // Create user
    let user_id = ulid::Ulid::new().to_string();
    let user = db::users::create_user(&conn, &user_id, username, &password_hash)?;

    // Auto-join all channels
    let channels = db::channels::get_all_channels(&conn)?;
    for channel in &channels {
        conn.execute(
            "INSERT OR IGNORE INTO channel_members (channel_id, user_id) VALUES (?1, ?2)",
            rusqlite::params![channel.id, user_id],
        )?;
    }

    // Create session
    let session_id = auth::session::create_session(&conn, &user_id)?;

    let response = AuthResponse {
        user,
        session_id: session_id.clone(),
    };

    let cookie = format!(
        "relay_session={session_id}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        30 * 24 * 60 * 60 // 30 days
    );

    Ok((
        [(SET_COOKIE, cookie)],
        Json(response),
    )
        .into_response())
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<axum::response::Response> {
    let conn = state.db.get()?;

    let user_with_auth = db::users::get_user_by_username(&conn, &req.username)?
        .ok_or(AppError::Unauthorized)?;

    let valid = auth::password::verify_password(&req.password, &user_with_auth.password_hash)
        .map_err(|e| AppError::Internal(format!("Password verification failed: {e}")))?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    let session_id = auth::session::create_session(&conn, &user_with_auth.user.id)?;

    let response = AuthResponse {
        user: user_with_auth.user,
        session_id: session_id.clone(),
    };

    let cookie = format!(
        "relay_session={session_id}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        30 * 24 * 60 * 60
    );

    Ok((
        [(SET_COOKIE, cookie)],
        Json(response),
    )
        .into_response())
}

pub async fn logout(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> AppResult<axum::response::Response> {
    if let Some(session_id) = crate::auth::middleware::extract_session_id(&request) {
        let conn = state.db.get()?;
        auth::session::delete_session(&conn, &session_id)?;
    }

    let cookie = "relay_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";

    Ok((
        [(SET_COOKIE, cookie.to_string())],
        Json(serde_json::json!({"ok": true})),
    )
        .into_response())
}

pub async fn me(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> AppResult<Json<db::users::User>> {
    let conn = state.db.get()?;
    let user = db::users::get_user_by_id(&conn, &auth_user.0)?
        .ok_or(AppError::NotFound)?;
    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct CreateInviteRequest {
    pub max_uses: Option<i64>,
}

#[derive(Serialize)]
pub struct InviteResponse {
    pub code: String,
}

pub async fn create_invite(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateInviteRequest>,
) -> AppResult<Json<InviteResponse>> {
    let conn = state.db.get()?;
    let code = auth::invite::create_invite_code(&conn, &auth_user.0, req.max_uses, None)?;
    Ok(Json(InviteResponse { code }))
}

pub async fn list_invites(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> AppResult<Json<serde_json::Value>> {
    let conn = state.db.get()?;
    let invites = db::invites::get_all_invites(&conn)?;
    let invites: Vec<serde_json::Value> = invites
        .into_iter()
        .map(|(code, created_by, max_uses, uses, expires_at, created_at)| {
            serde_json::json!({
                "code": code,
                "created_by": created_by,
                "max_uses": max_uses,
                "uses": uses,
                "expires_at": expires_at,
                "created_at": created_at,
            })
        })
        .collect();
    Ok(Json(serde_json::json!(invites)))
}

use axum::response::IntoResponse;
