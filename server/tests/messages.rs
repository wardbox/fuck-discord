mod common;

use common::{TestClient, TestServer, WsClient};

#[tokio::test]
async fn send_message_via_ws_appears_in_history() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    let reg = client.register(&server, "alice", "password123").await;
    let session_id = reg["session_id"].as_str().unwrap();

    // Get the general channel
    let channels = client.get_channels().await;
    let general = channels
        .iter()
        .find(|c| c["name"] == "general")
        .expect("No general channel");
    let channel_id = general["id"].as_str().unwrap();

    // Send message via WebSocket
    let mut ws = WsClient::connect(&server).await;
    ws.authenticate(session_id).await;
    ws.send_message(channel_id, "Hello from integration test!")
        .await;

    // Wait for broadcast
    let msg = ws.recv_type("message_create").await;
    assert!(msg.is_some(), "Expected message_create event");
    let msg = msg.unwrap();
    assert_eq!(msg["message"]["content"], "Hello from integration test!");
    assert_eq!(msg["message"]["author_username"], "alice");

    // Verify via REST history
    let messages = client.get_messages(channel_id).await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["content"], "Hello from integration test!");
}

#[tokio::test]
async fn edit_message_via_ws() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    let reg = client.register(&server, "alice", "password123").await;
    let session_id = reg["session_id"].as_str().unwrap();

    let channels = client.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    let mut ws = WsClient::connect(&server).await;
    ws.authenticate(session_id).await;
    ws.send_message(channel_id, "original text").await;

    let create = ws.recv_type("message_create").await.unwrap();
    let message_id = create["message"]["id"].as_str().unwrap();

    // Edit the message
    ws.edit_message(message_id, "edited text").await;

    let update = ws.recv_type("message_update").await;
    assert!(update.is_some(), "Expected message_update event");
    let update = update.unwrap();
    assert_eq!(update["message"]["content"], "edited text");
    assert!(update["message"]["edited_at"].is_string());

    // Verify via REST
    let messages = client.get_messages(channel_id).await;
    assert_eq!(messages[0]["content"], "edited text");
}

#[tokio::test]
async fn delete_message_via_ws() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    let reg = client.register(&server, "alice", "password123").await;
    let session_id = reg["session_id"].as_str().unwrap();

    let channels = client.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    let mut ws = WsClient::connect(&server).await;
    ws.authenticate(session_id).await;
    ws.send_message(channel_id, "delete me").await;

    let create = ws.recv_type("message_create").await.unwrap();
    let message_id = create["message"]["id"].as_str().unwrap();

    // Delete the message
    ws.delete_message(message_id).await;

    let delete = ws.recv_type("message_delete").await;
    assert!(delete.is_some(), "Expected message_delete event");
    let delete = delete.unwrap();
    assert_eq!(delete["message_id"], message_id);

    // Verify via REST — should be empty
    let messages = client.get_messages(channel_id).await;
    assert!(messages.is_empty(), "Message should be deleted");
}

#[tokio::test]
async fn search_exact_match() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    let reg = client.register(&server, "alice", "password123").await;
    let session_id = reg["session_id"].as_str().unwrap();

    let channels = client.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    let mut ws = WsClient::connect(&server).await;
    ws.authenticate(session_id).await;
    ws.send_message(channel_id, "unique search term xylophone")
        .await;
    ws.recv_type("message_create").await;

    let results = client.search("xylophone").await;
    assert_eq!(results.len(), 1);
    assert!(results[0]["content"]
        .as_str()
        .unwrap()
        .contains("xylophone"));
}

#[tokio::test]
async fn search_prefix_matching() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    let reg = client.register(&server, "alice", "password123").await;
    let session_id = reg["session_id"].as_str().unwrap();

    let channels = client.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    let mut ws = WsClient::connect(&server).await;
    ws.authenticate(session_id).await;

    ws.send_message(channel_id, "I am testing the search feature")
        .await;
    ws.recv_type("message_create").await;

    ws.send_message(channel_id, "This is a test message").await;
    ws.recv_type("message_create").await;

    // "test" should match both "testing" and "test"
    let results = client.search("test").await;
    assert_eq!(
        results.len(),
        2,
        "Expected 2 results for prefix search 'test', got: {}",
        results.len()
    );
}

#[tokio::test]
async fn search_no_results() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let results = client.search("nonexistenttermxyz").await;
    assert!(results.is_empty());
}

#[tokio::test]
async fn search_empty_query_rejected() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let res = client
        .get(&format!("{}/api/search", server.base_url))
        .query(&[("q", "   ")])
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn message_history_pagination() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    let reg = client.register(&server, "alice", "password123").await;
    let session_id = reg["session_id"].as_str().unwrap();

    let channels = client.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    let mut ws = WsClient::connect(&server).await;
    ws.authenticate(session_id).await;

    // Send 5 messages
    for i in 0..5 {
        ws.send_message(channel_id, &format!("message {i}")).await;
        ws.recv_type("message_create").await;
    }

    // Get all messages (default limit)
    let messages = client.get_messages(channel_id).await;
    assert_eq!(messages.len(), 5);

    // Get messages with limit
    let res = client
        .get(&format!(
            "{}/api/channels/{}/messages?limit=2",
            server.base_url, channel_id
        ))
        .send()
        .await
        .unwrap();
    let page: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(page.len(), 2);

    // Get messages before a cursor (use oldest message in page as the "before" cursor)
    let cursor = page.first().unwrap()["id"].as_str().unwrap();
    let res = client
        .get(&format!(
            "{}/api/channels/{}/messages?before={}&limit=2",
            server.base_url, channel_id, cursor
        ))
        .send()
        .await
        .unwrap();
    let page2: Vec<serde_json::Value> = res.json().await.unwrap();
    assert_eq!(page2.len(), 2);

    // Ensure pages don't overlap
    let page1_ids: Vec<&str> = page.iter().filter_map(|m| m["id"].as_str()).collect();
    let page2_ids: Vec<&str> = page2.iter().filter_map(|m| m["id"].as_str()).collect();
    for id in &page2_ids {
        assert!(
            !page1_ids.contains(id),
            "Pagination overlap: {id} in both pages"
        );
    }
}

#[tokio::test]
async fn cannot_edit_others_messages() {
    let server = TestServer::new().await;

    // Alice sends a message
    let mut alice = TestClient::new(&server);
    let alice_reg = alice.register(&server, "alice", "password123").await;
    let alice_session = alice_reg["session_id"].as_str().unwrap();

    let channels = alice.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    let mut alice_ws = WsClient::connect(&server).await;
    alice_ws.authenticate(alice_session).await;
    alice_ws.send_message(channel_id, "alice's message").await;
    let create = alice_ws.recv_type("message_create").await.unwrap();
    let message_id = create["message"]["id"].as_str().unwrap();

    // Bob tries to edit Alice's message
    let invite = server.create_invite(&alice).await;
    let mut bob = TestClient::new(&server);
    let bob_reg = bob
        .register_with_invite(&invite, "bob", "password123")
        .await;
    let bob_body: serde_json::Value = bob_reg.json().await.unwrap();
    let bob_session = bob_body["session_id"].as_str().unwrap();

    let mut bob_ws = WsClient::connect(&server).await;
    bob_ws.authenticate(bob_session).await;
    bob_ws.edit_message(message_id, "hacked!").await;

    // Bob should NOT receive a message_update (server silently ignores)
    let update = bob_ws.recv_type("message_update").await;
    assert!(
        update.is_none(),
        "Bob should not be able to edit Alice's message"
    );

    // Verify message unchanged via REST
    let messages = alice.get_messages(channel_id).await;
    assert_eq!(messages[0]["content"], "alice's message");
}

#[tokio::test]
async fn cannot_delete_others_messages() {
    let server = TestServer::new().await;

    let mut alice = TestClient::new(&server);
    let alice_reg = alice.register(&server, "alice", "password123").await;
    let alice_session = alice_reg["session_id"].as_str().unwrap();

    let channels = alice.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    let mut alice_ws = WsClient::connect(&server).await;
    alice_ws.authenticate(alice_session).await;
    alice_ws.send_message(channel_id, "alice's message").await;
    let create = alice_ws.recv_type("message_create").await.unwrap();
    let message_id = create["message"]["id"].as_str().unwrap();

    // Bob tries to delete Alice's message
    let invite = server.create_invite(&alice).await;
    let mut bob = TestClient::new(&server);
    let bob_reg = bob
        .register_with_invite(&invite, "bob", "password123")
        .await;
    let bob_body: serde_json::Value = bob_reg.json().await.unwrap();
    let bob_session = bob_body["session_id"].as_str().unwrap();

    let mut bob_ws = WsClient::connect(&server).await;
    bob_ws.authenticate(bob_session).await;
    bob_ws.delete_message(message_id).await;

    // Bob should NOT receive a message_delete
    let delete = bob_ws.recv_type("message_delete").await;
    assert!(
        delete.is_none(),
        "Bob should not be able to delete Alice's message"
    );

    // Verify message still exists
    let messages = alice.get_messages(channel_id).await;
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn message_has_reactions_field() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    let reg = client.register(&server, "alice", "password123").await;
    let session_id = reg["session_id"].as_str().unwrap();

    let channels = client.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    let mut ws = WsClient::connect(&server).await;
    ws.authenticate(session_id).await;
    ws.send_message(channel_id, "react to me").await;
    ws.recv_type("message_create").await;

    // REST response should include reactions array
    let messages = client.get_messages(channel_id).await;
    assert!(
        messages[0]["reactions"].is_array(),
        "Messages should include reactions field"
    );
    assert!(messages[0]["reactions"].as_array().unwrap().is_empty());
}
