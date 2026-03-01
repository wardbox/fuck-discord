# Pitfalls Research

**Domain:** Self-hosted encrypted group chat platform (desktop/mobile apps, voice, E2EE, push notifications)
**Researched:** 2026-02-28
**Confidence:** MEDIUM-HIGH (verified across multiple sources; some areas are Tauri/Capacitor version-specific and may shift)

## Critical Pitfalls

### Pitfall 1: CORS Origin Hell When Sharing SvelteKit Across Web, Tauri, and Capacitor

**What goes wrong:**
The same SvelteKit frontend must run in three different contexts: browser (same origin as API), Tauri desktop (`tauri://localhost`), and Capacitor mobile (`capacitor://localhost` on iOS, `http://localhost` on Android). Each has a different origin. API requests that work in the browser break silently in Tauri and Capacitor because the server rejects the unfamiliar origin, or because preflight OPTIONS requests are not handled.

**Why it happens:**
Developers build and test in the browser first where CORS is not an issue (same origin). They add Tauri/Capacitor late and discover the API calls fail with opaque CORS errors. The origin strings are platform-specific and not obvious -- Android Capacitor uses `http://localhost` which collides with local dev.

**How to avoid:**
- Maintain an explicit CORS allowlist in the Axum server: `http://localhost`, `https://localhost`, `tauri://localhost`, `capacitor://localhost`, and the production domain. Add it now, before building native apps.
- Handle OPTIONS preflight requests explicitly in Axum middleware.
- Use environment-based `PUBLIC_API_BASE` in SvelteKit so static builds point to the correct API URL.
- Test CORS from all three origins in CI or a manual checklist before each release.

**Warning signs:**
- API calls return 0 status or empty bodies in Tauri/Capacitor but work in browser.
- "No 'Access-Control-Allow-Origin' header" errors in webview dev console.
- Android works but iOS does not (or vice versa) due to different default origins.

**Phase to address:**
Desktop app phase (Tauri integration). Must be resolved before any native app testing begins. Retroactively fixing CORS after building features on top of broken API calls is painful.

---

### Pitfall 2: WebSocket Death on Mobile Background/Foreground Transitions

**What goes wrong:**
iOS and Android aggressively kill WebSocket connections when the app is backgrounded. iOS gives approximately 180 seconds before terminating background tasks. When the user returns, the app shows stale data, missed messages, phantom "online" presence for users who left, or a blank screen requiring manual refresh. The existing WebSocket handler sets users offline only on disconnect -- if the OS kills the socket ungracefully, presence state becomes permanently wrong.

**Why it happens:**
The current `handle_socket` function in `ws/handler.rs` assumes clean disconnects. Mobile OS lifecycle is fundamentally hostile to persistent connections. Developers test on desktop where sockets survive sleep/wake cycles, then discover mobile is a different world.

**How to avoid:**
- Implement a heartbeat/ping-pong mechanism. Server sends periodic pings (every 30 seconds); if no pong is received within 10 seconds, mark the user offline and clean up. The existing code has no heartbeat.
- Add a `Resume` message to the WebSocket protocol that includes a `last_seen_message_id`. On reconnect, the server sends missed messages instead of requiring a full page reload.
- Add presence timeout on the server side -- if no heartbeat received for 60 seconds, mark user offline regardless of socket state.
- On the client, detect `visibilitychange` / app lifecycle events and proactively close + reconnect the socket on foreground resume.
- Queue outbound messages during brief disconnects and flush on reconnect.

**Warning signs:**
- Users appear "online" hours after closing the app.
- Messages sent while backgrounded disappear (never received by server, no error shown).
- Users complain about "missing messages" that exist in the database but were never delivered.

**Phase to address:**
Mobile app phase (Capacitor). However, the server-side heartbeat and resume protocol should be added during the desktop phase because Tauri apps also sleep on lid close.

---

### Pitfall 3: E2EE Key Management Without Multi-Device Support

**What goes wrong:**
You implement X25519 key exchange for DMs on a single device. It works. Then a user logs in on a second device (desktop + phone) and cannot read their DM history. Or worse, they generate a new key pair on the second device, and now messages encrypted to the old key are unreadable, while new messages from different senders go to different keys depending on which device was "last seen."

**Why it happens:**
Single-device E2EE is straightforward: generate keypair, exchange public keys, derive shared secret, encrypt. Multi-device E2EE is a fundamentally different problem requiring either key synchronization between devices or per-device encryption (sender encrypts to all recipient devices). Signal's approach has the sender encrypt separately for each recipient device -- but this requires device registration, pre-key bundles, and a server-side device registry.

**How to avoid:**
- Design the key architecture for multi-device from day one, even if you only ship single-device initially. This means:
  - Each device gets its own X25519 keypair (do NOT share private keys between devices).
  - Server maintains a registry of each user's active devices and their public keys.
  - Sender encrypts the message once with a random symmetric key, then encrypts that symmetric key for each recipient device's public key (fan-out encryption).
- For DM history on new devices: accept that old messages are not readable on new devices (this is how Signal works) OR implement a "device linking" protocol where the old device re-encrypts the symmetric message keys for the new device.
- Do NOT try to export/import private keys between devices -- this defeats the purpose of E2EE and creates a key exfiltration risk.

**Warning signs:**
- The DM schema stores a single `public_key` per user instead of per-device.
- No concept of "device ID" in the authentication or session model.
- Tests only cover single-device scenarios.

**Phase to address:**
E2EE DM phase. The device registry and per-device keypair model must be the foundation. Retrofitting multi-device onto single-device E2EE requires rewriting the entire encryption layer.

---

### Pitfall 4: Encryption-at-Rest Key Stored Next to the Database

**What goes wrong:**
You encrypt the SQLite database with ChaCha20-Poly1305, but the encryption key is stored in a config file or environment variable on the same machine. Anyone with filesystem access to the database also has access to the key. The encryption provides zero additional security -- it is security theater.

**Why it happens:**
For a self-hosted single binary, there is no external key management service (no AWS KMS, no HashiCorp Vault). The operator needs to provide a key somehow, and the path of least resistance is a config file. But if the threat model is "attacker gets access to the server filesystem," a key on that same filesystem does nothing.

**How to avoid:**
- Be honest about the threat model. For a self-hosted chat server, encryption at rest protects against:
  - Stolen hard drives / decommissioned hardware (key in RAM or on separate volume)
  - Database backups being leaked (if key is not in the backup)
  - Shared hosting where other tenants can read files (if key is in memory only)
- Require the encryption key to be provided at startup via stdin, environment variable, or a separate secrets file that is NOT in the data directory. Document this clearly.
- Consider column-level encryption for only sensitive fields (message content, DM content) rather than full database encryption. This is simpler, has less performance impact (~6% for ChaCha20), and lets you index non-sensitive columns.
- TEMP tables and in-memory databases are NOT encrypted by SQLite encryption extensions. Ensure sensitive data never lands in temp storage.
- Test that VACUUM operations work correctly with encryption (they can use massive temp storage).

**Warning signs:**
- The encryption key is in `config.toml` in the same directory as `relay.db`.
- No documentation about key management for self-hosters.
- Backups include both the database file and the config file.

**Phase to address:**
Encryption-at-rest phase. Design the key input mechanism before implementing encryption. The mechanism influences the entire deployment story.

---

### Pitfall 5: Building a WebRTC SFU Instead of Integrating One

**What goes wrong:**
Voice channels seem simple: "just use WebRTC." But peer-to-peer mesh topology breaks at 4-5 participants (each peer sends N-1 streams and receives N-1 streams). So you need an SFU (Selective Forwarding Unit). Building an SFU is an entire company's worth of engineering. You spend months on it, it barely works, and it has echo/noise issues across browsers.

**Why it happens:**
WebRTC's "hello world" (two peers exchanging audio) takes a day. Developers extrapolate that group voice will take a week. It does not. NAT traversal alone (STUN/TURN) accounts for 80% of connectivity failures. Browser differences in echo cancellation and noise suppression cause mysterious audio quality issues. And you still need the SFU.

**How to avoid:**
- Use LiveKit. The project already identified this. Do not deviate. LiveKit is open-source (Apache 2.0), self-hostable as a single binary (Go), handles STUN/TURN internally, and has SDKs for web, iOS, and Android.
- Budget TURN server bandwidth. Even for 10-25 users, if 10% of connections need TURN relay, that is significant bandwidth. LiveKit handles this internally with its built-in TURN.
- Deploy LiveKit as a separate process alongside the Relay binary. Do not try to embed it. The Relay server acts as the signaling/room management layer; LiveKit handles media.
- Use LiveKit's room tokens (JWT-based) -- generate them server-side in Rust when a user joins a voice channel, pass them to the client.
- Test on real networks early (cellular, corporate VPNs, symmetric NATs). "Works on localhost" means nothing for WebRTC.

**Warning signs:**
- Any code that directly uses `RTCPeerConnection` for group calls instead of going through an SFU SDK.
- Audio quality complaints that vary by browser (Chrome vs Firefox vs Safari echo cancellation differences).
- "Works on my network but not on theirs" reports.

**Phase to address:**
Voice channel phase. LiveKit integration should be a clear dependency -- do not start voice work without a running LiveKit instance to test against.

---

### Pitfall 6: Push Notifications Require External Infrastructure You Cannot Self-Host

**What goes wrong:**
You want push notifications for mobile (new message while app is backgrounded). You discover that iOS requires APNs (Apple Push Notification service) and Android requires FCM (Firebase Cloud Messaging). Both are centralized, vendor-controlled services. There is no way to push-notify a mobile device without going through Apple or Google. This directly conflicts with the "no cloud dependencies" and "no external services for core functionality" constraints.

**Why it happens:**
Mobile operating systems do not allow apps to maintain persistent background connections for battery reasons. The only way to wake a backgrounded app is through the OS vendor's push notification service. This is an architectural limitation of iOS and Android, not something you can work around.

**How to avoid:**
- Accept the tradeoff: push notifications on mobile REQUIRE APNs/FCM. Document this honestly. Self-hosters who want push must configure APNs/FCM credentials.
- Make push notifications optional. The app must work without them -- use pull-based unread counts when the app foregrounds, and rely on WebSocket for real-time delivery when the app is active.
- Use the Web Push API (VAPID) for browser-based notifications. This works without Firebase and does not require an Apple Developer account for PWA-style notifications on iOS 16.4+.
- For Capacitor: use `@capacitor/push-notifications` or `@capacitor-firebase/messaging`. Be aware that the standard plugin returns native APNs tokens, while Firebase expects FCM tokens -- using the wrong token type causes silent failures.
- UnifiedPush is an alternative for Android-only (no iOS support). It lets users choose their push provider, but adoption is limited to Matrix/Mastodon ecosystem apps.
- Keep the push notification system as a thin, optional layer. The core chat functionality must never depend on it.

**Warning signs:**
- Planning documents assume push notifications can be fully self-hosted.
- No fallback for when push is not configured.
- Testing only on simulators (push notifications do not work on iOS simulators).

**Phase to address:**
Mobile app phase (Capacitor). The push notification architecture should be designed as "optional enhancement" from the start, not "required for the app to be useful."

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Single `public_key` per user instead of per-device | Simpler DM encryption | Complete rewrite when adding second device support | Never -- design for per-device from day one |
| No WebSocket heartbeat | Less code, simpler protocol | Stale presence, ghost users, missed messages on mobile | Only during initial desktop-only phase; add before mobile |
| Polling for broadcast messages (100ms interval in current code) | Works, simple | CPU waste, 100ms latency floor, battery drain on mobile | Replace with event-driven approach before mobile |
| Hardcoded CORS origins | Quick to ship | Breaks when deploying to custom domains | Acceptable in dev; must be configurable before release |
| Full database encryption instead of column-level | "Everything is encrypted" marketing | Performance overhead on all reads/writes including non-sensitive data | Acceptable if ChaCha20 overhead (~6%) is measured and tolerable |
| Thread replies as flat messages with `parent_id` | Simple schema, easy to implement | Poor query performance for deep threads, N+1 fetches | Acceptable if threads are capped at 2-3 levels depth |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Tauri + SvelteKit | Using SSR or prerendering (Tauri APIs unavailable during build) | Use `adapter-static` in SPA mode with `ssr = false` in root layout |
| Capacitor + SvelteKit | Forgetting `npx cap sync` after installing plugins | Add `cap sync` to the build script; run it automatically after `npm run build` |
| Capacitor iOS | Testing push notifications on simulator | Always test push on physical devices; simulator does not support APNs |
| LiveKit + self-hosted | Running LiveKit without TLS/WSS | LiveKit requires a domain with SSL cert; browsers reject insecure WebRTC connections |
| LiveKit + Docker | Using bridge networking | Use host networking for LiveKit; bridge networking breaks UDP port mapping for WebRTC |
| Capacitor Android | Using `localhost` for dev API URL | Android emulator uses `10.0.2.2` to reach host machine; `localhost` resolves to the emulator itself |
| Web Crypto API | Assuming `crypto.subtle` is available everywhere | Only available in secure contexts (HTTPS or localhost); Tauri's `tauri://` origin counts; Capacitor's `capacitor://` counts |
| APNs + Capacitor | Using native APNs token with Firebase Cloud Messaging | FCM expects FCM tokens, not raw APNs tokens; use `@capacitor-firebase/messaging` which handles token translation |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Broadcast polling at 100ms intervals | Battery drain on mobile, unnecessary CPU on desktop, 100ms minimum latency | Switch to `tokio::sync::broadcast` with `recv().await` (event-driven) instead of `try_recv()` in a loop | Immediately on mobile; noticeable with 5+ channels on desktop |
| Full database encryption with VACUUM | VACUUM operation consumes RAM equal to DB size when using `temp_store=MEMORY` | Use `temp_store=FILE` for encrypted databases; schedule VACUUM during low-activity periods | When database exceeds 500MB |
| Loading all channel members on WebSocket Ready | `get_all_users` sends every user's data on every connect | Paginate or only send online users + users active in last 7 days | At 50+ registered users (even if only 10-25 concurrent) |
| JSON serialization of every message through broadcast | Each message is serialized N times (once per subscriber) | Serialize once, send the same `Arc<String>` to all subscribers | At 20+ concurrent connections with active chat |
| No message batching on reconnect | Reconnecting client requests messages one-by-one | Batch fetch: "give me all messages in channels X, Y, Z since timestamp T" as a single query | Every mobile foreground resume |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Storing E2EE private keys in localStorage/sessionStorage | XSS attack can exfiltrate all private keys, decrypt all DM history | Use IndexedDB with `extractable: false` via Web Crypto API; keys never leave the browser's crypto module |
| Nonce reuse in ChaCha20-Poly1305 | Catastrophic: reusing a nonce with the same key reveals plaintext via XOR of ciphertexts | Use random 192-bit nonces (XChaCha20-Poly1305) or a counter-based scheme; never generate nonces from timestamps alone |
| No authentication on WebSocket upgrade | Anyone can establish a WebSocket connection and consume server resources before authenticating | Current code handles this correctly (auth timeout); maintain this pattern |
| Trusting client-supplied encryption metadata | Client claims message is E2EE but server cannot verify; attacker could send plaintext claiming it is encrypted | Server should never interpret E2EE content; store opaque ciphertext blobs; verification happens client-side |
| Encryption key derivation from user passphrase without salt | Rainbow table attacks on encryption keys | Use Argon2id (already in use for auth) with per-user salt for any passphrase-derived encryption keys |
| LiveKit room tokens without expiry or scope | Stolen token grants permanent access to any voice room | Set short TTL (5 minutes), scope to specific room, include user identity in token claims |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Asking for notification permission on first launch | User denies permission reflexively; cannot re-ask on iOS | Wait until user enables notifications in settings; show in-app prompt explaining value first |
| No visual indication of E2EE status | Users do not know if their DMs are actually encrypted; no trust | Show a lock icon with verification state; allow users to compare safety numbers/key fingerprints |
| WebSocket disconnection shown as error | User sees scary error messages during normal mobile background/foreground transitions | Show subtle "Reconnecting..." indicator; auto-reconnect silently; only show error after 30+ seconds of failure |
| Thread replies breaking chronological flow | Users confused by messages appearing out of order in channel view | Show thread replies inline with a "replied to" reference but keep chronological order; expand thread in a side panel |
| Voice channel with no mute state persistence | User joins voice, mutes, app backgrounds, reconnects -- now unmuted, broadcasting to the room | Persist mute/deafen state in client-side storage; restore on reconnect; default to muted on rejoin |
| Desktop notification spam | Every message in every channel generates a desktop notification | Default to notifications only for mentions and DMs; let users configure per-channel; respect system Do Not Disturb |

## "Looks Done But Isn't" Checklist

- [ ] **WebSocket reconnection:** Often missing exponential backoff -- verify reconnection does not hammer the server with 100 attempts/second after a network blip
- [ ] **E2EE DMs:** Often missing key verification UI -- verify users can compare key fingerprints to detect MITM
- [ ] **Push notifications:** Often missing per-channel mute -- verify users can mute specific channels without disabling all notifications
- [ ] **Voice channels:** Often missing TURN fallback -- verify voice works from behind corporate firewalls and symmetric NATs, not just home WiFi
- [ ] **Tauri desktop:** Often missing auto-update mechanism -- verify the app can update itself without users manually downloading new versions
- [ ] **Capacitor mobile:** Often missing deep link handling -- verify tapping a notification opens the correct channel/DM, not just the app
- [ ] **Threads:** Often missing unread tracking per-thread -- verify "new replies" badge works independently from channel unreads
- [ ] **Encryption at rest:** Often missing backup story -- verify encrypted database can be backed up and restored on a different machine with the same key
- [ ] **Presence:** Often missing "idle" detection -- verify users are marked idle after inactivity, not just online/offline
- [ ] **Mobile app:** Often missing offline message queue -- verify messages typed while offline are sent when connection resumes

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Single-device E2EE design | HIGH | Must add device registry, rewrite key exchange, migrate existing DM encryption -- effectively rewriting the E2EE layer |
| No WebSocket heartbeat/resume | MEDIUM | Add heartbeat to protocol, add resume with `last_seen_id`, update client reconnection logic -- backwards compatible if old clients ignore heartbeat |
| CORS origin issues | LOW | Add origins to allowlist, redeploy server -- no data migration needed |
| Encryption key next to database | MEDIUM | Change key input mechanism, re-encrypt database with new key management -- requires coordinated migration for self-hosters |
| Built custom SFU instead of using LiveKit | HIGH | Discard custom SFU code, integrate LiveKit, rewrite voice channel client -- weeks of wasted work |
| Push notification dependency on Firebase | MEDIUM | Make push optional, add fallback polling, document setup -- requires architectural change but not data migration |
| Polling broadcast loop (current code) | LOW | Replace `try_recv()` loop with `recv().await` -- straightforward refactor, no protocol changes |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| CORS origin hell | Desktop (Tauri) | API calls succeed from `tauri://localhost` origin in Tauri dev mode |
| WebSocket background death | Desktop (Tauri) for heartbeat; Mobile (Capacitor) for full lifecycle | User marked offline within 60s of closing lid; messages delivered on reopen |
| Single-device E2EE | E2EE DMs | Schema has device_id column; key exchange tests cover 2+ devices per user |
| Encryption key placement | Encryption at rest | Key is NOT in the data directory; backup of data dir alone is insufficient to decrypt |
| Building custom SFU | Voice channels | Zero custom WebRTC media handling code; all media goes through LiveKit |
| Push notification dependency | Mobile (Capacitor) | App functions fully (text, presence, channels) without push configured |
| Broadcast polling performance | Desktop (Tauri) -- before mobile | CPU usage does not increase linearly with number of channels |
| Notification permission UX | Mobile (Capacitor) | App never asks for notification permission before user has sent their first message |
| Thread schema design | Threads | Thread replies queryable with single SQL query; no N+1 fetches for thread view |
| Mobile reconnection | Mobile (Capacitor) | Backgrounding app for 5 minutes, then foregrounding shows all missed messages within 2 seconds |

## Sources

- [Tauri v2 SvelteKit integration](https://v2.tauri.app/start/frontend/sveltekit/) -- Official Tauri docs on SSR/SPA requirements (HIGH confidence)
- [Build for Web, Mobile & Desktop from a Single SvelteKit App](https://nsarrazin.com/blog/sveltekit-universal) -- CORS origin handling across platforms (MEDIUM confidence)
- [Why WebRTC Remains Deceptively Complex in 2025](https://webrtc.ventures/2025/08/why-webrtc-remains-deceptively-complex-in-2025/) -- WebRTC infrastructure complexity (MEDIUM confidence)
- [How we built WebRTC chat: top 3 lessons learned](https://www.mindk.com/blog/what-is-webrtc-and-how-to-avoid-its-3-deadliest-pitfalls/) -- WebRTC pitfalls (MEDIUM confidence)
- [TURN server for WebRTC: Complete Guide](https://www.videosdk.live/developer-hub/webrtc/turn-server-for-webrtc) -- NAT traversal statistics (MEDIUM confidence)
- [LiveKit self-hosted deployments](https://docs.livekit.io/deploy/custom/deployments/) -- LiveKit deployment requirements (HIGH confidence)
- [LiveKit self-hosting overview](https://docs.livekit.io/transport/self-hosting/) -- SSL, networking requirements (HIGH confidence)
- [E2EE in Chat Applications: A Complete Guide](https://medium.com/@siddhantshelake/end-to-end-encryption-e2ee-in-chat-applications-a-complete-guide-12b226cae8f8) -- Key management patterns (LOW confidence -- single source)
- [The Ambassador Protocol: Multi-device E2EE](https://medium.com/@TalBeerySec/the-ambassador-protocol-multi-device-e2ee-with-privacy-5c906a2d210a) -- Multi-device encryption approaches (MEDIUM confidence)
- [Signal Double Ratchet Algorithm](https://signal.org/docs/specifications/doubleratchet/) -- Key management best practices (HIGH confidence)
- [UnifiedPush](https://unifiedpush.org/) -- Alternative push notification protocol (MEDIUM confidence)
- [Capacitor Push Notifications Guide](https://capawesome.io/blog/the-push-notifications-guide-for-capacitor/) -- APNs/FCM token issues (MEDIUM confidence)
- [SQLite Performance Optimization with ChaCha20](https://forwardemail.net/en/blog/docs/sqlite-performance-optimization-pragma-chacha20-production-guide) -- Encryption performance impact (MEDIUM confidence)
- [Capacitor Background Runner](https://capacitorjs.com/docs/apis/background-runner) -- iOS background task limitations (HIGH confidence)
- [WebSocket reconnection strategies](https://oneuptime.com/blog/post/2026-01-27-websocket-reconnection-logic/view) -- Mobile reconnection patterns (MEDIUM confidence)
- [Five Mistakes in Designing Mobile Push Notifications](https://www.nngroup.com/articles/push-notification/) -- Notification UX research (HIGH confidence -- NN/g)
- [XChat's E2EE Critically Weak](https://cyberinsider.com/xchats-end-to-end-encryption-critically-weak-warns-researcher/) -- Real-world E2EE implementation failures (MEDIUM confidence)

---
*Pitfalls research for: Self-hosted encrypted group chat platform (Relay / fuck-discord)*
*Researched: 2026-02-28*
