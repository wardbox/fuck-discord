# Relay

## What This Is

A self-hosted, privacy-first group chat platform that replaces Discord. No email, no phone number, no face scan — just invite codes and usernames. Built for a 10-25 person friend group first, but designed so anyone can spin up their own server. Text-first, keyboard-driven, compact by default — IRC reborn for 2025.

## Core Value

People can communicate with their friends in real-time across text, voice, and DMs without surrendering personal identity to a corporation.

## Requirements

### Validated

- ✓ Invite-based registration (no email/phone) — existing
- ✓ Text channels with real-time messaging — existing
- ✓ Reactions, typing indicators, presence — existing
- ✓ File uploads (images, clips) — existing
- ✓ Full-text search (FTS5) — existing
- ✓ Message editing and deletion — existing
- ✓ Session-based auth with Argon2id — existing

### Active

- [ ] Desktop application (Tauri v2) — #1 blocker to switching
- [ ] Mobile application (Capacitor v6) — #1 blocker to switching
- [ ] Voice channels (WebRTC) — important for gaming sessions and hangouts
- [ ] Direct messages and group DMs
- [ ] Threaded conversations within channels
- [ ] Configurable notifications (unreads + push, per-channel controls)
- [ ] Encryption at rest (ChaCha20-Poly1305)
- [ ] E2EE for DMs (X25519 + AES-256-GCM)

### Out of Scope

- Video/screen sharing — complexity too high for v1, defer until voice is solid
- Federation between servers — interesting future direction, not v1
- Bot/plugin API — defer to post-v1
- Forum-style channels — defer to post-v1
- IRC protocol compatibility — defer to post-v1
- OAuth / SSO — invite codes are the auth model, intentionally

## Context

The group is leaving Discord because Discord now requires facial identity verification. This isn't a side project curiosity — it's driven by a real need to get off a platform that's become hostile to user privacy. The existing codebase has a working chat app: Rust/Axum backend with SQLite, SvelteKit frontend compiled into a single binary via rust-embed. WebSocket-based real-time protocol is functional. The gap is making it feel like a real product (native apps, voice, notifications) rather than a browser-tab experiment.

The server is designed as a single self-hostable binary. Anyone should be able to download it, run it, and have their own chat server. No cloud dependencies, no external services required for core functionality.

## Constraints

- **Tech stack**: Rust + Axum (server), SvelteKit + Svelte 5 (client), Tauri v2 (desktop), Capacitor v6 (mobile) — already committed
- **Database**: SQLite via rusqlite (bundled-full) — single-file, zero system deps, already in use
- **Wire format**: JSON over WebSocket — already implemented, keep for v1
- **Auth model**: Invite codes + username/passphrase only — no email, no phone, no third-party OAuth
- **Deployment**: Single binary, self-hostable — no Docker/Kubernetes requirement
- **Voice**: Use existing WebRTC infrastructure (LiveKit or str0m) — don't build an SFU from scratch
- **Scale target**: 10-50 concurrent users, not thousands — optimize for simplicity over throughput

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| SvelteKit over SolidJS | Better multi-platform story (Tauri + Capacitor guides) | — Pending |
| Capacitor over native mobile | One codebase for iOS + Android | — Pending |
| JSON over Protobuf for WS | Simplicity and debuggability, good enough for friend-group scale | — Pending |
| rusqlite over sqlx | FTS5 works without friction, bundled-full for zero system deps | — Pending |
| Don't build voice SFU | Use LiveKit or str0m — voice infra is a whole company's worth of work | — Pending |
| Single binary via rust-embed | Simplest deployment story for self-hosters | — Pending |

---
*Last updated: 2026-02-28 after initialization*
