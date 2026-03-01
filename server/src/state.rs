use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::ws::protocol::ServerMessage;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Per-channel broadcast sender for real-time message delivery
pub type ChannelBroadcast = broadcast::Sender<ServerMessage>;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    /// Map of channel_id -> broadcast sender
    pub channels: Arc<RwLock<HashMap<String, ChannelBroadcast>>>,
    /// Directory for file uploads
    pub uploads_dir: PathBuf,
}

impl AppState {
    pub fn new(db: DbPool, uploads_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&uploads_dir).expect("Failed to create uploads directory");
        Self {
            db,
            channels: Arc::new(RwLock::new(HashMap::new())),
            uploads_dir,
        }
    }

    /// Get or create a broadcast channel for a given channel ID
    pub async fn get_or_create_broadcast(&self, channel_id: &str) -> ChannelBroadcast {
        // Check if it exists first (read lock)
        {
            let channels = self.channels.read().await;
            if let Some(tx) = channels.get(channel_id) {
                return tx.clone();
            }
        }

        // Create it (write lock)
        let mut channels = self.channels.write().await;
        // Double-check after acquiring write lock
        if let Some(tx) = channels.get(channel_id) {
            return tx.clone();
        }

        let (tx, _) = broadcast::channel(1024);
        channels.insert(channel_id.to_string(), tx.clone());
        tx
    }
}
