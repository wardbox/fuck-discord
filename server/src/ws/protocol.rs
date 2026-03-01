use serde::{Deserialize, Serialize};

use crate::db::{channels::Channel, messages::Message, reactions::Reaction, users::User};

// === Client → Server messages ===

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Authenticate {
        token: String,
    },
    SendMessage {
        channel_id: String,
        content: String,
        nonce: Option<String>,
    },
    EditMessage {
        message_id: String,
        content: String,
    },
    DeleteMessage {
        message_id: String,
    },
    Typing {
        channel_id: String,
    },
    AddReaction {
        message_id: String,
        emoji: String,
    },
    RemoveReaction {
        message_id: String,
        emoji: String,
    },
    SetStatus {
        status: String,
    },
    Subscribe {
        channel_ids: Vec<String>,
    },
    Unsubscribe {
        channel_ids: Vec<String>,
    },
}

// === Server → Client messages ===

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Ready {
        user: User,
        channels: Vec<Channel>,
        members: Vec<User>,
    },
    MessageCreate {
        message: Message,
        nonce: Option<String>,
    },
    MessageUpdate {
        message: Message,
    },
    MessageDelete {
        channel_id: String,
        message_id: String,
    },
    TypingStart {
        channel_id: String,
        user_id: String,
        username: String,
    },
    PresenceUpdate {
        user_id: String,
        status: String,
    },
    ChannelCreate {
        channel: Channel,
    },
    ChannelUpdate {
        channel: Channel,
    },
    ChannelDelete {
        channel_id: String,
    },
    MemberJoin {
        user: User,
    },
    MemberLeave {
        user_id: String,
    },
    ReactionUpdate {
        channel_id: String,
        message_id: String,
        reactions: Vec<Reaction>,
    },
    Error {
        code: String,
        message: String,
    },
}
