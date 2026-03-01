# Architecture Patterns

**Domain:** Self-hosted encrypted group chat platform (subsequent milestone features)
**Researched:** 2026-02-28

## Current Architecture (Baseline)

Before describing how new features integrate, here is what exists and works today:

```
[Browser SPA] --HTTP REST--> [Axum Server] --rusqlite--> [SQLite DB]
[Browser SPA] --WebSocket--> [WS Handler]  --broadcast--> [Per-Channel Tx]
                                           --rusqlite--> [SQLite DB]
```

**Server:** Single Rust binary (Axum 0.8) serving both API routes and the embedded SvelteKit SPA via rust-embed. WebSocket handler authenticates via session token, subscribes to per-channel `tokio::sync::broadcast` channels, and forwards messages bidirectionally. Database is SQLite via r2d2 connection pool (max 8 connections, WAL mode).

**Client:** SvelteKit SPA compiled with `@sveltejs/adapter-static` (fallback: `index.html`). Connects to same-origin WebSocket. Uses Svelte 5 runes for state management across stores (auth, channels, messages, members, connection). No SSR -- pure client-side rendering.

**Protocol:** JSON-tagged union over WebSocket (`{ "type": "send_message", ... }`). 12 server message types, 10 client message types. Authentication is first-message-must-be-authenticate with 10s timeout.

**Key architectural property:** The SPA is already built with `adapter-static` and `ssr = false`, which is exactly what Tauri and Capacitor require. This means the client can be wrapped in native shells with zero architectural changes to the rendering layer.

## Recommended Architecture (With New Features)

```
                                    +------------------+
                                    |   Tauri Desktop   |
                                    | (WebView + Rust)  |
                                    +--------+---------+
                                             |
+------------------+              +----------v-----------+              +------------------+
| Capacitor Mobile | --HTTPS/WSS->|   Axum Server         | --SQLite--> |    relay.db      |
| (WKWebView/      |              |                       |             | (messages, users, |
|  WebView)         |              | /api/*   REST routes  |             |  channels, keys, |
+------------------+              | /ws      WebSocket    |             |  threads, DMs)   |
                                  | /voice/* Signaling    |             +------------------+
+------------------+              | /push    Registration |
| Browser SPA      | --WS/HTTP--> |                       |
| (same as today)  |              +-----------+-----------+
+------------------+                          |
                                   +----------v-----------+
                                   |   Voice SFU Module    |
                                   |   (str0m / LiveKit)   |
                                   |   UDP media plane     |
                                   +-----------------------+
```

### Component Boundaries

| Component | Responsibility | Communicates With | New/Existing |
|-----------|---------------|-------------------|--------------|
| **Axum HTTP Server** | REST API, auth, file uploads, static file serving | All clients, SQLite | Existing -- extend with DM, thread, push, voice signaling routes |
| **WebSocket Handler** | Real-time message relay, presence, typing | All connected clients, broadcast channels | Existing -- extend protocol for DMs, threads, E2EE key exchange, voice signaling |
| **SQLite Database** | Persistent storage for all data | Axum server only (via r2d2 pool) | Existing -- add tables for DMs, threads, push subscriptions, public keys |
| **Voice SFU Module** | WebRTC media forwarding for voice channels | Clients (UDP), Axum (signaling via WS) | **NEW** -- separate async task or thread within the same binary |
| **Tauri Shell** | Desktop native wrapper, system tray, notifications, deep links | SvelteKit SPA (WebView IPC), Relay server (WS/HTTP) | **NEW** -- wraps existing SPA |
| **Capacitor Shell** | Mobile native wrapper, push notifications, background keep-alive | SvelteKit SPA (WebView bridge), Relay server (WS/HTTP), FCM/APNs | **NEW** -- wraps existing SPA |
| **E2EE Crypto Layer** | Client-side DM encryption/decryption | Runs entirely in browser/WebView, server stores opaque ciphertext | **NEW** -- client-only JS module |
| **Push Notification Service** | Server-side Web Push (VAPID) delivery | Axum server sends pushes, clients register subscriptions | **NEW** -- server-side crate + client registration |

### Data Flow

#### Text Messages (existing, unchanged)
```
Client --WS--> Authenticate --> Subscribe to channels
Client --WS--> SendMessage { channel_id, content }
  Server: write to SQLite + FTS5, broadcast via channel Tx
  Server --WS--> MessageCreate to all subscribers
```

#### Direct Messages (new, with E2EE)
```
Client A: Generate X25519 keypair, upload public key to server
Client B: Generate X25519 keypair, upload public key to server

Client A --WS--> SendDM { recipient_id, encrypted_content, sender_public_key_id }
  Server: store opaque ciphertext in dm_messages table, relay via WS
  Server --WS--> DMCreate to recipient (if online)
  Server --Push--> Web Push to recipient (if offline + subscribed)

Client B: fetch sender's public key, ECDH derive shared secret, AES-256-GCM decrypt
```

The server NEVER sees plaintext DM content. It stores ciphertext and forwards it. Key exchange happens client-side via Web Crypto API (X25519) or `@noble/curves` for browsers without native X25519 support.

#### Threaded Messages (new)
```
Client --WS--> SendMessage { channel_id, content, thread_parent_id }
  Server: write message with thread_parent_id FK, broadcast to channel
  Server --WS--> MessageCreate { message: { ..., thread_parent_id } }

Client --REST--> GET /api/channels/{id}/messages/{msg_id}/thread?before=X&limit=50
  Server: query messages WHERE thread_parent_id = msg_id ORDER BY id
```

Threads are a flat list under a parent message (no nesting). Discord-style: click a message to open a thread panel. The `thread_parent_id` column on the messages table is nullable -- null means top-level message, non-null means thread reply.

#### Voice Channels (new)
```
Client --WS--> JoinVoice { channel_id }
  Server: create or join voice room, return ICE candidates and SDP offer
Client --WS--> VoiceSignal { sdp/ice data }
  Server: relay signaling to SFU module

SFU Module <--UDP--> Client (direct media plane, peer-to-peer after signaling)
  Audio: Opus codec, 48kHz
  SFU forwards audio from each participant to all others
```

Signaling goes through the existing WebSocket. Media goes over UDP directly between client and SFU. The SFU runs as a separate async task within the same Rust binary, using str0m's sans-IO pattern (no internal threads -- driven by the tokio event loop).

#### Push Notifications (new)
```
Client --REST--> POST /api/push/subscribe { endpoint, keys: { p256dh, auth } }
  Server: store Web Push subscription in push_subscriptions table

On new DM or @mention while recipient is offline:
  Server: web-push crate sends VAPID-signed notification to push endpoint
  Browser/OS: displays native notification
  User taps: deep link opens app to relevant channel/DM
```

#### Desktop App (Tauri, new shell around existing SPA)
```
Tauri app launches:
  1. Loads SvelteKit SPA in WebView (same build output as rust-embed)
  2. SPA connects to remote Relay server via WSS/HTTPS (configurable server URL)
  3. Tauri provides: system tray, native notifications, deep links, auto-update

IPC: SPA calls Tauri commands via @tauri-apps/api for:
  - Reading/writing local settings (server URL, notification prefs)
  - Showing native notifications (tauri-plugin-notification)
  - System tray badge/icon updates
  - Deep link handling (relay://open/channel/general)
```

Key insight: The Tauri app does NOT embed or run the Relay server. It is a client that connects to a remote Relay server, just like the browser does. The WebSocket URL becomes configurable rather than same-origin.

#### Mobile App (Capacitor, new shell around existing SPA)
```
Capacitor app launches:
  1. Loads SvelteKit SPA in native WebView
  2. SPA connects to remote Relay server (server URL stored in Capacitor Storage)
  3. Capacitor provides: push notifications, background mode, haptics, camera

Push flow:
  App registers with FCM (Android) / APNs (iOS) via @capacitor/push-notifications
  App sends device token to Relay server: POST /api/push/subscribe
  Server stores subscription, sends push when needed
```

## Patterns to Follow

### Pattern 1: Platform-Agnostic Connection Store
**What:** Abstract the WebSocket connection URL so the same SPA works in browser (same-origin), Tauri (remote URL), and Capacitor (remote URL).
**When:** Building multi-platform client.
**Why:** The current `connection.svelte.ts` hardcodes `window.location.host` for the WS URL. This must become configurable.
**Example:**
```typescript
// lib/config.ts
function getServerUrl(): string {
  // Tauri: read from local settings via IPC
  // Capacitor: read from Capacitor Preferences
  // Browser: same-origin
  if (window.__TAURI__) {
    return localStorage.getItem('relay_server_url') || 'ws://localhost:3000';
  }
  if ((window as any).Capacitor) {
    return localStorage.getItem('relay_server_url') || 'ws://localhost:3000';
  }
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${window.location.host}`;
}
```

### Pattern 2: Server-Side CORS for Native Clients
**What:** The Axum server must allow cross-origin requests from `tauri://localhost` and `capacitor://localhost` origins in production.
**When:** Desktop and mobile clients connect to a remote server.
**Example:**
```rust
// In handlers/mod.rs router setup
let cors = CorsLayer::new()
    .allow_origin([
        "tauri://localhost".parse().unwrap(),
        "capacitor://localhost".parse().unwrap(),
        "http://localhost:5173".parse().unwrap(), // dev
    ])
    .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
    .allow_headers(Any)
    .allow_credentials(true);
```

### Pattern 3: E2EE as Client-Only Module
**What:** All encryption/decryption happens in the browser/WebView. The server is a dumb relay for encrypted blobs.
**When:** DM messages.
**Why:** The server should never be able to read DMs, even if the database is compromised.
**Architecture:**
```
Client-side crypto module:
  1. generateKeyPair() -> { publicKey, privateKey } using X25519
  2. deriveSharedSecret(theirPublicKey, myPrivateKey) -> sharedKey
  3. encrypt(plaintext, sharedKey) -> { ciphertext, nonce } using AES-256-GCM
  4. decrypt(ciphertext, nonce, sharedKey) -> plaintext

Server stores:
  - User public keys (uploaded at registration or key rotation)
  - Encrypted message blobs (opaque to server)
  - Key IDs (so clients know which key was used)
```

### Pattern 4: Voice Signaling Through Existing WebSocket
**What:** Reuse the existing WebSocket connection for WebRTC signaling rather than adding a separate signaling channel.
**When:** Voice channel join/leave/ICE/SDP exchange.
**Why:** One fewer connection to manage. The WS is already authenticated.
**Example protocol additions:**
```rust
// New ClientMessage variants
JoinVoice { channel_id: String },
LeaveVoice { channel_id: String },
VoiceSignal { channel_id: String, signal: serde_json::Value },

// New ServerMessage variants
VoiceReady { channel_id: String, participants: Vec<String>, sdp_offer: String },
VoiceSignal { channel_id: String, from_user_id: String, signal: serde_json::Value },
VoiceJoin { channel_id: String, user_id: String },
VoiceLeave { channel_id: String, user_id: String },
```

### Pattern 5: Thread Replies as Column, Not Separate Table
**What:** Add `thread_parent_id` nullable FK to the existing `messages` table rather than creating a separate threads table.
**When:** Implementing threaded conversations.
**Why:** Thread replies ARE messages. They share the same schema, FTS index, reactions, etc. A separate table means duplicated logic. Discord, Slack, and Matrix all use the parent-reference approach.
**Schema change:**
```sql
ALTER TABLE messages ADD COLUMN thread_parent_id TEXT REFERENCES messages(id) ON DELETE SET NULL;
CREATE INDEX idx_messages_thread ON messages(thread_parent_id) WHERE thread_parent_id IS NOT NULL;
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Embedding the Server in Tauri
**What:** Running the Relay Axum server inside the Tauri Rust backend.
**Why bad:** Tauri's Rust side is for local IPC commands, not long-running network servers. It would mean every desktop client runs a full server, which is not the deployment model (one server, many clients). It also makes the Tauri app much heavier and harder to update.
**Instead:** Tauri app is a pure client. It connects to an external Relay server via WSS/HTTPS. The server URL is user-configurable.

### Anti-Pattern 2: Full Double Ratchet for Friend-Group DMs
**What:** Implementing the full Signal Protocol (X3DH + Double Ratchet) for DM encryption.
**Why bad:** The Double Ratchet provides forward secrecy per-message, which is designed for adversarial environments where keys may be compromised. For a friend-group chat where the server is self-hosted and trusted for metadata (just not content), this is extreme over-engineering. It requires prekey bundles, session management, ratchet state persistence, and handling of out-of-order messages -- each a significant implementation effort.
**Instead:** Use static X25519 ECDH key agreement + AES-256-GCM per-conversation. Each user has a long-lived X25519 keypair. Shared secrets are derived per conversation pair. This provides encryption at rest (server cannot read DMs) without the complexity of per-message ratcheting. If the threat model later requires forward secrecy, the crypto module can be upgraded without changing the server.

### Anti-Pattern 3: Building a Custom SFU from Scratch
**What:** Writing WebRTC media handling (DTLS, SRTP, ICE, Opus decoding, jitter buffers) from zero.
**Why bad:** An SFU is a whole company's worth of work. str0m provides the primitives but explicitly warns that "writing a full-featured SFU or MCU is a significant undertaking." LiveKit, Janus, and mediasoup exist because this is genuinely hard.
**Instead:** Start with str0m's chat example as a proof-of-concept for voice-only (no video). Keep the SFU scope minimal: forward Opus audio between N participants in a voice channel. No recording, no mixing, no transcoding. If this proves too complex, fall back to LiveKit as an external service (it's open-source and self-hostable, written in Go).

### Anti-Pattern 4: Firebase Dependency for Push Notifications
**What:** Using Firebase Cloud Messaging as the push infrastructure on all platforms.
**Why bad:** The entire project philosophy is self-hosted, no-cloud-dependency. Firebase requires a Google account and sends data through Google's servers.
**Instead:** Use Web Push with VAPID keys (self-hosted, no Google dependency) for browser and Tauri desktop notifications. For Capacitor mobile, unfortunately, native push on iOS requires APNs and Android requires FCM -- these are platform-level dependencies, not Firebase-the-product. Use @capacitor/push-notifications which abstracts this. The server side uses the Rust `web-push` crate to send VAPID-signed notifications directly to the browser push endpoints.

## Database Schema Extensions

New tables and columns needed for the features in this milestone:

```sql
-- Migration: add thread support to messages
ALTER TABLE messages ADD COLUMN thread_parent_id TEXT REFERENCES messages(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_parent_id)
  WHERE thread_parent_id IS NOT NULL;

-- DM conversations
CREATE TABLE IF NOT EXISTS dm_conversations (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS dm_participants (
    conversation_id TEXT NOT NULL REFERENCES dm_conversations(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (conversation_id, user_id)
);

-- DM messages (encrypted content)
CREATE TABLE IF NOT EXISTS dm_messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES dm_conversations(id) ON DELETE CASCADE,
    author_id TEXT NOT NULL REFERENCES users(id),
    ciphertext BLOB NOT NULL,          -- AES-256-GCM encrypted
    nonce BLOB NOT NULL,               -- 12-byte IV
    sender_key_id TEXT NOT NULL,       -- which public key was used
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_dm_messages_conv ON dm_messages(conversation_id, created_at);

-- User public keys for E2EE
CREATE TABLE IF NOT EXISTS user_public_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    public_key BLOB NOT NULL,          -- X25519 public key (32 bytes)
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    revoked_at TEXT                     -- null = active
);
CREATE INDEX IF NOT EXISTS idx_user_keys ON user_public_keys(user_id);

-- Web Push subscriptions
CREATE TABLE IF NOT EXISTS push_subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL,
    p256dh TEXT NOT NULL,              -- client public key for push encryption
    auth TEXT NOT NULL,                -- client auth secret
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_push_subs_user ON push_subscriptions(user_id);

-- Unread tracking (for notification badges)
CREATE TABLE IF NOT EXISTS unread_cursors (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    last_read_message_id TEXT,
    mention_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, channel_id)
);
```

## Build Order (Dependency-Driven)

The features have genuine dependencies that dictate build order:

```
1. Threads          (no dependencies -- extends existing messages)
2. DMs              (no dependencies -- new tables + WS protocol)
3. E2EE for DMs     (depends on: DMs existing to encrypt)
4. Unread tracking  (depends on: DMs and threads to track)
5. Desktop (Tauri)  (depends on: configurable server URL in SPA)
6. Push notifications (depends on: unreads to know what to push)
7. Mobile (Capacitor) (depends on: push notifications for viable mobile experience)
8. Voice channels   (independent -- but highest complexity, do last)
```

**Rationale for this ordering:**

1. **Threads first** because they are the simplest change (one column + one index + minor protocol/UI changes) and immediately improve the chat experience. No new subsystems.

2. **DMs second** because they establish the conversation model that E2EE will build on. Getting DMs working with plaintext first lets you validate the UX before adding encryption complexity.

3. **E2EE third** because it layers on top of working DMs. Separating DM transport from DM encryption means you can debug each independently. The crypto module is client-only, so it does not touch the server beyond key storage endpoints.

4. **Unread tracking fourth** because it is needed by both push notifications and the desktop/mobile notification badges. It is a prerequisite for meaningful notifications.

5. **Desktop (Tauri) fifth** because it requires making the SPA connection URL configurable (a small refactor) and then wrapping in Tauri. The SPA is already `adapter-static` with `ssr = false` -- perfectly compatible. Tauri also provides native notifications without push infrastructure.

6. **Push notifications sixth** because desktop notifications can work without push (Tauri's native notification API), but mobile needs push to be usable. Building push before mobile means the server-side infrastructure is ready.

7. **Mobile (Capacitor) seventh** because it needs push notifications to be viable (mobile apps that only work when open are useless). The SPA wrapping is similar to Tauri. The main work is FCM/APNs integration via @capacitor/push-notifications.

8. **Voice channels last** because they are the highest complexity, most independent feature. Voice has zero dependency on other features and the most risk of scope expansion. Doing it last means all other features are stable and you can give voice full attention.

## Scalability Considerations

| Concern | At 10 users (target) | At 50 users | At 200 users |
|---------|---------------------|-------------|--------------|
| **WebSocket connections** | 10 persistent WS -- trivial | 50 WS -- still fine for tokio | 200 WS -- may need to tune broadcast channel buffer sizes (currently 1024) |
| **Voice SFU** | 5-person voice call -- SFU forwards 4 audio streams per user = 20 total | 10-person call -- 90 forwarded streams, still manageable | 50-person call -- SFU forwarding scales O(N^2) in bandwidth, limit to ~25 per room |
| **SQLite writes** | WAL mode handles concurrent readers fine. Writes are serialized but fast at this scale | Still fine -- SQLite handles thousands of inserts/sec | May see write contention. Solution: sharding by channel or upgrading to PostgreSQL (unlikely needed) |
| **Push notifications** | 10 push targets -- instant | 50 push targets per message (mentions) -- still fast | 200 pushes on @everyone -- batch with async task, do not block message handler |
| **E2EE key derivation** | One ECDH per DM conversation, cached -- negligible | Same -- O(conversations), not O(messages) | Same |

## Sources

- [Tauri v2 SvelteKit guide](https://v2.tauri.app/start/frontend/sveltekit/) - Official Tauri docs (HIGH confidence)
- [Tauri v2 WebSocket plugin](https://v2.tauri.app/plugin/websocket/) - Official plugin docs (HIGH confidence)
- [Tauri v2 Notification plugin](https://v2.tauri.app/plugin/notification/) - Official plugin docs (HIGH confidence)
- [Tauri v2 Deep Linking](https://v2.tauri.app/plugin/deep-linking/) - Official plugin docs (HIGH confidence)
- [Capacitor Push Notifications API](https://capacitorjs.com/docs/apis/push-notifications) - Official Capacitor docs (HIGH confidence)
- [Capacitor with Svelte](https://capacitorjs.com/solution/svelte) - Official Capacitor guide (HIGH confidence)
- [SvelteKit adapter-static](https://svelte.dev/docs/kit/adapter-static) - Official SvelteKit docs (HIGH confidence)
- [Building universal SvelteKit apps](https://nsarrazin.com/blog/sveltekit-universal) - Community guide for Tauri + Capacitor from single codebase (MEDIUM confidence)
- [str0m WebRTC library](https://docs.rs/str0m) - Official docs (HIGH confidence)
- [str0m GitHub](https://github.com/algesten/str0m) - Source + SFU chat example (HIGH confidence)
- [LiveKit Rust SDK](https://github.com/livekit/rust-sdks) - Official SDK (HIGH confidence)
- [web-push Rust crate](https://crates.io/crates/web-push) - VAPID push notifications in Rust (HIGH confidence)
- [Web Crypto API Secure Curves](https://wicg.github.io/webcrypto-secure-curves/) - X25519 in browsers spec (HIGH confidence)
- [Signal Double Ratchet spec](https://signal.org/docs/specifications/doubleratchet/) - Reference for what NOT to build (MEDIUM confidence)
- [VAPID self-hosted push](https://medium.com/@kaushalsinh73/fastapi-web-push-vapid-real-time-notifications-without-vendor-lock-in-43540ec855f6) - Self-hosted push architecture (MEDIUM confidence)
- [Developing a WebRTC SFU in Rust](https://medium.com/@h3poteto/developing-a-webrtc-sfu-library-in-rust-019d467ab6c1) - SFU architecture patterns (MEDIUM confidence)
