mod common;

use common::{TestClient, TestServer};

#[tokio::test]
async fn register_with_valid_invite() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);

    let body = client.register(&server, "alice", "password123").await;
    assert!(body["user"]["id"].is_string());
    assert_eq!(body["user"]["username"], "alice");
    assert!(body["session_id"].is_string());
}

#[tokio::test]
async fn register_with_invalid_invite() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);

    let res = client
        .register_with_invite("bogus-code", "alice", "password123")
        .await;
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn register_duplicate_username() {
    let server = TestServer::new().await;
    let mut client1 = TestClient::new(&server);
    client1.register(&server, "alice", "password123").await;

    // Create a second invite for the second registration
    let invite2 = server.create_invite(&client1).await;

    let mut client2 = TestClient::new(&server);
    let res = client2
        .register_with_invite(&invite2, "alice", "differentpass")
        .await;
    assert_eq!(res.status(), 409);
}

#[tokio::test]
async fn register_short_username() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);

    let res = client
        .register_with_invite(&server.invite_code, "a", "password123")
        .await;
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn register_short_password() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);

    let res = client
        .register_with_invite(&server.invite_code, "alice", "short")
        .await;
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn login_with_correct_credentials() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    // Login with a fresh client (no existing session)
    let mut client2 = TestClient::new(&server);
    let res = client2.login("alice", "password123").await;
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn login_with_wrong_password() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let mut client2 = TestClient::new(&server);
    let res = client2.login("alice", "wrongpassword").await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn login_nonexistent_user() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);

    let res = client.login("nobody", "password123").await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn protected_endpoint_without_session() {
    let server = TestServer::new().await;
    let client = TestClient::new(&server);

    // Try to access channels without logging in
    let res = client
        .get(&format!("{}/api/channels", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn protected_endpoint_with_session() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let res = client
        .get(&format!("{}/api/channels", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn logout_clears_session() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    // Logout
    let res = client
        .post(&format!("{}/api/auth/logout", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // Protected endpoint should now fail
    let res = client
        .get(&format!("{}/api/channels", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn get_current_user() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let res = client
        .get(&format!("{}/api/auth/me", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["username"], "alice");
}

#[tokio::test]
async fn create_and_list_invites() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    // Create invite
    let res = client
        .post(&format!("{}/api/invites", server.base_url))
        .json(&serde_json::json!({"max_uses": 5}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body["code"].is_string());

    // List invites
    let res = client
        .get(&format!("{}/api/invites", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let invites: Vec<serde_json::Value> = res.json().await.unwrap();
    assert!(!invites.is_empty());
}

#[tokio::test]
async fn update_user_profile() {
    let server = TestServer::new().await;
    let mut client = TestClient::new(&server);
    client.register(&server, "alice", "password123").await;

    let res = client
        .patch(&format!("{}/api/users/me", server.base_url))
        .json(&serde_json::json!({"display_name": "Alice W."}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["display_name"], "Alice W.");
}
