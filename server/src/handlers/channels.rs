use axum::{extract::{Path, State}, Extension, Json};
use serde::Deserialize;

use crate::auth::middleware::AuthUser;
use crate::db;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub async fn list_channels(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> AppResult<Json<Vec<db::channels::Channel>>> {
    let conn = state.db.get()?;
    let channels = db::channels::get_all_channels(&conn)?;
    Ok(Json(channels))
}

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub topic: Option<String>,
    pub category: Option<String>,
}

pub async fn create_channel(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Json(req): Json<CreateChannelRequest>,
) -> AppResult<Json<db::channels::Channel>> {
    let name = req.name.trim().to_lowercase().replace(' ', "-");
    if name.is_empty() || name.len() > 100 {
        return Err(crate::error::AppError::BadRequest(
            "Channel name must be 1-100 characters".to_string(),
        ));
    }

    let conn = state.db.get()?;
    let id = ulid::Ulid::new().to_string();
    let channel = db::channels::create_channel(
        &conn,
        &id,
        &name,
        req.topic.as_deref(),
        req.category.as_deref(),
    )?;

    // Broadcast channel creation to all connected users
    let channels = db::channels::get_all_channels(&conn)?;
    for ch in &channels {
        let tx = state.get_or_create_broadcast(&ch.id).await;
        let _ = tx.send(crate::ws::protocol::ServerMessage::ChannelCreate {
            channel: channel.clone(),
        });
    }

    Ok(Json(channel))
}

fn deserialize_optional_field<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

#[derive(Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub topic: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub category: Option<Option<String>>,
}

pub async fn update_channel(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(channel_id): Path<String>,
    Json(req): Json<UpdateChannelRequest>,
) -> AppResult<Json<db::channels::Channel>> {
    let conn = state.db.get()?;

    let name = req.name.as_ref().map(|n| {
        n.trim().to_lowercase().replace(' ', "-")
    });
    if let Some(ref name) = name {
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::BadRequest("Channel name must be 1-100 characters".to_string()));
        }
    }

    let channel = db::channels::update_channel(
        &conn,
        &channel_id,
        name.as_deref(),
        req.topic.as_ref().map(|t| t.as_deref()),
        req.category.as_ref().map(|c| c.as_deref()),
    )?
    .ok_or(AppError::NotFound)?;

    // Broadcast channel update
    let channels = db::channels::get_all_channels(&conn)?;
    for ch in &channels {
        let tx = state.get_or_create_broadcast(&ch.id).await;
        let _ = tx.send(crate::ws::protocol::ServerMessage::ChannelUpdate {
            channel: channel.clone(),
        });
    }

    Ok(Json(channel))
}

pub async fn delete_channel(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(channel_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let conn = state.db.get()?;

    // Broadcast to the deleted channel's broadcaster *before* removing it,
    // so connected clients on that channel receive the deletion event.
    // Also broadcast to all other channels so all clients are notified.
    let channels_before_delete = db::channels::get_all_channels(&conn)?;

    if !db::channels::delete_channel(&conn, &channel_id)? {
        return Err(AppError::NotFound);
    }

    // Broadcast channel deletion to all channels (including the deleted one's broadcaster)
    for ch in &channels_before_delete {
        let tx = state.get_or_create_broadcast(&ch.id).await;
        let _ = tx.send(crate::ws::protocol::ServerMessage::ChannelDelete {
            channel_id: channel_id.clone(),
        });
    }

    Ok(Json(serde_json::json!({"ok": true})))
}
