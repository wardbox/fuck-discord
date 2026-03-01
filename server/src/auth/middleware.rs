use axum::{
    extract::{Request, State},
    http::header::COOKIE,
    middleware::Next,
    response::Response,
};

use crate::error::AppError;
use crate::state::AppState;

/// Extract user ID from session cookie
pub fn extract_session_id(request: &Request) -> Option<String> {
    let cookie_header = request.headers().get(COOKIE)?.to_str().ok()?;
    cookie_header
        .split(';')
        .find_map(|cookie| {
            let cookie = cookie.trim();
            if let Some(value) = cookie.strip_prefix("relay_session=") {
                Some(value.to_string())
            } else {
                None
            }
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
