use axum::{
    extract::{
        ws::{self, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::collections::HashSet;
use tokio::sync::broadcast;

use crate::db;
use crate::state::AppState;
use crate::ws::protocol::{ClientMessage, ServerMessage};

pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Wait for authentication message (with timeout)
    let auth_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(10);
    let user_id = loop {
        let remaining = auth_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            let err = ServerMessage::Error {
                code: "auth_timeout".to_string(),
                message: "Authentication timed out".to_string(),
            };
            let _ = sender
                .send(ws::Message::Text(
                    serde_json::to_string(&err).unwrap().into(),
                ))
                .await;
            return;
        }

        match tokio::time::timeout(remaining, receiver.next()).await {
            Ok(Some(Ok(ws::Message::Text(text)))) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Authenticate { token }) => {
                        // Validate session token
                        let conn = match state.db.get() {
                            Ok(c) => c,
                            Err(_) => return,
                        };
                        match crate::auth::session::validate_session(&conn, &token) {
                            Ok(Some(uid)) => break uid,
                            _ => {
                                let err = ServerMessage::Error {
                                    code: "auth_failed".to_string(),
                                    message: "Invalid or expired session".to_string(),
                                };
                                let _ = sender
                                    .send(ws::Message::Text(
                                        serde_json::to_string(&err).unwrap().into(),
                                    ))
                                    .await;
                                return;
                            }
                        }
                    }
                    _ => {
                        let err = ServerMessage::Error {
                            code: "auth_required".to_string(),
                            message: "First message must be authenticate".to_string(),
                        };
                        let _ = sender
                            .send(ws::Message::Text(
                                serde_json::to_string(&err).unwrap().into(),
                            ))
                            .await;
                        return;
                    }
                }
            }
            Ok(Some(Ok(ws::Message::Close(_)))) | Ok(None) => return,
            Err(_) => {
                // Timeout expired
                let err = ServerMessage::Error {
                    code: "auth_timeout".to_string(),
                    message: "Authentication timed out".to_string(),
                };
                let _ = sender
                    .send(ws::Message::Text(
                        serde_json::to_string(&err).unwrap().into(),
                    ))
                    .await;
                return;
            }
            _ => continue,
        }
    };

    tracing::info!("WebSocket authenticated for user {user_id}");

    // Set user online
    {
        let conn = match state.db.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("DB error setting user online: {e}");
                return;
            }
        };
        let _ = db::users::update_status(&conn, &user_id, "online");
    }

    // Send ready message with initial state
    let (user, channels, members) = {
        let conn = match state.db.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("DB error fetching initial state: {e}");
                return;
            }
        };
        let user = match db::users::get_user_by_id(&conn, &user_id) {
            Ok(Some(u)) => u,
            Ok(None) => {
                tracing::error!("User {user_id} not found after auth");
                return;
            }
            Err(e) => {
                tracing::error!("DB error fetching user: {e}");
                return;
            }
        };
        let channels = db::channels::get_all_channels(&conn).unwrap_or_default();
        let members = db::users::get_all_users(&conn).unwrap_or_default();
        (user, channels, members)
    };

    let ready = ServerMessage::Ready {
        user: user.clone(),
        channels: channels.clone(),
        members,
    };
    if sender
        .send(ws::Message::Text(
            serde_json::to_string(&ready).unwrap().into(),
        ))
        .await
        .is_err()
    {
        return;
    }

    // Broadcast presence update
    broadcast_to_all_channels(&state, &channels, &ServerMessage::PresenceUpdate {
        user_id: user_id.clone(),
        status: "online".to_string(),
    })
    .await;

    // Subscribe to all channels by default
    let mut subscribed: HashSet<String> = HashSet::new();
    let mut broadcast_receivers: Vec<(String, broadcast::Receiver<ServerMessage>)> = Vec::new();

    for channel in &channels {
        let tx = state.get_or_create_broadcast(&channel.id).await;
        broadcast_receivers.push((channel.id.clone(), tx.subscribe()));
        subscribed.insert(channel.id.clone());
    }

    // Spawn task to forward broadcast messages to this client
    let (outgoing_tx, mut outgoing_rx) = tokio::sync::mpsc::channel::<ServerMessage>(256);

    // Broadcast listener task
    let listen_tx = outgoing_tx;
    let listen_state = state.clone();
    let listen_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            let mut new_subs = Vec::new();
            for (_ch_id, rx) in broadcast_receivers.iter_mut() {
                while let Ok(msg) = rx.try_recv() {
                    // Dynamically subscribe to newly created channels
                    if let ServerMessage::ChannelCreate { ref channel } = msg {
                        if !subscribed.contains(&channel.id) {
                            let tx = listen_state.get_or_create_broadcast(&channel.id).await;
                            new_subs.push((channel.id.clone(), tx.subscribe()));
                            subscribed.insert(channel.id.clone());
                        }
                    }
                    if listen_tx.send(msg).await.is_err() {
                        return;
                    }
                }
            }
            if !new_subs.is_empty() {
                broadcast_receivers.extend(new_subs);
            }
        }
    });

    // Outgoing sender task
    let send_task = tokio::spawn(async move {
        while let Some(msg) = outgoing_rx.recv().await {
            let text = serde_json::to_string(&msg).unwrap();
            if sender.send(ws::Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    // Incoming message handler
    let incoming_state = state.clone();
    let incoming_user_id = user_id.clone();
    let incoming_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                ws::Message::Text(text) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(client_msg) => {
                            handle_client_message(
                                &incoming_state,
                                &incoming_user_id,
                                client_msg,
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::warn!("Invalid message from {}: {e}", incoming_user_id);
                        }
                    }
                }
                ws::Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for any task to finish (client disconnect)
    tokio::select! {
        _ = send_task => {},
        _ = incoming_task => {},
    }
    listen_task.abort();

    // Set user offline and broadcast with fresh channel list
    {
        match state.db.get() {
            Ok(conn) => {
                let _ = db::users::update_status(&conn, &user_id, "offline");
                let current_channels = db::channels::get_all_channels(&conn).unwrap_or_default();
                broadcast_to_all_channels(&state, &current_channels, &ServerMessage::PresenceUpdate {
                    user_id: user_id.clone(),
                    status: "offline".to_string(),
                })
                .await;
            }
            Err(e) => {
                tracing::error!("DB error setting user offline: {e}");
            }
        }
    }

    tracing::info!("WebSocket disconnected for user {user_id}");
}

async fn handle_client_message(
    state: &AppState,
    user_id: &str,
    msg: ClientMessage,
) {
    match msg {
        ClientMessage::SendMessage {
            channel_id,
            content,
            nonce,
        } => {
            let content = content.trim().to_string();
            if content.is_empty() {
                return;
            }

            let message_id = ulid::Ulid::new().to_string();
            let message = {
                let conn = match state.db.get() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("DB error: {e}");
                        return;
                    }
                };
                match db::messages::create_message(&conn, &message_id, &channel_id, user_id, &content) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::error!("Failed to create message: {e}");
                        return;
                    }
                }
            };

            // Broadcast to channel
            let tx = state.get_or_create_broadcast(&channel_id).await;
            let _ = tx.send(ServerMessage::MessageCreate {
                message,
                nonce,
            });
        }
        ClientMessage::EditMessage {
            message_id,
            content,
        } => {
            let conn = match state.db.get() {
                Ok(c) => c,
                Err(_) => return,
            };

            // Verify ownership
            if let Ok(Some(existing)) = db::messages::get_message_by_id(&conn, &message_id) {
                if existing.author_id != user_id {
                    return; // Can't edit others' messages
                }
                if let Ok(Some(updated)) = db::messages::edit_message(&conn, &message_id, &content) {
                    let tx = state.get_or_create_broadcast(&updated.channel_id).await;
                    let _ = tx.send(ServerMessage::MessageUpdate { message: updated });
                }
            }
        }
        ClientMessage::DeleteMessage { message_id } => {
            let conn = match state.db.get() {
                Ok(c) => c,
                Err(_) => return,
            };

            if let Ok(Some(existing)) = db::messages::get_message_by_id(&conn, &message_id) {
                if existing.author_id != user_id {
                    return; // Can't delete others' messages (for now)
                }
                let channel_id = existing.channel_id.clone();
                if let Ok(true) = db::messages::delete_message(&conn, &message_id) {
                    let tx = state.get_or_create_broadcast(&channel_id).await;
                    let _ = tx.send(ServerMessage::MessageDelete {
                        channel_id,
                        message_id,
                    });
                }
            }
        }
        ClientMessage::Typing { channel_id } => {
            let username = {
                let conn = match state.db.get() {
                    Ok(c) => c,
                    Err(_) => return,
                };
                db::users::get_user_by_id(&conn, user_id)
                    .ok()
                    .flatten()
                    .map(|u| u.username)
                    .unwrap_or_default()
            };

            let tx = state.get_or_create_broadcast(&channel_id).await;
            let _ = tx.send(ServerMessage::TypingStart {
                channel_id,
                user_id: user_id.to_string(),
                username,
            });
        }
        ClientMessage::AddReaction { message_id, emoji } => {
            let conn = match state.db.get() {
                Ok(c) => c,
                Err(_) => return,
            };
            if let Ok(Some(msg)) = db::messages::get_message_by_id(&conn, &message_id) {
                if db::reactions::add_reaction(&conn, &message_id, user_id, &emoji).unwrap_or(false) {
                    let reactions = db::reactions::get_reactions(&conn, &message_id).unwrap_or_default();
                    let tx = state.get_or_create_broadcast(&msg.channel_id).await;
                    let _ = tx.send(ServerMessage::ReactionUpdate {
                        channel_id: msg.channel_id,
                        message_id,
                        reactions,
                    });
                }
            }
        }
        ClientMessage::RemoveReaction { message_id, emoji } => {
            let conn = match state.db.get() {
                Ok(c) => c,
                Err(_) => return,
            };
            if let Ok(Some(msg)) = db::messages::get_message_by_id(&conn, &message_id) {
                if db::reactions::remove_reaction(&conn, &message_id, user_id, &emoji).unwrap_or(false) {
                    let reactions = db::reactions::get_reactions(&conn, &message_id).unwrap_or_default();
                    let tx = state.get_or_create_broadcast(&msg.channel_id).await;
                    let _ = tx.send(ServerMessage::ReactionUpdate {
                        channel_id: msg.channel_id,
                        message_id,
                        reactions,
                    });
                }
            }
        }
        ClientMessage::SetStatus { status } => {
            let valid = matches!(status.as_str(), "online" | "idle" | "dnd");
            if !valid { return; }

            let conn = match state.db.get() {
                Ok(c) => c,
                Err(_) => return,
            };
            let _ = db::users::update_status(&conn, user_id, &status);

            let channels = db::channels::get_all_channels(&conn).unwrap_or_default();
            broadcast_to_all_channels(state, &channels, &ServerMessage::PresenceUpdate {
                user_id: user_id.to_string(),
                status,
            })
            .await;
        }
        ClientMessage::Authenticate { .. } => {
            // Already authenticated, ignore
        }
        ClientMessage::Subscribe { .. } | ClientMessage::Unsubscribe { .. } => {
            // TODO: Dynamic subscription management
        }
    }
}

/// Broadcast a message to all channels. Deduplicates by channel ID to avoid
/// sending the same message multiple times through shared broadcast senders.
async fn broadcast_to_all_channels(
    state: &AppState,
    channels: &[db::channels::Channel],
    msg: &ServerMessage,
) {
    let mut seen = HashSet::new();
    for channel in channels {
        if seen.insert(&channel.id) {
            let tx = state.get_or_create_broadcast(&channel.id).await;
            let _ = tx.send(msg.clone());
        }
    }
}
