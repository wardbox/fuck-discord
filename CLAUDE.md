# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Relay — self-hosted encrypted group chat. Rust/Axum backend + SvelteKit frontend compiled into a single binary.

## Commands

```bash
# Development (run in separate terminals)
npm run dev:client          # SvelteKit dev server (Vite HMR, proxies /api + /ws → :3000)
npm run dev:server          # Rust server with auto-reload (cargo watch)

# Build
npm run build               # Client → server (client embedded via rust-embed)
npm run build:client        # SvelteKit static build only
npm run build:server        # cargo build --release only

# Tests (integration tests, spin up temp server + SQLite per test)
cd server && cargo test      # All tests
cd server && cargo test auth # Single test module
cd server && cargo test test_register_and_login # Single test

# Type checking
cd client && npm run check   # svelte-check + TypeScript

# Create invite code
cargo run -- invite --max-uses 5

# Run server directly
cargo run -- --port 3000 --database relay.db
```

## Architecture

**Monorepo:** `/server` (Rust, Cargo) + `/client` (SvelteKit, npm). Root `package.json` orchestrates both.

**Single binary deployment:** SvelteKit builds to `/client/build`, which `rust-embed` embeds into the Rust binary. The server serves the SPA as a fallback route.

**Database:** SQLite via `rusqlite` (bundled-full, zero system deps). r2d2 connection pool (max 8). WAL mode. Hand-written migrations in `server/migrations/` applied on startup via `run_migrations()`. FTS5 for message search.

**Auth flow:** Invite code + username/password → Argon2id hash → session token (32-byte hex, 30-day expiry) → `relay_session` cookie. `require_auth` middleware extracts `AuthUser` from cookie on protected routes.

**WebSocket:** JSON protocol. First message must be `authenticate` with session token (10s timeout). Per-channel broadcast channels in `AppState`. Messages fan out to all subscribers.

**IDs:** ULIDs everywhere (text columns in SQLite, sortable).

## Key Patterns

- **Server error handling:** `AppError` enum in `server/src/error.rs` implements `IntoResponse`. Database/pool errors log internally, return generic 500. Use `?` with `AppError` in handlers.
- **State:** `AppState` holds db pool, per-channel broadcast senders, and uploads dir path. Passed as Axum state.
- **Client stores:** Svelte 5 runes (`$state`, `$derived`) in `client/src/lib/stores/`. Each store is a module with reactive state + methods. `connection.svelte.ts` is the WebSocket hub that dispatches to other stores.
- **Client routing:** SPA with `/` (login) and `/app/[channelId]` (chat). Static adapter with `index.html` fallback.
- **Vite proxy:** Dev mode proxies `/api`, `/ws`, `/uploads` to `localhost:3000` (configured in `client/vite.config.ts`).

## WS Protocol Types

Client→Server and Server→Client message types are defined in:
- Rust: `server/src/ws/protocol.rs` (`ClientMessage`, `ServerMessage` enums)
- TypeScript: `client/src/lib/protocol/types.ts`

Keep these in sync when adding new message types.

## Test Infrastructure

Integration tests in `server/tests/`. `common/mod.rs` provides:
- `TestServer` — starts server on random port with temp SQLite
- `TestClient` — HTTP client wrapping reqwest with cookie jar
- `WsClient` — WebSocket client for testing real-time flows

Tests create users via registration (need invite codes), so test helpers handle invite creation internally.
