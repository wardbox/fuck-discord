# Stack Research: Subsequent Milestone Technologies

**Domain:** Self-hosted encrypted group chat (desktop, mobile, voice, E2EE)
**Researched:** 2026-02-28
**Confidence:** MEDIUM (versions verified via official sources; some integration patterns based on community evidence)

## Existing Stack (not re-researched)

Already in place and working:
- **Server:** Rust + Axum 0.8 + tokio + rusqlite (bundled-full) + r2d2 connection pool
- **Client:** SvelteKit 2 + Svelte 5 + TypeScript + Tailwind CSS 4 + Vite 7
- **Auth:** Argon2id + session-based (invite codes, no email/phone)
- **Protocol:** JSON over WebSocket
- **Search:** SQLite FTS5
- **Embedding:** rust-embed for single binary deployment

---

## New Stack: Desktop Application

### Recommended: Tauri v2

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| tauri | 2.10.x | Desktop app shell (macOS, Linux, Windows) | Already a Rust project; Tauri v2 is the natural fit. Uses system webview (no Chromium bundle = tiny binaries). Official SvelteKit guide exists. v2 is stable with active releases through Feb 2026 |
| @tauri-apps/cli | 2.x | Build and dev tooling | Official CLI for scaffolding, building, and bundling |
| @tauri-apps/api | 2.x | Frontend JS bridge to Tauri APIs | Type-safe access to native APIs from SvelteKit |
| @sveltejs/adapter-static | 3.x | Static SPA output for Tauri | Required because Tauri does not support server-side rendering; the SvelteKit app must compile to static HTML/JS/CSS |
| tauri-plugin-notification | 2.3.x | Desktop native notifications | Official first-party plugin for OS-level toast notifications with scheduling, actions, and channels |

**Confidence:** HIGH -- Tauri v2.10.2 verified via docs.rs (published 2026-02-04), SvelteKit integration verified via official Tauri docs.

### Key Configuration

SvelteKit must be configured in SPA mode for Tauri:
- Use `@sveltejs/adapter-static` with `fallback: 'index.html'`
- Set `export const ssr = false` in root `+layout.ts`
- Tauri loads the static build from `../build` directory

### Critical Decision: Tauri v2 for Mobile Too

**Recommendation: Use Tauri v2 for BOTH desktop AND mobile instead of adding Capacitor.**

Tauri v2 now officially supports iOS and Android builds from the same codebase. Since this project is already Rust-native, using Tauri for mobile eliminates an entire technology layer (Capacitor) and keeps the stack unified:

| Factor | Tauri v2 Mobile | Capacitor 8 |
|--------|----------------|-------------|
| Language alignment | Rust (already in stack) | JavaScript/TypeScript |
| Binary size | Small (system webview) | Larger (ships webview on Android) |
| Plugin ecosystem | Smaller but growing; first-party covers core needs | Mature, extensive third-party |
| Push notifications | Via tauri-plugin-notification + community FCM/APNs plugin | Mature @capacitor-firebase/messaging |
| Mobile maturity | Production-ready since v2 stable; App Store/Play Store deployment documented | Battle-tested, years of production apps |
| Learning curve | Zero additional (already using Rust + Tauri for desktop) | New toolchain, new plugin API patterns |
| Build tooling | Single `tauri` CLI for all platforms | Separate `cap` CLI + Xcode/Android Studio |

**Why Tauri for mobile:** For a friend-group app (10-50 users), the plugin ecosystem gap is irrelevant -- you need notifications, filesystem, and camera at most. The massive upside is one codebase, one toolchain, one plugin system for desktop AND mobile. The team already knows Rust. Capacitor would add a second native bridge layer with different APIs, different build processes, and different debugging workflows for the same SvelteKit frontend.

**If Tauri mobile proves insufficient:** Fall back to Capacitor 8 (NOT v6 -- v6 is EOL). Capacitor 8.1.0 is current, requires Node.js 22+, Xcode 26+, Android Studio 2025.2.1+. The SvelteKit static build works identically with either.

**Confidence:** MEDIUM -- Tauri v2 mobile is officially supported and production-ready per Tauri docs, but the community base for mobile is smaller than Capacitor. Multiple sources confirm it works for App Store deployment.

---

## New Stack: Capacitor (Contingency Only)

If Tauri mobile proves insufficient, here is the Capacitor stack. Note the version update from the original plan.

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| @capacitor/core | 8.1.x | Mobile runtime | Current stable. v6 from original plan is now EOL. v8 requires Node.js 22+ |
| @capacitor/cli | 8.1.x | Build/sync tooling | Matches core version |
| @capacitor/ios | 8.1.x | iOS platform layer | Requires Xcode 26+, iOS 15+ deployment target |
| @capacitor/android | 8.1.x | Android platform layer | Edge-to-edge support built-in via SystemBars plugin |
| @capacitor-firebase/messaging | latest | Push notifications (FCM + APNs) | Unified FCM token for both platforms. The official @capacitor/push-notifications returns raw APNs token on iOS which Firebase cannot use |
| @sveltejs/adapter-static | 3.x | Static SPA output | Same adapter as Tauri -- one adapter serves both |

**Confidence:** HIGH -- Capacitor 8.1.0 verified via npm search results (published ~2 weeks ago as of research date). Breaking changes from v6 documented in official migration guide.

**WARNING: Capacitor v6 is outdated.** The original plan specified Capacitor v6. Capacitor is now at v8. Key breaking changes:
- Requires Node.js 22+ (up from 18)
- iOS: Swift Package Manager replaces CocoaPods for new projects
- Android: Edge-to-edge is now default (margins API removed)
- Requires Xcode 26+ and Android Studio 2025.2.1+

---

## New Stack: Voice Channels (WebRTC)

### Recommended: LiveKit (external SFU process)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| livekit-server | 1.9.11 | SFU media server (separate Go binary) | Production-ready, self-hostable, single binary. Handles all WebRTC complexity (STUN/TURN, codec negotiation, bandwidth estimation). The project constraint explicitly says "don't build an SFU" |
| livekit-api | 0.4.14 | Rust server SDK for room management | Create/manage voice rooms from Axum backend. Generate access tokens. Published 2026-02-16 |
| livekit-client (npm) | 2.17.x | JavaScript client SDK for browser/Tauri | Handles WebRTC connection, audio tracks, UI events from SvelteKit frontend |

**Confidence:** HIGH -- livekit-server v1.9.11 verified via GitHub releases (Jan 2025). livekit-api 0.4.14 verified via docs.rs (Feb 2026). JS client SDK v2.17.2 verified via official docs.

### Why LiveKit, Not str0m

str0m (v0.16.2) is a Sans I/O WebRTC library in Rust -- it gives you raw WebRTC primitives. To build an SFU with str0m you must implement:
- Network I/O (UDP sockets, STUN/TURN)
- Room/session management
- Codec negotiation
- Bandwidth estimation
- Participant state tracking
- Reconnection logic

This is "a whole company's worth of work" as the project plan notes. LiveKit does all of this out of the box. str0m is the right choice if you want to build a WebRTC product company; LiveKit is the right choice if you want voice chat working next month.

### Deployment Consideration

LiveKit is a separate Go binary, which conflicts with the "single binary" philosophy. Mitigation options:
1. **Recommended:** Ship a `docker-compose.yml` or install script that runs both `relay` and `livekit-server` side by side. Voice is an opt-in feature; users who don't need it skip LiveKit entirely.
2. **Alternative:** Bundle LiveKit as a sidecar process managed by the Relay binary (spawn on startup, kill on shutdown). Adds complexity but preserves the "download and run" experience.
3. **Defer decision:** Get voice working first with LiveKit as a separate process. Optimize deployment later.

### LiveKit Self-Hosting Requirements

- Single Go binary (`livekit-server --dev` for development)
- Requires UDP ports 7882 (WebRTC media), TCP 7880 (HTTP API), TCP 7881 (WebSocket)
- Production requires SSL certificate + domain
- No Redis needed for single-node deployment (Redis only needed for horizontal scaling)
- For 10-50 users: single node is more than sufficient

---

## New Stack: End-to-End Encryption for DMs

### Client-Side (SvelteKit/Browser)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Web Crypto API | (browser built-in) | X25519 key exchange + AES-256-GCM encryption | Native browser API, no library needed for core operations. Supported in all modern browsers. Zero bundle size impact |
| @noble/curves | 2.0.x | X25519 key generation and ECDH | Web Crypto API supports X25519 natively in modern browsers, but @noble/curves provides a fallback and more ergonomic API. Audited, minimal, pure JS. v2.0.1 is current stable |

**Confidence:** MEDIUM -- Web Crypto API X25519 support is relatively recent (Chrome 113+, Firefox 130+, Safari 17.4+). For Tauri's webview, availability depends on the system webview version. @noble/curves is the safety net.

### Server-Side (Rust)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| x25519-dalek | 2.0.1 | X25519 key exchange (server-side key generation if needed) | Stable release from dalek-cryptography. v3.0.0 is in pre-release (pre.6 as of Feb 2026); use v2 for stability |

**Confidence:** HIGH -- x25519-dalek 2.0.1 is the stable release, verified via crates.io. v3 pre-releases are active but not production-ready.

### E2EE Protocol Design

The server should NEVER see plaintext DM content. The protocol:
1. Each user generates an X25519 keypair; public key is uploaded to server
2. To send a DM, sender derives shared secret via ECDH (their private key + recipient's public key)
3. Message encrypted with AES-256-GCM using derived key
4. Server stores ciphertext only
5. Recipient derives same shared secret and decrypts

For group DMs: use a per-group symmetric key, encrypted to each member's public key (similar to Signal's Sender Keys).

---

## New Stack: Encryption at Rest

### Recommended: chacha20poly1305 crate

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| chacha20poly1305 | 0.10.1 | Encrypt stored messages/files on disk | RustCrypto project, audited by NCC Group (no significant findings). Pure Rust, optional AVX2 acceleration. ChaCha20 chosen over AES because it's fast in software without hardware AES-NI (important for ARM servers, Raspberry Pi self-hosting) |
| rand | 0.8.x | Cryptographically secure nonce generation | Already in the project for auth; reuse for encryption nonces |

**Confidence:** HIGH -- chacha20poly1305 0.10.1 verified via crates.io/docs.rs. NCC Group audit confirmed.

### What Gets Encrypted

- Message content in SQLite (encrypt before INSERT, decrypt after SELECT)
- Uploaded files on disk
- NOT metadata (timestamps, channel IDs, user IDs) -- needed for indexing/queries
- NOT FTS5 index -- encrypting the search index defeats the purpose. Accept this tradeoff or implement encrypted search later

### Key Management

- Derive encryption key from server master passphrase using Argon2id (already in stack)
- Store master key hash, not the key itself
- Key rotation: re-encrypt existing data with new key (offline migration)

---

## New Stack: Push Notifications

### Desktop (via Tauri)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| tauri-plugin-notification | 2.3.x | OS-native notifications | First-party Tauri plugin. Toast notifications with actions and scheduling |

### Mobile (via Tauri or Capacitor)

If using Tauri mobile: Use the same `tauri-plugin-notification` for local notifications. For push notifications while app is closed, a community plugin (tauri-plugin-notifications by Choochmeque) adds FCM/APNs support but is third-party.

If using Capacitor: Use `@capacitor-firebase/messaging` for unified FCM/APNs token handling.

### Server-Side Push

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| web-push (Rust crate) | 0.11.0 | Send Web Push notifications from Axum backend | Implements RFC8188 encryption + VAPID auth. Works with FCM, Mozilla push, Edge. Published 2025-02-22 |

**Confidence:** MEDIUM -- web-push 0.11.0 verified via docs.rs. The crate notes it's "still in active development" with potential breaking changes. For mobile push, the path depends on Tauri vs Capacitor decision.

### Web Push for Browser Users

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| @vite-pwa/sveltekit | 1.1.0 | PWA service worker for web push | Zero-config PWA plugin. Handles service worker registration, caching, and push notification scaffolding |

**Confidence:** MEDIUM -- @vite-pwa/sveltekit 1.1.0 verified via npm. Push notification support is a PWA capability, but configuration details are thin in docs.

---

## New Stack: Threaded Conversations

No new libraries needed. Threads are a data model and UI concern:
- **Database:** Add `parent_message_id` column to messages table (SQLite, already in stack)
- **Protocol:** Extend existing WebSocket JSON protocol with thread-related message types
- **UI:** SvelteKit component work (thread panel, thread indicators)

**Confidence:** HIGH -- Standard pattern, no new dependencies.

---

## Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| tauri-cli | Tauri build/dev for all platforms | `cargo install tauri-cli` or `npm install -D @tauri-apps/cli` |
| Android Studio 2025.2.1+ | Android builds (if using Tauri mobile) | Required for Tauri Android builds |
| Xcode 26+ | iOS builds | Required for both Tauri and Capacitor iOS builds |
| livekit-server | Local voice dev server | `livekit-server --dev` for development with placeholder keys |

---

## Installation

### Rust (server/Cargo.toml additions)

```toml
# Encryption at rest
chacha20poly1305 = "0.10"

# E2EE key exchange (server-side, if generating keys server-side)
x25519-dalek = "2.0"

# Push notifications (server-side Web Push)
web-push = "0.11"

# LiveKit room management
livekit-api = "0.4"
```

### JavaScript (client/package.json additions)

```bash
# Tauri (desktop + mobile)
npm install -D @tauri-apps/cli
npm install @tauri-apps/api @tauri-apps/plugin-notification

# LiveKit voice client
npm install livekit-client

# E2EE (fallback for older webviews)
npm install @noble/curves

# PWA/push notifications for web users
npm install -D @vite-pwa/sveltekit
```

### Contingency: Capacitor (only if Tauri mobile proves insufficient)

```bash
npm install @capacitor/core @capacitor/cli
npm install @capacitor/ios @capacitor/android
npm install @capacitor-firebase/messaging firebase
npx cap init
npx cap add ios
npx cap add android
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Tauri v2 (desktop + mobile) | Tauri desktop + Capacitor 8 mobile | If Tauri mobile plugin ecosystem proves insufficient for push notifications or camera access |
| LiveKit (external SFU) | str0m (Rust WebRTC library) | If you want to embed WebRTC directly in the Relay binary with no external process. Requires months of SFU implementation work |
| LiveKit | mediasoup (C++ SFU with Node.js wrapper) | If you want a non-Go SFU. But adds Node.js dependency and C++ build complexity |
| chacha20poly1305 | SQLCipher (transparent SQLite encryption) | If you want full-database encryption instead of per-field. But SQLCipher replaces rusqlite's bundled SQLite, which may break FTS5 bundled-full |
| @noble/curves | libsodium-wrappers | If you need broader crypto primitives. But heavier dependency (WASM binary) for something Web Crypto already handles |
| web-push (Rust) | fcm-v1 + apns2 (separate crates) | If you need direct FCM/APNs control without Web Push. More work, more control |
| @vite-pwa/sveltekit | Manual service worker | If you need full control over caching strategy and push handling |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Capacitor v6 | EOL. Capacitor is now at v8. Using v6 means missing security patches and modern Android/iOS requirements | Capacitor 8 if needed, or Tauri v2 mobile |
| webrtc-rs | Incomplete, less maintained than str0m for SFU use cases | LiveKit (if you want a product) or str0m (if you want a library) |
| x25519-dalek v3 pre-release | In active pre-release development (pre.6 as of Feb 2026). Breaking changes likely | x25519-dalek 2.0.1 (stable) |
| Building your own SFU with str0m | "A whole company's worth of work" for a friend-group chat app | LiveKit self-hosted |
| SQLCipher for encryption at rest | Replaces rusqlite's bundled SQLite, may conflict with FTS5 bundled-full feature flag. Also adds C dependency | chacha20poly1305 for per-field encryption |
| Electron for desktop | Bundles Chromium (~150MB). Tauri uses system webview (~5MB) | Tauri v2 |
| React Native for mobile | Completely different UI framework. Would require rewriting the frontend | Tauri v2 mobile (same SvelteKit frontend) |

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| tauri 2.10.x | @sveltejs/adapter-static 3.x | Tauri requires static SPA output; adapter-static with `fallback: 'index.html'` |
| tauri 2.10.x | SvelteKit 2.x / Svelte 5.x | Verified in official Tauri docs (tested with SK 2.20.4 / Svelte 5.25.8) |
| livekit-client 2.17.x | livekit-server 1.9.x | Client SDK v2 works with server v1.9+ |
| livekit-api 0.4.x | livekit-server 1.9.x | Rust server SDK matches server protocol |
| chacha20poly1305 0.10.x | Rust 2021 edition | Uses RustCrypto trait ecosystem (aead crate) |
| @noble/curves 2.0.x | All modern browsers | Pure JS, no native dependencies |
| web-push 0.11.x | Requires OpenSSL (or vendored feature) | May need `openssl = { version = "0.10", features = ["vendored"] }` for single binary builds |
| Capacitor 8.1.x (contingency) | Node.js 22+ | Breaking from v6 which worked with Node 18 |

## Sources

- [Tauri v2 SvelteKit guide](https://v2.tauri.app/start/frontend/sveltekit/) -- official docs, verified SvelteKit 2.20.4 / Svelte 5.25.8 (HIGH confidence)
- [Tauri v2 release page](https://v2.tauri.app/release/) -- v2.10.2 current (HIGH confidence)
- [Tauri notification plugin](https://v2.tauri.app/plugin/notification/) -- v2.3.3 (HIGH confidence)
- [Tauri v2 stable release announcement](https://v2.tauri.app/blog/tauri-20/) -- mobile support confirmed (HIGH confidence)
- [chacha20poly1305 on docs.rs](https://docs.rs/crate/chacha20poly1305/latest) -- v0.10.1, NCC Group audited (HIGH confidence)
- [str0m on docs.rs](https://docs.rs/str0m) -- v0.16.2, Sans I/O WebRTC (HIGH confidence)
- [LiveKit GitHub releases](https://github.com/livekit/livekit/releases) -- v1.9.11 (HIGH confidence)
- [livekit-api on docs.rs](https://docs.rs/crate/livekit-api/latest) -- v0.4.14, published 2026-02-16 (HIGH confidence)
- [LiveKit JS client SDK docs](https://docs.livekit.io/reference/client-sdk-js/) -- v2.17.2 (HIGH confidence)
- [LiveKit self-hosting docs](https://docs.livekit.io/transport/self-hosting/) -- single binary, no Redis for single-node (MEDIUM confidence)
- [x25519-dalek on crates.io](https://crates.io/crates/x25519-dalek) -- v2.0.1 stable, v3 pre-release (HIGH confidence)
- [@noble/curves on npm](https://www.npmjs.com/package/@noble/curves) -- v2.0.1, audited (HIGH confidence)
- [web-push on docs.rs](https://docs.rs/crate/web-push/latest) -- v0.11.0, published 2025-02-22 (HIGH confidence)
- [Capacitor 8 update guide](https://capacitorjs.com/docs/updating/8-0) -- breaking changes from v6 (HIGH confidence)
- [@vite-pwa/sveltekit on npm](https://www.npmjs.com/package/@vite-pwa/sveltekit) -- v1.1.0 (MEDIUM confidence)
- [Web Crypto API X25519 blog](https://blog.vitalvas.com/post/2025/07/24/private-messages-x25519-aes256-gcm/) -- implementation pattern (LOW confidence)
- [SvelteKit universal build guide](https://nsarrazin.com/blog/sveltekit-universal) -- multi-platform SvelteKit pattern (MEDIUM confidence)
- [Capacitor SvelteKit guide](https://capacitorjs.com/solution/svelte) -- official Capacitor + Svelte docs (HIGH confidence)
- [E2EE chat with Web Crypto](https://getstream.io/blog/web-crypto-api-chat/) -- implementation reference (MEDIUM confidence)

---
*Stack research for: Relay (fuck-discord) subsequent milestone*
*Researched: 2026-02-28*
