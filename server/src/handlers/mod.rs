pub mod auth;
pub mod channels;
pub mod messages;
pub mod uploads;
pub mod users;

use axum::{
    response::IntoResponse,
    Router,
    middleware,
    routing::{delete, get, post, patch},
};
use rust_embed::RustEmbed;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::auth::middleware::require_auth;
use crate::state::AppState;
use crate::ws;

#[derive(RustEmbed)]
#[folder = "../client/build"]
#[prefix = ""]
struct ClientAssets;

pub fn router(state: AppState) -> Router {
    let api_public = Router::new()
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login));

    let api_protected = Router::new()
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route("/api/channels", get(channels::list_channels))
        .route("/api/channels", post(channels::create_channel))
        .route("/api/channels/{channel_id}", patch(channels::update_channel))
        .route("/api/channels/{channel_id}", delete(channels::delete_channel))
        .route(
            "/api/channels/{channel_id}/messages",
            get(messages::get_messages),
        )
        .route("/api/search", get(messages::search))
        .route("/api/users/me", get(users::get_me))
        .route("/api/users/me", patch(users::update_me))
        .route("/api/invites", post(auth::create_invite))
        .route("/api/invites", get(auth::list_invites))
        .route("/api/upload", post(uploads::upload_file))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let upload_serve = Router::new()
        .route("/uploads/{filename}", get(uploads::serve_upload))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let ws_route = Router::new().route("/ws", get(ws::handler::ws_upgrade));

    Router::new()
        .merge(api_public)
        .merge(api_protected)
        .merge(upload_serve)
        .merge(ws_route)
        .fallback(serve_client)
        .layer(if cfg!(debug_assertions) {
            // In dev, allow cross-origin requests for separate dev servers
            CorsLayer::permissive()
        } else {
            // In release, client is embedded — all requests are same-origin
            CorsLayer::new()
        })
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Serve embedded client files, falling back to index.html for SPA routing
async fn serve_client(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Don't serve index.html for API, WS, or upload paths
    if path == "api" || path.starts_with("api/") || path == "ws" || path.starts_with("uploads/") {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    }

    // Try to serve the exact file
    if let Some(file) = ClientAssets::get(path) {
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        return (
            [(axum::http::header::CONTENT_TYPE, mime)],
            file.data.to_vec(),
        )
            .into_response();
    }

    // Fall back to index.html for SPA routing
    if let Some(file) = ClientAssets::get("index.html") {
        return (
            [(
                axum::http::header::CONTENT_TYPE,
                "text/html".to_string(),
            )],
            file.data.to_vec(),
        )
            .into_response();
    }

    axum::http::StatusCode::NOT_FOUND.into_response()
}
