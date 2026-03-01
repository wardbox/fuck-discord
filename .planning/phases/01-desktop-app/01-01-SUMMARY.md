---
phase: 01-desktop-app
plan: 01
subsystem: desktop
tags: [tauri, tauri-v2, desktop, cors, bearer-auth, platform-detection, sveltekit]

# Dependency graph
requires:
  - phase: 00-scaffolding
    provides: SvelteKit SPA client and Rust/Axum server with auth
provides:
  - Tauri v2 desktop app shell with window config and plugins
  - Platform-aware config module (isTauri, getServerUrl, getWsUrl, fetchWithAuth)
  - Cross-origin support for Tauri origins in server CORS
  - Authorization Bearer token fallback in auth middleware
  - Server URL entry screen for first-launch configuration
affects: [01-02-PLAN, desktop-notifications, tray-icon, mobile]

# Tech tracking
tech-stack:
  added: ["@tauri-apps/cli", "@tauri-apps/api", "@tauri-apps/plugin-notification", "@tauri-apps/plugin-store", "tauri v2", "tauri-plugin-notification", "tauri-plugin-store"]
  patterns: [platform-aware-config, fetchWithAuth-wrapper, bearer-token-auth, server-url-persistence]

key-files:
  created:
    - src-tauri/Cargo.toml
    - src-tauri/tauri.conf.json
    - src-tauri/src/lib.rs
    - src-tauri/src/main.rs
    - src-tauri/capabilities/default.json
    - src-tauri/build.rs
    - client/src/lib/config.ts
  modified:
    - server/src/auth/middleware.rs
    - server/src/handlers/mod.rs
    - client/src/lib/stores/auth.svelte.ts
    - client/src/lib/stores/connection.svelte.ts
    - client/src/lib/stores/messages.svelte.ts
    - client/src/routes/+layout.svelte
    - package.json
    - client/package.json

key-decisions:
  - "Used '__TAURI_INTERNALS__' in window check for isTauri() detection instead of __TAURI__ for Tauri v2 compatibility"
  - "fetchWithAuth reads session from localStorage directly to avoid circular dependency with auth store"
  - "Server URL entry screen placed in root +layout.svelte to gate all child routes until config initialized"
  - "Change Server button only visible when in Tauri mode and not authenticated"

patterns-established:
  - "Platform config: All API calls route through fetchWithAuth() from client/src/lib/config.ts"
  - "WS URL: All WebSocket connections use getWsUrl() from config module"
  - "Auth dual-path: Server auth middleware checks Authorization Bearer header first, falls back to cookie"
  - "Tauri store: User settings persisted via @tauri-apps/plugin-store in settings.json"

requirements-completed: [DESK-01, DESK-04]

# Metrics
duration: 13min
completed: 2026-03-01
---

# Phase 1 Plan 1: Tauri Desktop Shell Summary

**Tauri v2 desktop shell with platform-aware config, Bearer auth fallback, cross-origin CORS, and first-launch server URL entry screen**

## Performance

- **Duration:** 13 min
- **Started:** 2026-03-01T08:01:03Z
- **Completed:** 2026-03-01T08:14:14Z
- **Tasks:** 3
- **Files modified:** 21

## Accomplishments
- Tauri v2 project structure created with notification and store plugins, ready for dev/build
- All client API/WS calls now route through platform-aware config module (fetchWithAuth, getWsUrl)
- Server accepts cross-origin requests from Tauri origins and authenticates via Bearer token
- First-launch server URL entry screen validates connectivity before storing URL
- Existing browser-based usage completely unchanged (same-origin behavior preserved)

## Task Commits

Each task was committed atomically:

1. **Task 1: Initialize Tauri project and update server for cross-origin support** - `b51a267` (feat)
2. **Task 2: Create platform-aware config module, fetchWithAuth wrapper, and update all client stores** - `44791a7` (feat)
3. **Task 3: Create server URL entry screen and app initialization flow** - `0a79de4` (feat)

## Files Created/Modified
- `src-tauri/Cargo.toml` - Tauri desktop app crate with plugin dependencies
- `src-tauri/tauri.conf.json` - Tauri configuration linking to SvelteKit build
- `src-tauri/src/lib.rs` - Tauri app setup with notification and store plugins
- `src-tauri/src/main.rs` - Desktop app entry point
- `src-tauri/capabilities/default.json` - Window, notification, and store permissions
- `src-tauri/build.rs` - Tauri build script
- `src-tauri/icons/` - Placeholder app icons (32x32, 128x128, 256x256)
- `client/src/lib/config.ts` - Platform-aware config with isTauri, getServerUrl, getWsUrl, getApiUrl, fetchWithAuth, setServerUrl, clearServerUrl
- `client/src/lib/stores/auth.svelte.ts` - All 4 fetch calls updated to use fetchWithAuth
- `client/src/lib/stores/connection.svelte.ts` - WebSocket URL from getWsUrl() instead of window.location
- `client/src/lib/stores/messages.svelte.ts` - loadHistory uses fetchWithAuth
- `client/src/routes/+layout.svelte` - Config init gate, server URL entry screen, Change Server button
- `server/src/auth/middleware.rs` - Bearer token extraction as primary auth, cookie as fallback
- `server/src/handlers/mod.rs` - CORS allows tauri://localhost and https://tauri.localhost
- `package.json` - dev:desktop and build:desktop scripts, @tauri-apps/cli devDep
- `client/package.json` - @tauri-apps/api, plugin-notification, plugin-store deps

## Decisions Made
- Used `'__TAURI_INTERNALS__' in window` for Tauri v2 detection (v2 uses `__TAURI_INTERNALS__` not `__TAURI__`)
- `fetchWithAuth` reads session from `localStorage` directly rather than importing auth store, avoiding circular dependency
- Root `+layout.svelte` gates all child rendering on `configReady`, ensuring `initConfig()` completes before any API calls
- "Change Server" button hidden when authenticated to avoid disrupting active sessions
- Added `defaults: {}` to Tauri store options to satisfy the `StoreOptions` type requirement

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed StoreOptions type for @tauri-apps/plugin-store**
- **Found during:** Task 2 (config.ts creation)
- **Issue:** `StoreOptions` type requires a `defaults` property; passing `{ autoSave: true }` alone caused type error
- **Fix:** Added `defaults: {}` to all `load()` calls
- **Files modified:** client/src/lib/config.ts
- **Verification:** svelte-check passes with 0 errors
- **Committed in:** 44791a7 (Task 2 commit)

**2. [Rule 1 - Bug] Fixed isTauri() window type cast**
- **Found during:** Task 2 (config.ts creation)
- **Issue:** Casting `window` to `Record<string, unknown>` caused TypeScript error due to insufficient type overlap
- **Fix:** Used `'__TAURI_INTERNALS__' in window` operator instead of type assertion
- **Files modified:** client/src/lib/config.ts
- **Verification:** svelte-check passes with 0 errors
- **Committed in:** 44791a7 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both fixes were TypeScript type corrections needed to pass svelte-check. No scope creep.

## Issues Encountered
- `client/build/` directory did not exist, causing `cargo check` to fail on the `RustEmbed` derive macro. Created the directory as a pre-existing requirement for the server to compile. This is a known characteristic of the project (build dir only exists after `npm run build:client`).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Tauri shell ready for notifications (01-02 plan: DESK-02, DESK-03)
- All API/WS infrastructure supports configurable remote server
- Desktop dev workflow available via `npm run dev:desktop`
- Plugin registration in lib.rs ready for tray-icon additions

## Self-Check: PASSED

All 8 created files verified. All 3 task commits verified (b51a267, 44791a7, 0a79de4).

---
*Phase: 01-desktop-app*
*Completed: 2026-03-01*
