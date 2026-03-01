use reqwest::Client;
use serde_json::Value;
use std::net::SocketAddr;
use tempfile::TempDir;
use tokio::net::TcpListener;

/// A test server instance with its own database and uploads directory.
pub struct TestServer {
    pub addr: SocketAddr,
    pub base_url: String,
    pub invite_code: String,
    _temp_dir: TempDir,
}

impl TestServer {
    /// Spin up a fresh server on a random port with a temporary database.
    pub async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let uploads_dir = temp_dir.path().join("uploads");

        let pool = relay_server::db::create_pool(db_path.to_str().unwrap())
            .expect("Failed to create pool");
        relay_server::db::run_migrations(&pool).expect("Failed to run migrations");

        // Seed default channels
        {
            let conn = pool.get().unwrap();
            relay_server::db::channels::seed_defaults(&conn).unwrap();
        }

        let state = relay_server::state::AppState::new(pool.clone(), uploads_dir);
        let router = relay_server::handlers::router(state);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        // Create a bootstrap invite code directly in the DB
        let invite_code = {
            let conn = pool.get().unwrap();
            // Need a bootstrap user to create invite — create one directly
            let user_id = ulid::Ulid::new().to_string();
            conn.execute(
                "INSERT INTO users (id, username, password_hash, created_at, updated_at) VALUES (?1, ?2, ?3, datetime('now'), datetime('now'))",
                rusqlite::params![user_id, "__bootstrap__", "unused"],
            ).unwrap();
            relay_server::auth::invite::create_invite_code(&conn, &user_id, None, None).unwrap()
        };

        let base_url = format!("http://{addr}");

        TestServer {
            addr,
            base_url,
            invite_code,
            _temp_dir: temp_dir,
        }
    }

    /// Create a new invite code (requires a registered user's session).
    pub async fn create_invite(&self, client: &TestClient) -> String {
        let res = client
            .post(&format!("{}/api/invites", self.base_url))
            .json(&serde_json::json!({"max_uses": null}))
            .send()
            .await
            .unwrap();
        let body: Value = res.json().await.unwrap();
        body["code"].as_str().unwrap().to_string()
    }
}

/// HTTP test client with automatic cookie handling.
pub struct TestClient {
    pub client: Client,
    pub base_url: String,
    pub session_id: Option<String>,
}

impl TestClient {
    pub fn new(server: &TestServer) -> Self {
        let client = Client::builder()
            .cookie_store(true)
            .build()
            .unwrap();
        TestClient {
            client,
            base_url: server.base_url.clone(),
            session_id: None,
        }
    }

    pub fn post(&self, url: &str) -> reqwest::RequestBuilder {
        self.client.post(url)
    }

    pub fn get(&self, url: &str) -> reqwest::RequestBuilder {
        self.client.get(url)
    }

    pub fn patch(&self, url: &str) -> reqwest::RequestBuilder {
        self.client.patch(url)
    }

    pub fn delete(&self, url: &str) -> reqwest::RequestBuilder {
        self.client.delete(url)
    }

    /// Register a new user, stores session_id.
    pub async fn register(&mut self, server: &TestServer, username: &str, password: &str) -> Value {
        let res = self
            .post(&format!("{}/api/auth/register", self.base_url))
            .json(&serde_json::json!({
                "username": username,
                "password": password,
                "invite_code": server.invite_code,
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200, "Registration failed");
        let body: Value = res.json().await.unwrap();
        self.session_id = body["session_id"].as_str().map(|s| s.to_string());
        body
    }

    /// Register with a specific invite code.
    pub async fn register_with_invite(
        &mut self,
        invite_code: &str,
        username: &str,
        password: &str,
    ) -> reqwest::Response {
        self.post(&format!("{}/api/auth/register", self.base_url))
            .json(&serde_json::json!({
                "username": username,
                "password": password,
                "invite_code": invite_code,
            }))
            .send()
            .await
            .unwrap()
    }

    /// Login with credentials.
    pub async fn login(&mut self, username: &str, password: &str) -> reqwest::Response {
        let res = self
            .post(&format!("{}/api/auth/login", self.base_url))
            .json(&serde_json::json!({
                "username": username,
                "password": password,
            }))
            .send()
            .await
            .unwrap();
        if res.status() == 200 {
            self.session_id = res
                .headers()
                .get(reqwest::header::SET_COOKIE)
                .and_then(|h| h.to_str().ok())
                .and_then(|cookie| {
                    cookie
                        .split(';')
                        .find_map(|p| p.trim().strip_prefix("relay_session="))
                })
                .map(str::to_string);
        }
        res
    }

    /// Get channels list.
    pub async fn get_channels(&self) -> Vec<Value> {
        let res = self
            .get(&format!("{}/api/channels", self.base_url))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        res.json().await.unwrap()
    }

    /// Create a channel.
    pub async fn create_channel(&self, name: &str, category: Option<&str>) -> Value {
        let mut body = serde_json::json!({"name": name});
        if let Some(cat) = category {
            body["category"] = Value::String(cat.to_string());
        }
        let res = self
            .post(&format!("{}/api/channels", self.base_url))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200, "Channel creation failed");
        res.json().await.unwrap()
    }

    /// Get messages for a channel.
    pub async fn get_messages(&self, channel_id: &str) -> Vec<Value> {
        let res = self
            .get(&format!(
                "{}/api/channels/{}/messages",
                self.base_url, channel_id
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        res.json().await.unwrap()
    }

    /// Search messages.
    pub async fn search(&self, query: &str) -> Vec<Value> {
        let res = self
            .get(&format!("{}/api/search", self.base_url))
            .query(&[("q", query)])
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        res.json().await.unwrap()
    }
}

/// WebSocket test client for real-time testing.
pub struct WsClient {
    write: futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        tokio_tungstenite::tungstenite::Message,
    >,
    read: futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
}

impl WsClient {
    /// Connect to the WebSocket endpoint.
    pub async fn connect(server: &TestServer) -> Self {
        let ws_url = format!("ws://{}/ws", server.addr);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .expect("Failed to connect WebSocket");

        use futures::StreamExt;
        let (write, read) = ws_stream.split();

        WsClient { write, read }
    }

    /// Authenticate with a session token.
    pub async fn authenticate(&mut self, session_id: &str) -> Value {
        self.send_json(&serde_json::json!({
            "type": "authenticate",
            "token": session_id,
        }))
        .await;

        // Should receive a "ready" message
        let msg = self.recv().await.expect("Expected ready message");
        assert_eq!(msg["type"], "ready", "Expected ready message, got: {msg}");
        msg
    }

    /// Send a JSON message.
    pub async fn send_json(&mut self, value: &Value) {
        use futures::SinkExt;
        use tokio_tungstenite::tungstenite::Message;
        let text = serde_json::to_string(value).unwrap();
        self.write.send(Message::Text(text.into())).await.unwrap();
    }

    /// Send a chat message.
    pub async fn send_message(&mut self, channel_id: &str, content: &str) {
        self.send_json(&serde_json::json!({
            "type": "send_message",
            "channel_id": channel_id,
            "content": content,
        }))
        .await;
    }

    /// Send an edit message command.
    pub async fn edit_message(&mut self, message_id: &str, content: &str) {
        self.send_json(&serde_json::json!({
            "type": "edit_message",
            "message_id": message_id,
            "content": content,
        }))
        .await;
    }

    /// Send a delete message command.
    pub async fn delete_message(&mut self, message_id: &str) {
        self.send_json(&serde_json::json!({
            "type": "delete_message",
            "message_id": message_id,
        }))
        .await;
    }

    /// Send a typing indicator.
    pub async fn send_typing(&mut self, channel_id: &str) {
        self.send_json(&serde_json::json!({
            "type": "typing",
            "channel_id": channel_id,
        }))
        .await;
    }

    /// Add a reaction.
    pub async fn add_reaction(&mut self, message_id: &str, emoji: &str) {
        self.send_json(&serde_json::json!({
            "type": "add_reaction",
            "message_id": message_id,
            "emoji": emoji,
        }))
        .await;
    }

    /// Receive the next JSON message, with a timeout.
    pub async fn recv(&mut self) -> Option<Value> {
        use futures::StreamExt;
        use tokio_tungstenite::tungstenite::Message;

        let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                match self.read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        return Some(serde_json::from_str::<Value>(&text).unwrap());
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
                    Some(Ok(Message::Close(_))) | None => return None,
                    Some(Err(e)) => {
                        eprintln!("WS recv error: {e}");
                        return None;
                    }
                    _ => continue,
                }
            }
        });

        match timeout.await {
            Ok(result) => result,
            Err(_) => None, // Timeout
        }
    }

    /// Receive messages until we find one matching the given type.
    pub async fn recv_type(&mut self, msg_type: &str) -> Option<Value> {
        for _ in 0..20 {
            if let Some(msg) = self.recv().await {
                if msg["type"].as_str() == Some(msg_type) {
                    return Some(msg);
                }
            } else {
                return None;
            }
        }
        None
    }
}
