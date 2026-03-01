mod common;

use common::{TestClient, TestServer, WsClient};

#[tokio::test]
async fn two_users_see_each_others_messages() {
    let server = TestServer::new().await;

    // Register Alice
    let mut alice_http = TestClient::new(&server);
    let alice_reg = alice_http.register(&server, "alice", "password123").await;
    let alice_session = alice_reg["session_id"].as_str().unwrap();

    let channels = alice_http.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    // Register Bob
    let invite = server.create_invite(&alice_http).await;
    let mut bob_http = TestClient::new(&server);
    let bob_reg = bob_http
        .register_with_invite(&invite, "bob", "password123")
        .await;
    let bob_body: serde_json::Value = bob_reg.json().await.unwrap();
    let bob_session = bob_body["session_id"].as_str().unwrap();

    // Both connect via WebSocket
    let mut alice_ws = WsClient::connect(&server).await;
    alice_ws.authenticate(alice_session).await;

    let mut bob_ws = WsClient::connect(&server).await;
    bob_ws.authenticate(bob_session).await;

    // Small delay for broadcast subscriptions to settle
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Alice sends a message
    alice_ws
        .send_message(channel_id, "Hello from Alice!")
        .await;

    // Alice should get her own message back via broadcast
    let alice_msg = alice_ws.recv_type("message_create").await;
    assert!(alice_msg.is_some(), "Alice should see her own message");

    // Bob should also receive Alice's message
    let bob_msg = bob_ws.recv_type("message_create").await;
    assert!(bob_msg.is_some(), "Bob should see Alice's message");
    let bob_msg = bob_msg.unwrap();
    assert_eq!(bob_msg["message"]["content"], "Hello from Alice!");
    assert_eq!(bob_msg["message"]["author_username"], "alice");

    // Bob sends a reply
    bob_ws
        .send_message(channel_id, "Hey Alice, Bob here!")
        .await;

    // Bob gets his own message
    let bob_own = bob_ws.recv_type("message_create").await;
    assert!(bob_own.is_some());

    // Alice receives Bob's message
    let alice_recv = alice_ws.recv_type("message_create").await;
    assert!(alice_recv.is_some(), "Alice should see Bob's message");
    let alice_recv = alice_recv.unwrap();
    assert_eq!(alice_recv["message"]["content"], "Hey Alice, Bob here!");
}

#[tokio::test]
async fn typing_indicator_broadcasts_to_others() {
    let server = TestServer::new().await;

    let mut alice_http = TestClient::new(&server);
    let alice_reg = alice_http.register(&server, "alice", "password123").await;
    let alice_session = alice_reg["session_id"].as_str().unwrap();

    let channels = alice_http.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    let invite = server.create_invite(&alice_http).await;
    let mut bob_http = TestClient::new(&server);
    let bob_reg = bob_http
        .register_with_invite(&invite, "bob", "password123")
        .await;
    let bob_body: serde_json::Value = bob_reg.json().await.unwrap();
    let bob_session = bob_body["session_id"].as_str().unwrap();

    let mut alice_ws = WsClient::connect(&server).await;
    alice_ws.authenticate(alice_session).await;

    let mut bob_ws = WsClient::connect(&server).await;
    bob_ws.authenticate(bob_session).await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Alice starts typing
    alice_ws.send_typing(channel_id).await;

    // Bob should receive typing indicator
    let typing = bob_ws.recv_type("typing_start").await;
    assert!(
        typing.is_some(),
        "Bob should see Alice's typing indicator"
    );
    let typing = typing.unwrap();
    assert_eq!(typing["username"], "alice");
    assert_eq!(typing["channel_id"], channel_id);
}

#[tokio::test]
async fn presence_online_on_connect() {
    let server = TestServer::new().await;

    let mut alice_http = TestClient::new(&server);
    let alice_reg = alice_http.register(&server, "alice", "password123").await;
    let alice_session = alice_reg["session_id"].as_str().unwrap();

    let invite = server.create_invite(&alice_http).await;
    let mut bob_http = TestClient::new(&server);
    let bob_reg = bob_http
        .register_with_invite(&invite, "bob", "password123")
        .await;
    let bob_body: serde_json::Value = bob_reg.json().await.unwrap();
    let bob_session = bob_body["session_id"].as_str().unwrap();

    // Alice connects first
    let mut alice_ws = WsClient::connect(&server).await;
    alice_ws.authenticate(alice_session).await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Bob connects — Alice should get presence_update "online"
    let mut bob_ws = WsClient::connect(&server).await;
    bob_ws.authenticate(bob_session).await;

    let presence = alice_ws.recv_type("presence_update").await;
    assert!(presence.is_some(), "Alice should see Bob come online");
    let presence = presence.unwrap();
    assert_eq!(presence["status"], "online");
}

#[tokio::test]
async fn ws_ready_includes_user_and_channels() {
    let server = TestServer::new().await;

    let mut client = TestClient::new(&server);
    let reg = client.register(&server, "alice", "password123").await;
    let session_id = reg["session_id"].as_str().unwrap();

    let mut ws = WsClient::connect(&server).await;
    let ready = ws.authenticate(session_id).await;

    assert_eq!(ready["type"], "ready");
    assert_eq!(ready["user"]["username"], "alice");
    assert!(ready["channels"].is_array());
    assert!(!ready["channels"].as_array().unwrap().is_empty());
    assert!(ready["members"].is_array());
}

#[tokio::test]
async fn ws_auth_with_invalid_token() {
    let server = TestServer::new().await;

    let mut ws = WsClient::connect(&server).await;
    ws.send_json(&serde_json::json!({
        "type": "authenticate",
        "token": "invalid-session-token",
    }))
    .await;

    // Should receive an error message
    let msg = ws.recv().await;
    assert!(msg.is_some());
    let msg = msg.unwrap();
    assert_eq!(msg["type"], "error");
    assert_eq!(msg["code"], "auth_failed");
}

#[tokio::test]
async fn new_channel_broadcasts_to_connected_clients() {
    let server = TestServer::new().await;

    let mut alice_http = TestClient::new(&server);
    let alice_reg = alice_http.register(&server, "alice", "password123").await;
    let alice_session = alice_reg["session_id"].as_str().unwrap();

    // Connect via WebSocket
    let mut alice_ws = WsClient::connect(&server).await;
    alice_ws.authenticate(alice_session).await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Create channel via REST
    alice_http.create_channel("new-channel", None).await;

    // Should receive channel_create via WebSocket
    let channel_create = alice_ws.recv_type("channel_create").await;
    assert!(
        channel_create.is_some(),
        "Should receive channel_create event"
    );
    let channel_create = channel_create.unwrap();
    assert_eq!(channel_create["channel"]["name"], "new-channel");
}

#[tokio::test]
async fn reactions_broadcast_to_channel() {
    let server = TestServer::new().await;

    let mut alice_http = TestClient::new(&server);
    let alice_reg = alice_http.register(&server, "alice", "password123").await;
    let alice_session = alice_reg["session_id"].as_str().unwrap();

    let channels = alice_http.get_channels().await;
    let channel_id = channels[0]["id"].as_str().unwrap();

    let invite = server.create_invite(&alice_http).await;
    let mut bob_http = TestClient::new(&server);
    let bob_reg = bob_http
        .register_with_invite(&invite, "bob", "password123")
        .await;
    let bob_body: serde_json::Value = bob_reg.json().await.unwrap();
    let bob_session = bob_body["session_id"].as_str().unwrap();

    let mut alice_ws = WsClient::connect(&server).await;
    alice_ws.authenticate(alice_session).await;

    let mut bob_ws = WsClient::connect(&server).await;
    bob_ws.authenticate(bob_session).await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Alice sends a message
    alice_ws.send_message(channel_id, "react to this!").await;
    let create = alice_ws.recv_type("message_create").await.unwrap();
    let message_id = create["message"]["id"].as_str().unwrap();

    // Bob also receives the message
    bob_ws.recv_type("message_create").await;

    // Bob adds a reaction
    bob_ws.add_reaction(message_id, "👍").await;

    // Both should receive reaction_update
    let alice_reaction = alice_ws.recv_type("reaction_update").await;
    assert!(
        alice_reaction.is_some(),
        "Alice should see the reaction update"
    );
    let alice_reaction = alice_reaction.unwrap();
    assert_eq!(alice_reaction["message_id"], message_id);
    let reactions = alice_reaction["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0]["emoji"], "👍");
    assert_eq!(reactions[0]["count"], 1);
}

#[tokio::test]
async fn messages_in_new_channel_broadcast_correctly() {
    let server = TestServer::new().await;

    let mut alice_http = TestClient::new(&server);
    let alice_reg = alice_http.register(&server, "alice", "password123").await;
    let alice_session = alice_reg["session_id"].as_str().unwrap();

    let invite = server.create_invite(&alice_http).await;
    let mut bob_http = TestClient::new(&server);
    let bob_reg = bob_http
        .register_with_invite(&invite, "bob", "password123")
        .await;
    let bob_body: serde_json::Value = bob_reg.json().await.unwrap();
    let bob_session = bob_body["session_id"].as_str().unwrap();

    // Both connect
    let mut alice_ws = WsClient::connect(&server).await;
    alice_ws.authenticate(alice_session).await;

    let mut bob_ws = WsClient::connect(&server).await;
    bob_ws.authenticate(bob_session).await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Create a new channel AFTER connections are established
    let new_channel = alice_http
        .create_channel("dynamic-channel", None)
        .await;
    let new_channel_id = new_channel["id"].as_str().unwrap();

    // Consume channel_create events
    alice_ws.recv_type("channel_create").await;
    bob_ws.recv_type("channel_create").await;

    // Wait for dynamic subscription to kick in
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Send a message in the new channel
    alice_ws
        .send_message(new_channel_id, "Hello in new channel!")
        .await;

    // Alice gets her message
    let alice_msg = alice_ws.recv_type("message_create").await;
    assert!(alice_msg.is_some());

    // Bob should also get it (dynamic subscription)
    let bob_msg = bob_ws.recv_type("message_create").await;
    assert!(
        bob_msg.is_some(),
        "Bob should receive messages in dynamically created channel"
    );
    assert_eq!(
        bob_msg.unwrap()["message"]["content"],
        "Hello in new channel!"
    );
}

#[tokio::test]
async fn set_status_via_ws() {
    let server = TestServer::new().await;

    let mut alice_http = TestClient::new(&server);
    let alice_reg = alice_http.register(&server, "alice", "password123").await;
    let alice_session = alice_reg["session_id"].as_str().unwrap();

    let invite = server.create_invite(&alice_http).await;
    let mut bob_http = TestClient::new(&server);
    let bob_reg = bob_http
        .register_with_invite(&invite, "bob", "password123")
        .await;
    let bob_body: serde_json::Value = bob_reg.json().await.unwrap();
    let bob_session = bob_body["session_id"].as_str().unwrap();

    let mut alice_ws = WsClient::connect(&server).await;
    alice_ws.authenticate(alice_session).await;

    let mut bob_ws = WsClient::connect(&server).await;
    bob_ws.authenticate(bob_session).await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Alice sets status to idle
    alice_ws
        .send_json(&serde_json::json!({
            "type": "set_status",
            "status": "idle",
        }))
        .await;

    // Bob should receive presence_update
    let presence = bob_ws.recv_type("presence_update").await;
    assert!(presence.is_some(), "Bob should see Alice go idle");
    let presence = presence.unwrap();
    assert_eq!(presence["status"], "idle");
}
