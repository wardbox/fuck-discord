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

    // Wait for authentication message
    let user_id = loop {
        match receiver.next().await {
            Some(Ok(ws::Message::Text(text))) => {
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
            Some(Ok(ws::Message::Close(_))) | None => return,
            _ => continue,
        }
    };

    tracing::info!("WebSocket authenticated for user {user_id}");

    // Set user online
    {
        let conn = state.db.get().unwrap();
        let _ = db::users::update_status(&conn, &user_id, "online");
    }

    // Send ready message with initial state
    let (user, channels, members) = {
        let conn = match state.db.get() {
            Ok(c) => c,
            Err(_) => return,
        };
        let user = db::users::get_user_by_id(&conn, &user_id).unwrap().unwrap();
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
    let outgoing_tx_clone = outgoing_tx.clone();
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
                        let tx = listen_state.get_or_create_broadcast(&channel.id).await;
                        new_subs.push((channel.id.clone(), tx.subscribe()));
                    }
                    if outgoing_tx_clone.send(msg).await.is_err() {
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
                                &outgoing_tx,
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

    // Set user offline
    {
        let conn = state.db.get().unwrap();
        let _ = db::users::update_status(&conn, &user_id, "offline");
    }

    // Broadcast offline presence
    broadcast_to_all_channels(&state, &channels, &ServerMessage::PresenceUpdate {
        user_id: user_id.clone(),
        status: "offline".to_string(),
    })
    .await;

    tracing::info!("WebSocket disconnected for user {user_id}");
}

async fn handle_client_message(
    state: &AppState,
    user_id: &str,
    msg: ClientMessage,
    _outgoing: &tokio::sync::mpsc::Sender<ServerMessage>,
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

            // Broadcast presence change
            let channels = db::channels::get_all_channels(&conn).unwrap_or_default();
            for ch in &channels {
                let tx = state.get_or_create_broadcast(&ch.id).await;
                let _ = tx.send(ServerMessage::PresenceUpdate {
                    user_id: user_id.to_string(),
                    status: status.clone(),
                });
            }
        }
        ClientMessage::Authenticate { .. } => {
            // Already authenticated, ignore
        }
        ClientMessage::Subscribe { .. } | ClientMessage::Unsubscribe { .. } => {
            // TODO: Dynamic subscription management
        }
    }
}

async fn broadcast_to_all_channels(
    state: &AppState,
    channels: &[db::channels::Channel],
    msg: &ServerMessage,
) {
    for channel in channels {
        let tx = state.get_or_create_broadcast(&channel.id).await;
        let _ = tx.send(msg.clone());
    }
}
