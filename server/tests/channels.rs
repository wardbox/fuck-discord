mod common;

use common::{TestClient, TestServer};

#[tokio::test]
async fn default_general_channel_exists() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let channels = client.get_channels().await;
    let names: Vec<&str> = channels
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert!(
        names.contains(&"general"),
        "Expected #general channel, got: {names:?}"
    );
}

#[tokio::test]
async fn create_channel() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let channel = client.create_channel("dev-talk", None).await;
    assert_eq!(channel["name"], "dev-talk");
    assert!(channel["id"].is_string());

    // Verify it appears in the channel list
    let channels = client.get_channels().await;
    let names: Vec<&str> = channels
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert!(names.contains(&"dev-talk"));
}

#[tokio::test]
async fn create_channel_with_category() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let channel = client.create_channel("rust-help", Some("Development")).await;
    assert_eq!(channel["name"], "rust-help");
    assert_eq!(channel["category"], "Development");
}

#[tokio::test]
async fn create_channel_name_normalized() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let channel = client.create_channel("My Cool Channel", None).await;
    assert_eq!(channel["name"], "my-cool-channel");
}

#[tokio::test]
async fn create_channel_empty_name_rejected() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let res = client
        .post(&format!("{}/api/channels", server.base_url))
        .json(&serde_json::json!({"name": "   "}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn update_channel_topic() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let channel = client.create_channel("announcements", None).await;
    let channel_id = channel["id"].as_str().unwrap();

    let res = client
        .patch(&format!("{}/api/channels/{}", server.base_url, channel_id))
        .json(&serde_json::json!({"topic": "Important announcements only"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let updated: serde_json::Value = res.json().await.unwrap();
    assert_eq!(updated["topic"], "Important announcements only");
}

#[tokio::test]
async fn update_channel_name() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let channel = client.create_channel("old-name", None).await;
    let channel_id = channel["id"].as_str().unwrap();

    let res = client
        .patch(&format!("{}/api/channels/{}", server.base_url, channel_id))
        .json(&serde_json::json!({"name": "New Name"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let updated: serde_json::Value = res.json().await.unwrap();
    assert_eq!(updated["name"], "new-name");
}

#[tokio::test]
async fn update_channel_category() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let channel = client.create_channel("random", None).await;
    let channel_id = channel["id"].as_str().unwrap();

    // Set category
    let res = client
        .patch(&format!("{}/api/channels/{}", server.base_url, channel_id))
        .json(&serde_json::json!({"category": "Social"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let updated: serde_json::Value = res.json().await.unwrap();
    assert_eq!(updated["category"], "Social");

    // Clear category
    let res = client
        .patch(&format!("{}/api/channels/{}", server.base_url, channel_id))
        .json(&serde_json::json!({"category": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let updated: serde_json::Value = res.json().await.unwrap();
    assert!(updated["category"].is_null());
}

#[tokio::test]
async fn delete_channel() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let channel = client.create_channel("to-delete", None).await;
    let channel_id = channel["id"].as_str().unwrap();

    let res = client
        .delete(&format!("{}/api/channels/{}", server.base_url, channel_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // Verify it's gone
    let channels = client.get_channels().await;
    let ids: Vec<&str> = channels
        .iter()
        .filter_map(|c| c["id"].as_str())
        .collect();
    assert!(!ids.contains(&channel_id));
}

#[tokio::test]
async fn delete_nonexistent_channel_returns_404() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let res = client
        .delete(&format!("{}/api/channels/nonexistent-id", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}
