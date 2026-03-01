# Project Research Summary

**Project:** Relay (fuck-discord)
**Domain:** Self-hosted encrypted group chat platform
**Researched:** 2026-02-28
**Confidence:** HIGH

## Executive Summary

Relay is a self-hosted, privacy-first group chat platform targeting Discord's exact pain points: forced identity verification, cloud dependency, and lack of meaningful encryption. The existing codebase is in strong shape — Rust/Axum server with SQLite, SvelteKit SPA, real-time WebSocket messaging, and FTS5 search are all working. The research confirms that the next milestone should complete the "table stakes" gap between Relay and Discord: Direct Messages, a desktop app, unread tracking, @mentions, and desktop notifications. These features, delivered together, are what gets a friend group to make the switch and stay switched.

The recommended stack evolution is unified around Tauri v2 as the cross-platform shell for both desktop and mobile. Since the project is already Rust-native and the SvelteKit SPA is already built with `adapter-static` and `ssr = false`, Tauri wrapping requires zero architectural changes to the rendering layer. LiveKit (self-hosted Go SFU) is the clear choice for voice channels — the research is emphatic that building a custom WebRTC SFU is "a whole company's worth of work" and must not be attempted. E2EE for DMs uses X25519 + AES-256-GCM via Web Crypto API, with the server acting as a dumb relay for opaque ciphertext — the server never sees DM plaintext.

The primary risks are architectural: multi-device E2EE design must be planned for from the start (retrofitting it later requires rewriting the encryption layer), the WebSocket protocol needs heartbeat/resume before mobile deployment (iOS/Android aggressively kill background connections), and CORS origin handling for Tauri and Capacitor must be addressed before native app testing begins. These are not optional polish items — they are foundation decisions that determine whether the platform is buildable within a reasonable timeframe. Get them right in the correct phase and the rest follows naturally.

## Key Findings

### Recommended Stack

The existing stack is well-chosen and stable. New additions for subsequent milestones are modest and proven. See `.planning/research/STACK.md` for full version details.

**Core technologies (new additions only):**
- **Tauri v2 (desktop + mobile):** System webview, tiny binaries, official SvelteKit guide, Rust-native — zero friction given existing stack. Supersedes original Capacitor v6 plan (Capacitor is now at v8; v6 is EOL).
- **LiveKit 1.9.x (voice SFU):** Self-hostable Go binary, handles all WebRTC complexity (NAT/TURN/codec/bandwidth). livekit-api 0.4.x (Rust) for room management from Axum. livekit-client 2.17.x (JS) for the SvelteKit frontend.
- **chacha20poly1305 0.10.1 (encryption at rest):** NCC Group audited, fast on ARM without hardware AES-NI. Per-column encryption of message content — not full database encryption, which conflicts with FTS5.
- **x25519-dalek 2.0.1 + Web Crypto API + @noble/curves 2.0.x (E2EE):** Server-side key storage, client-side only encryption/decryption. Server never touches plaintext.
- **web-push 0.11.0 (server-side push):** VAPID-based Web Push from Axum. No Google/Apple dependency for browser push. Mobile push (APNs/FCM) is a platform-level constraint that cannot be avoided.
- **Threads:** No new libraries. One nullable `parent_message_id` column on the messages table.

**Version correction:** The original plan specified Capacitor v6. Capacitor is now at v8 (Node.js 22+, Xcode 26+ required). Use Tauri v2 mobile as the primary path; fall back to Capacitor 8 only if Tauri's mobile plugin ecosystem proves insufficient for the specific use case.

### Expected Features

See `.planning/research/FEATURES.md` for the full prioritization matrix and competitor analysis.

**Must have — P1 (gets the group to switch):**
- Direct Messages (1:1 and group) — private conversation is non-negotiable; #1 missing feature
- Desktop App (Tauri v2) — system tray, native notifications, always-on; makes Relay a real app
- Unread Tracking + Indicators — sidebar dots, mention badges; without this the app feels dead
- @Mentions — `@username`, `@everyone`, `@here` with autocomplete; fundamental to multi-user chat
- Desktop Notifications — OS-native toast for mentions/DMs; required for the desktop app to be useful
- Message Pinning — low complexity, high utility; quick win

**Should have — P2 (makes the group stay):**
- Mobile App (Tauri v2 mobile) — push notifications required for viability
- Voice Channels (LiveKit) — the friend group still needs Discord for gaming voice until this ships
- Notification Settings (per-channel) — needed when users start complaining about noise
- Link Previews — URL unfurling; SSRF prevention required server-side
- Encryption at Rest (ChaCha20-Poly1305) — server admin cannot read message plaintext
- E2EE for DMs — genuine differentiator; no competitor ships this by default at friend-group scale
- Threads — needed when channel conversations get tangled; not critical at 10-25 users

**Defer — v2+ (makes the platform mature):**
- Video/Screen Sharing — LiveKit supports it but UI complexity is high; defer until voice is proven
- Custom Emoji — Unicode covers 99% of use cases
- Webhook Integrations — simpler than a full bot API; add when asked for
- Admin Dashboard — CLI sufficient for now
- Message Export — SQLite makes this trivial; build it when asked

**Anti-features (explicitly don't build):** Federation, Bot API, OAuth/SSO, Activity Status/Rich Presence, Server Discovery, AI Features, Forum Channels, WYSIWYG Editor.

### Architecture Approach

The client is already in perfect shape for Tauri and Capacitor wrapping: `adapter-static` with `ssr = false` means zero changes to the rendering layer. The primary architectural work for new features is: (1) making the WebSocket connection URL configurable rather than same-origin, (2) extending the WebSocket protocol with new message types for DMs/threads/voice signaling, (3) adding new SQLite tables for DMs, push subscriptions, unread cursors, and user public keys, and (4) integrating LiveKit as a separate process for voice.

The Tauri app is a pure client — it connects to a remote Relay server and does NOT embed or run the server. The server URL becomes user-configurable. This is a clean separation that must be established before building native features on top.

**Major components:**
1. **Axum HTTP Server** — REST API, auth, file uploads, static file serving; extend with DM/thread/push/voice signaling routes
2. **WebSocket Handler** — real-time relay, presence, typing; extend protocol for DMs, threads, E2EE key exchange, voice signaling
3. **SQLite Database** — add tables: `dm_conversations`, `dm_participants`, `dm_messages`, `user_public_keys`, `push_subscriptions`, `unread_cursors`; add column: `thread_parent_id` on messages
4. **E2EE Crypto Layer** — client-only JS module; X25519 ECDH + AES-256-GCM; server stores opaque ciphertext
5. **Tauri Shell** — desktop native wrapper; system tray, notifications, deep links; pure client connecting to remote server
6. **LiveKit SFU** — separate Go binary (external process); voice media plane; Relay server handles signaling via existing WebSocket
7. **Push Notification Service** — VAPID Web Push from Axum server; optional layer; mobile push requires APNs/FCM (platform constraint)

### Critical Pitfalls

See `.planning/research/PITFALLS.md` for full details with verification checklists.

1. **CORS origin hell (Tauri + Capacitor)** — `tauri://localhost` and `capacitor://localhost` are different origins than same-origin browser requests. Add explicit CORS allowlist to Axum before any native app work begins; handle OPTIONS preflight. Recovery is easy but retroactive fixing is painful. Address in the Desktop phase.

2. **WebSocket death on mobile background** — iOS/Android kill WebSocket connections within ~180 seconds of backgrounding. Current code has no heartbeat. Implement server-side ping/pong (every 30s, 10s timeout), presence timeout (60s without heartbeat = offline), and a `Resume` message with `last_seen_message_id` for reconnection. Add before mobile, partially in desktop phase.

3. **E2EE multi-device design** — Single-device E2EE is straightforward; multi-device requires per-device keypairs, a device registry, and fan-out encryption. Retrofitting this after shipping single-device design requires rewriting the entire encryption layer. Design for per-device from day one: `device_id` in sessions, per-device key records in `user_public_keys`. Recovery cost: HIGH.

4. **Encryption-at-rest key next to database** — Storing the ChaCha20 key in a config file in the same directory as `relay.db` provides zero additional security. Key must be provided at startup separately (stdin, environment variable, or separate secrets file outside the data directory). Design the key input mechanism before implementing encryption.

5. **Building a custom WebRTC SFU** — Do not build a custom SFU. NAT traversal alone accounts for 80% of WebRTC connectivity failures. Use LiveKit. Deploy it as a separate process. Voice channel work must not start without a running LiveKit instance to test against. Recovery cost: HIGH (weeks of wasted work).

## Implications for Roadmap

Architecture research provides an explicit dependency-driven build order. This should map directly to roadmap phases.

### Phase 1: Foundation & Direct Messages

**Rationale:** DMs are the #1 missing feature by user value. They establish the conversation data model that E2EE will build on. Threads are included here because they are a trivial schema change with no new subsystems and immediately improve daily use of the existing channel experience. Unread tracking and @mentions are foundational to everything notification-related.

**Delivers:** Private conversations, threaded discussions, awareness of what you missed, ability to ping people.

**Addresses (FEATURES.md P1):** Direct Messages, Group DMs, Threads, Unread Tracking, @Mentions, Message Pinning.

**Schema additions:** `dm_conversations`, `dm_participants`, `dm_messages`, `user_public_keys` (stub), `unread_cursors`; `thread_parent_id` column on messages.

**Pitfalls to avoid:** Design `user_public_keys` with `device_id` from the start (avoid single-device E2EE trap). DMs ship with plaintext first; E2EE layers on in a later phase.

**Research flag:** Standard patterns — DM conversation models are well-documented. Thread schema (parent_id on messages table) is proven by Discord/Slack/Matrix. Low uncertainty.

### Phase 2: Desktop App

**Rationale:** Desktop app is P1 in features. It depends on making the WebSocket connection URL configurable (a small but required refactor). Desktop also provides native notifications without the complexity of mobile push infrastructure. Tauri wrapping is low-risk given the SPA is already `adapter-static`.

**Delivers:** System tray, native OS notifications, auto-start on login, always-on chat experience; makes Relay a real app instead of a browser tab.

**Addresses (FEATURES.md P1):** Desktop App, Desktop Notifications.

**Stack (STACK.md):** `tauri 2.10.x`, `@tauri-apps/api 2.x`, `tauri-plugin-notification 2.3.x`, `@sveltejs/adapter-static 3.x`.

**Architecture work:** Configurable server URL in SPA (replace hardcoded `window.location.host`), CORS allowlist in Axum (`tauri://localhost`, `http://localhost:5173`), Tauri app wired up as pure client.

**Pitfalls to avoid:** CORS origin hell — must be fully resolved before any Tauri API testing. WebSocket heartbeat — add server-side ping/pong here because Tauri apps also sleep on lid close.

**Research flag:** Standard patterns, official documentation covers this exactly. Skip `research-phase`.

### Phase 3: E2EE for DMs + Encryption at Rest

**Rationale:** E2EE depends on working DMs (Phase 1). Validating DM UX without encryption first lets you separate transport bugs from crypto bugs. Once DMs are stable, layering E2EE is client-side only — the server already stores opaque blobs, just now they are actually opaque. Encryption at rest is independent but belongs in the same phase because both are "encryption hardening" and share key management concerns.

**Delivers:** Server admin cannot read DM content or message plaintext even with database access. Genuine privacy guarantee.

**Addresses (FEATURES.md P2):** E2EE for DMs, Encryption at Rest.

**Stack (STACK.md):** Web Crypto API (X25519 + AES-256-GCM), `@noble/curves 2.0.x` (fallback), `x25519-dalek 2.0.1` (Rust), `chacha20poly1305 0.10.1` (Rust).

**Pitfalls to avoid:** Multi-device E2EE design (device registry must already exist from Phase 1). Encryption key placement (key input mechanism designed before implementation, not after). Nonce reuse in ChaCha20 (use XChaCha20 with random 192-bit nonces). E2EE private keys in localStorage (use IndexedDB with `extractable: false`).

**Research flag:** Needs careful review during planning. Multi-device E2EE is a well-known problem space but the implementation details are subtle. The fan-out encryption approach (encrypt symmetric key once per device) should be validated against actual user scenarios (desktop + mobile).

### Phase 4: Mobile App + Push Notifications

**Rationale:** Mobile requires push notifications to be viable; push requires unread tracking to know what to push (both already exist from Phases 1-2). The Tauri mobile experience is validated during desktop development, so mobile wrapping is a known quantity. This is HIGH complexity due to APNs/FCM infrastructure requirements.

**Delivers:** Chat on phone with background notifications. Removes the last reason to keep Discord installed.

**Addresses (FEATURES.md P2):** Mobile App, Push Notifications, Notification Settings.

**Stack (STACK.md):** Tauri v2 mobile (primary); Capacitor 8 (fallback if Tauri mobile plugin ecosystem insufficient). `web-push 0.11.0` (Rust, VAPID for browser/desktop). `@vite-pwa/sveltekit 1.1.0` (PWA service worker for browser push).

**Architecture:** CORS must include `capacitor://localhost` (iOS) and `http://localhost` (Android). WebSocket heartbeat + resume protocol (partially built in Phase 2) is essential. Push is an optional layer — app must work fully without push configured.

**Pitfalls to avoid:** WebSocket background death (full mobile lifecycle handling). Push notification dependency (make push opt-in; app fully functional without it). APNs token vs FCM token confusion. Testing push on physical devices (simulators don't support APNs). Notification permission UX (ask after first message, not on launch).

**Research flag:** Needs research-phase during planning. Tauri mobile is officially supported but community base is smaller than Capacitor. FCM/APNs setup for self-hosted servers is an underspecified area — each self-hoster must configure their own credentials. The push infrastructure setup guide for server operators needs to be designed carefully.

### Phase 5: Voice Channels

**Rationale:** Voice is the last reason the friend group uses Discord. It is also the highest complexity, most independent feature. Doing it last means all other features are stable and full attention can be given to voice. LiveKit is the only viable path.

**Delivers:** Voice channels that run on your server. Voice data never leaves the self-hosted infrastructure.

**Addresses (FEATURES.md P2):** Voice Channels, Self-Hosted Voice.

**Stack (STACK.md):** `livekit-server 1.9.11` (external Go binary), `livekit-api 0.4.14` (Rust SDK for room management), `livekit-client 2.17.x` (JS SDK for SvelteKit frontend).

**Architecture:** LiveKit as separate process alongside Relay binary (docker-compose.yml or install script). Relay server acts as signaling/room management layer. Voice signaling routes through existing authenticated WebSocket — no separate signaling channel needed.

**Pitfalls to avoid:** Building a custom SFU (do not deviate from LiveKit regardless of how "simple" voice seems initially). LiveKit without TLS (browsers reject insecure WebRTC). LiveKit in Docker with bridge networking (use host networking for UDP). Testing only on localhost (test on real networks — cellular, corporate VPN, symmetric NAT — early). TURN fallback validation. Voice mute state persistence on reconnect.

**Research flag:** Needs research-phase during planning. LiveKit deployment with a self-hosted Relay server (TURN configuration, SSL cert requirements for self-hosters) has thin documentation. The deployment story for users who want voice needs to be designed (docker-compose vs install script vs sidecar).

### Phase Ordering Rationale

- DMs before E2EE: You cannot encrypt DMs before DMs exist. More importantly, separating DM transport from DM encryption lets you debug each independently.
- Unread/mentions in Phase 1: These are prerequisites for meaningful notifications in later phases. Building them early means notifications are wired up correctly when desktop/mobile arrives.
- Desktop before mobile: Tauri is simpler than Capacitor (no push infrastructure). Desktop validates native app experience before tackling mobile's harder problems.
- E2EE before mobile: Key management UX should be proven on desktop before mobile complicates it with device-switching scenarios.
- Mobile before voice: Voice on mobile requires mobile to exist. The ordering also means LiveKit gets full attention without mobile integration complexity running simultaneously.
- Voice last: Highest complexity, most independent. Zero dependency on other new features. Highest risk of scope expansion.

### Research Flags

**Phases needing `research-phase` during planning:**
- **Phase 3 (E2EE):** Multi-device fan-out encryption implementation details. Key backup/recovery UX (mnemonic phrase generation). Safety number / key fingerprint verification UI.
- **Phase 4 (Mobile + Push):** Tauri mobile plugin coverage validation against actual needs. Self-hosted push notification setup guide for server operators (APNs developer account, FCM project setup). UnifiedPush viability for Android-only deployments.
- **Phase 5 (Voice):** LiveKit TURN configuration for self-hosted deployments. SSL certificate requirements and setup for non-technical self-hosters. Deploy story (docker-compose vs install script vs sidecar binary).

**Phases with standard patterns (skip `research-phase`):**
- **Phase 1 (DMs + Threads):** Well-documented patterns. Thread schema (parent_id) used by Discord/Slack/Matrix. DM conversation model is standard.
- **Phase 2 (Desktop):** Official Tauri v2 + SvelteKit guide exists and is verified. CORS handling patterns are established. No uncertainty.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Tauri v2, LiveKit, chacha20poly1305, x25519-dalek all verified via official docs and crates.io. Version corrections from original plan (Capacitor v6 -> v8, Tauri mobile preference) are well-sourced. |
| Features | HIGH | Competitor analysis comprehensive. Feature dependency graph is sound. P1/P2/P3 prioritization aligns with real user need. Anti-features list is well-reasoned. |
| Architecture | HIGH | Build order is dependency-driven and internally consistent. Component boundaries are clean and proven patterns. Schema extensions are concrete and actionable. |
| Pitfalls | MEDIUM-HIGH | Critical pitfalls are verified across multiple sources. Some E2EE key management sources are lower confidence (single source). Mobile-specific pitfalls verified via official Capacitor and Tauri docs. |

**Overall confidence:** HIGH

### Gaps to Address

- **Tauri mobile in production:** Tauri v2 mobile is officially supported but has a smaller community than Capacitor. Validate Tauri mobile meets requirements (push notifications, camera, file system) before committing fully. The contingency plan (Capacitor 8) is documented and ready.
- **Self-hosted push notification setup:** The operational burden on self-hosters who want mobile push (Apple Developer account, Firebase project, APNs/FCM credentials) is non-trivial. This needs clear documentation and potentially a "push notification setup wizard" in the admin UI. Gap to address during Phase 4 planning.
- **LiveKit deployment story:** LiveKit as a separate process conflicts with the single-binary philosophy. The docker-compose / install script / sidecar approach needs a concrete decision before Phase 5. The recommendation is to defer the optimization (ship docker-compose first, optimize later), but this needs validation with the target users.
- **Broadcast polling performance:** Current code uses `try_recv()` in a loop with 100ms intervals. Research identifies this as a performance trap — CPU usage scales linearly with channels and battery impact on mobile is significant. This is a known technical debt item that must be addressed before mobile, ideally during Phase 2.
- **Web Crypto API X25519 availability in Tauri webview:** Availability depends on the system webview version. `@noble/curves` is the fallback, but the specific Tauri webview versions that support X25519 natively should be tested during Phase 3.

## Sources

### Primary (HIGH confidence)
- [Tauri v2 SvelteKit guide](https://v2.tauri.app/start/frontend/sveltekit/) — Tauri integration, adapter-static requirements, SvelteKit version compatibility
- [Tauri v2 release page](https://v2.tauri.app/release/) — v2.10.2 current as of Feb 2026
- [Tauri v2 Notification plugin](https://v2.tauri.app/plugin/notification/) — desktop notifications API
- [LiveKit GitHub releases](https://github.com/livekit/livekit/releases) — v1.9.11 current
- [livekit-api on docs.rs](https://docs.rs/crate/livekit-api/latest) — v0.4.14, published 2026-02-16
- [LiveKit JS client SDK docs](https://docs.livekit.io/reference/client-sdk-js/) — v2.17.2
- [chacha20poly1305 on docs.rs](https://docs.rs/crate/chacha20poly1305/latest) — v0.10.1, NCC Group audited
- [x25519-dalek on crates.io](https://crates.io/crates/x25519-dalek) — v2.0.1 stable
- [@noble/curves on npm](https://www.npmjs.com/package/@noble/curves) — v2.0.1, audited
- [web-push on docs.rs](https://docs.rs/crate/web-push/latest) — v0.11.0
- [Capacitor 8 update guide](https://capacitorjs.com/docs/updating/8-0) — breaking changes from v6
- [Capacitor Push Notifications API](https://capacitorjs.com/docs/apis/push-notifications) — push implementation
- [SvelteKit adapter-static](https://svelte.dev/docs/kit/adapter-static) — SPA mode configuration
- [LiveKit self-hosting docs](https://docs.livekit.io/transport/self-hosting/) — deployment requirements
- [str0m WebRTC docs](https://docs.rs/str0m) — v0.16.2, rationale for choosing LiveKit instead
- [Capacitor Background Runner](https://capacitorjs.com/docs/apis/background-runner) — iOS background task limitations
- [Five Mistakes in Designing Mobile Push Notifications](https://www.nngroup.com/articles/push-notification/) — notification UX research (NN/g)

### Secondary (MEDIUM confidence)
- [Build for Web, Mobile & Desktop from a Single SvelteKit App](https://nsarrazin.com/blog/sveltekit-universal) — CORS handling across platforms, multi-platform SvelteKit patterns
- [LiveKit self-hosting deployments](https://docs.livekit.io/deploy/custom/deployments/) — TURN configuration, SSL requirements
- [Capacitor Push Notifications Guide](https://capawesome.io/blog/the-push-notifications-guide-for-capacitor/) — APNs/FCM token handling
- [WebSocket reconnection strategies](https://oneuptime.com/blog/post/2026-01-27-websocket-reconnection-logic/view) — mobile reconnection patterns
- [Why WebRTC Remains Deceptively Complex in 2025](https://webrtc.ventures/2025/08/why-webrtc-remains-deceptively-complex-in-2025/) — WebRTC infrastructure pitfalls
- [The Ambassador Protocol: Multi-device E2EE](https://medium.com/@TalBeerySec/the-ambassador-protocol-multi-device-e2ee-with-privacy-5c906a2d210a) — multi-device encryption approaches
- [SQLite Performance Optimization with ChaCha20](https://forwardemail.net/en/blog/docs/sqlite-performance-optimization-pragma-chacha20-production-guide) — encryption performance impact (~6% overhead)
- [@vite-pwa/sveltekit on npm](https://www.npmjs.com/package/@vite-pwa/sveltekit) — v1.1.0, PWA push notifications
- [Developing a WebRTC SFU in Rust](https://medium.com/@h3poteto/developing-a-webrtc-sfu-library-in-rust-019d467ab6c1) — SFU architecture patterns

### Tertiary (LOW confidence)
- [Web Crypto API X25519 blog](https://blog.vitalvas.com/post/2025/07/24/private-messages-x25519-aes256-gcm/) — implementation pattern; verify against official Web Crypto spec
- [E2EE in Chat Applications: A Complete Guide](https://medium.com/@siddhantshelake/end-to-end-encryption-e2ee-in-chat-applications-a-complete-guide-12b226cae8f8) — single source; key management patterns need validation
- [XChat's E2EE Critically Weak](https://cyberinsider.com/xchats-end-to-end-encryption-critically-weak-warns-researcher/) — real-world E2EE failure reference; validate specific failure modes don't apply to our design

---
*Research completed: 2026-02-28*
*Ready for roadmap: yes*
