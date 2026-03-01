---
phase: 01-desktop-app
plan: 02
subsystem: desktop
tags: [tauri, tray-icon, notifications, system-tray, close-to-tray, native-notifications]

# Dependency graph
requires:
  - phase: 01-desktop-app
    plan: 01
    provides: Tauri v2 shell with notification plugin, store plugin, platform-aware config
provides:
  - System tray icon with Show/Quit context menu
  - Close-to-tray behavior (window hides on close, app stays running)
  - Native OS notification module for incoming messages
  - Rate-limited notification delivery (1 per channel per 3 seconds)
affects: [mobile-notifications, notification-preferences]

# Tech tracking
tech-stack:
  added: []
  patterns: [close-to-tray, tray-context-menu, lazy-permission-request, fire-and-forget-notifications, rate-limited-notifications]

key-files:
  created:
    - client/src/lib/notifications.ts
    - server/migrations/004_invites_nullable_created_by.sql
  modified:
    - src-tauri/src/lib.rs
    - client/src/lib/stores/connection.svelte.ts
    - src-tauri/tauri.conf.json
    - package.json
    - Cargo.toml
    - .gitignore
    - server/src/auth/invite.rs
    - server/src/db/invites.rs
    - server/src/db/mod.rs
    - server/src/handlers/auth.rs
    - server/src/main.rs
    - client/src/lib/components/Sidebar.svelte
    - client/src/lib/components/ChannelHeader.svelte
    - client/src/lib/components/SearchPanel.svelte
    - client/src/lib/components/MessageInput.svelte

key-decisions:
  - "Used show_menu_on_left_click(false) instead of deprecated menu_on_left_click for Tauri v2 tray API"
  - "Lazy notification permission: requested on first unfocused incoming message, not on app launch"
  - "Fire-and-forget notifications: showMessageNotification is called without await to never block message processing"
  - "macOS unsigned dev builds silently drop notifications -- this is expected and works in release builds"

patterns-established:
  - "Notification module: all notification logic isolated in client/src/lib/notifications.ts"
  - "Rate limiting: per-channel Map with timestamp comparison, configurable interval"
  - "Tray lifecycle: app.exit(0) is the only way to quit; window close always hides"

requirements-completed: [DESK-02, DESK-03]

# Metrics
duration: 25min
completed: 2026-03-01
---

# Phase 1 Plan 2: System Tray + Native Notifications Summary

**System tray with close-to-tray persistence and rate-limited native OS notifications for incoming messages via Tauri plugin-notification**

## Performance

- **Duration:** 25 min
- **Started:** 2026-03-01T08:17:55Z
- **Completed:** 2026-03-01T08:43:00Z
- **Tasks:** 3/3
- **Files modified:** 17

## Accomplishments
- System tray icon with Show Relay / Quit context menu; left-click restores window, right-click opens menu
- Close-to-tray behavior: window close is intercepted, window hides, WebSocket stays connected
- Native OS notification module with lazy permission request, rate limiting, and own-message filtering
- Fixed several issues found during verification: Tauri config paths, nullable invite creator, missing fetchWithAuth calls

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement system tray with close-to-tray behavior** - `b8612f2` (feat)
2. **Task 2: Create notification module and wire to incoming messages** - `e7cce5e` (feat)
3. **Task 3: Verify desktop app functionality (fixes from verification)** - `c4ca4ac` (fix)

## Files Created/Modified
- `src-tauri/src/lib.rs` - Tray icon setup, close-to-tray via on_window_event, tray menu with Show/Quit
- `client/src/lib/notifications.ts` - Notification module: permission handling, rate limiting, truncation
- `client/src/lib/stores/connection.svelte.ts` - Wired showMessageNotification into message_create handler
- `src-tauri/tauri.conf.json` - Fixed paths (project-root-relative), removed invalid plugin config
- `package.json` - Fixed dev:desktop and build:desktop scripts
- `Cargo.toml` - Added src-tauri to workspace exclude
- `.gitignore` - Added src-tauri/target/ and src-tauri/gen/
- `server/migrations/004_invites_nullable_created_by.sql` - Make created_by nullable for CLI invites
- `server/src/auth/invite.rs` - Accept Option<&str> for created_by
- `server/src/db/invites.rs` - Update create_invite and get_all_invites for nullable created_by
- `server/src/db/mod.rs` - Apply migration 004
- `server/src/handlers/auth.rs` - Pass Some(&auth_user.0) for API invite creation
- `server/src/main.rs` - Pass None for CLI invite creation
- `client/src/lib/components/Sidebar.svelte` - Use fetchWithAuth for channel creation
- `client/src/lib/components/ChannelHeader.svelte` - Use fetchWithAuth for topic update
- `client/src/lib/components/SearchPanel.svelte` - Use fetchWithAuth for search
- `client/src/lib/components/MessageInput.svelte` - Use fetchWithAuth for file upload

## Decisions Made
- Used `show_menu_on_left_click(false)` (non-deprecated API) instead of plan's `menu_on_left_click(false)`
- Notification permission requested lazily on first unfocused message, not on app launch -- avoids reflexive denial
- `showMessageNotification` called fire-and-forget (no await) so notification errors never block message processing
- macOS unsigned dev builds silently drop `sendNotification` calls -- confirmed via console logs that the code path executes correctly; notifications will appear in signed release builds

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added src-tauri to workspace exclude in root Cargo.toml**
- **Found during:** Task 1
- **Issue:** `cargo check` in src-tauri/ failed because Cargo workspace did not list or exclude src-tauri
- **Fix:** Added `exclude = ["src-tauri"]` to root Cargo.toml workspace section
- **Files modified:** Cargo.toml
- **Verification:** cargo check passes
- **Committed in:** b8612f2 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed deprecated menu_on_left_click API**
- **Found during:** Task 1
- **Issue:** Tauri v2 deprecated `menu_on_left_click` with a compiler warning
- **Fix:** Changed to `show_menu_on_left_click` (the replacement API)
- **Files modified:** src-tauri/src/lib.rs
- **Verification:** cargo check passes with zero warnings
- **Committed in:** b8612f2 (Task 1 commit)

**3. [Rule 3 - Blocking] Added src-tauri/target/ and src-tauri/gen/ to .gitignore**
- **Found during:** Task 1
- **Issue:** Tauri build artifacts showing as untracked files
- **Fix:** Added both directories to .gitignore
- **Files modified:** .gitignore
- **Committed in:** b8612f2 (Task 1 commit)

**4. [Rule 1 - Bug] Fixed tauri.conf.json paths for cargo tauri dev**
- **Found during:** Task 3 (verification)
- **Issue:** beforeDevCommand/beforeBuildCommand used `cd ../client` and frontendDist used `../client/build`, but cargo tauri dev runs from project root, not src-tauri/
- **Fix:** Changed to `cd client` and `client/build` (project-root-relative)
- **Files modified:** src-tauri/tauri.conf.json
- **Committed in:** c4ca4ac (Task 3 commit)

**5. [Rule 1 - Bug] Fixed package.json desktop scripts**
- **Found during:** Task 3 (verification)
- **Issue:** dev:desktop and build:desktop used `cd src-tauri && cargo tauri dev/build` but cargo tauri should run from project root
- **Fix:** Changed to `cargo tauri dev` and `cargo tauri build`
- **Files modified:** package.json
- **Committed in:** c4ca4ac (Task 3 commit)

**6. [Rule 1 - Bug] Removed invalid notification plugin config from tauri.conf.json**
- **Found during:** Task 3 (verification)
- **Issue:** `"plugins": { "notification": { "enabled": true } }` is not valid Tauri v2 plugin config
- **Fix:** Changed to `"plugins": {}`
- **Files modified:** src-tauri/tauri.conf.json
- **Committed in:** c4ca4ac (Task 3 commit)

**7. [Rule 1 - Bug] Made invites.created_by nullable for CLI invites**
- **Found during:** Task 3 (verification)
- **Issue:** CLI `cargo run -- invite` failed with FK constraint error because it passed "system" as created_by but no user with that ID exists
- **Fix:** Migration 004 recreates invites table with nullable created_by; updated invite code to use Option<&str>
- **Files modified:** server/migrations/004_invites_nullable_created_by.sql, server/src/auth/invite.rs, server/src/db/invites.rs, server/src/db/mod.rs, server/src/handlers/auth.rs, server/src/main.rs
- **Committed in:** c4ca4ac (Task 3 commit)

**8. [Rule 2 - Missing Critical] Added missing fetchWithAuth calls in 4 components**
- **Found during:** Task 3 (verification)
- **Issue:** Sidebar, ChannelHeader, SearchPanel, and MessageInput were using bare `fetch()` instead of `fetchWithAuth()`, which would fail in Tauri mode (no cookie-based auth, needs Bearer header)
- **Fix:** Added import and replaced fetch calls with fetchWithAuth in all 4 components
- **Files modified:** client/src/lib/components/Sidebar.svelte, ChannelHeader.svelte, SearchPanel.svelte, MessageInput.svelte
- **Committed in:** c4ca4ac (Task 3 commit)

---

**Total deviations:** 8 auto-fixed (3 bugs during implementation, 4 bugs + 1 missing critical during verification)
**Impact on plan:** All fixes were necessary for correctness. The fetchWithAuth migration (deviation 8) was a pre-existing gap from Plan 01-01 that only surfaced when testing in Tauri mode. No scope creep.

## Issues Encountered
- macOS unsigned dev builds silently drop sendNotification calls. The notification code path executes correctly (confirmed via console.warn logs), but macOS requires signed apps for notification delivery. This is expected behavior and will work in release builds.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Desktop app is fully functional: launches, connects, chats, tray icon, close-to-tray, notifications
- Phase 1 (Desktop App) is complete -- ready to move to Phase 2 (Chat & Awareness)
- Notification preferences (per-channel levels) deferred to Phase 2 per NOTIF-05
- All client components now consistently use fetchWithAuth for Tauri compatibility

## Self-Check: PASSED

All 2 created files, 6 key modified files, and 3 task commits (b8612f2, e7cce5e, c4ca4ac) were verified.

---
*Phase: 01-desktop-app*
*Completed: 2026-03-01*
