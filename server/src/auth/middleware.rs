use axum::{
    extract::{Request, State},
    http::header::{AUTHORIZATION, COOKIE},
    middleware::Next,
    response::Response,
};

use crate::error::AppError;
use crate::state::AppState;

/// Extract session ID from Authorization header (Bearer) or session cookie
pub fn extract_session_id(request: &Request) -> Option<String> {
    // Try Authorization: Bearer header first (Tauri/mobile clients)
    if let Some(auth_header) = request.headers().get(AUTHORIZATION) {
        if let Ok(value) = auth_header.to_str() {
            if let Some(token) = value.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    // Fall back to cookie (browser clients)
    let cookie_header = request.headers().get(COOKIE)?.to_str().ok()?;
    cookie_header
        .split(';')
        .find_map(|cookie| {
            let cookie = cookie.trim();
            cookie.strip_prefix("relay_session=").map(|v| v.to_string())
        })
}

/// Middleware that requires authentication
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let session_id = extract_session_id(&request).ok_or(AppError::Unauthorized)?;

    let conn = state.db.get()?;
    let user_id = crate::auth::session::validate_session(&conn, &session_id)?
        .ok_or(AppError::Unauthorized)?;

    // Store user_id in request extensions for handlers to use
    request.extensions_mut().insert(AuthUser(user_id));

    Ok(next.run(request).await)
}

/// Extracted authenticated user ID
#[derive(Clone, Debug)]
pub struct AuthUser(pub String);
