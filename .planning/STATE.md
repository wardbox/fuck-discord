# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-28)

**Core value:** People can communicate with their friends in real-time across text, voice, and DMs without surrendering personal identity to a corporation.
**Current focus:** Phase 1: Desktop App

## Current Position

Phase: 1 of 5 (Desktop App) -- COMPLETE
Plan: 2 of 2 in current phase
Status: Phase Complete
Last activity: 2026-03-01 -- Completed 01-02-PLAN.md (System tray + native notifications)

Progress: [██████████] 100%

## Performance Metrics

**Velocity:**
- Total plans completed: 2
- Average duration: 19 min
- Total execution time: 0.6 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-desktop-app | 2/2 | 38 min | 19 min |

**Recent Trend:**
- Last 5 plans: 01-01 (13 min), 01-02 (25 min)
- Trend: -

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Roadmap]: Tauri v2 for both desktop and mobile (replaces Capacitor v6 plan per research)
- [Roadmap]: LiveKit for voice (do not build custom SFU)
- [Roadmap]: DMs and E2EE deferred to v2 (not in current milestone)
- [Revision]: Desktop App moved to Phase 1 -- user wants native app running before chat polish. Desktop notifications (DESK-02) fire on any message in active channels; refined targeting comes in Phase 2 with unread/mention infrastructure.
- [01-01]: All client API calls route through fetchWithAuth() with configurable server URL
- [01-01]: Server auth supports Authorization: Bearer header (Tauri/mobile) with cookie fallback (browser)
- [01-01]: Tauri v2 __TAURI_INTERNALS__ for platform detection (not __TAURI__)
- [01-02]: Notification permission requested lazily on first unfocused message, not on app launch
- [01-02]: showMessageNotification is fire-and-forget (no await) to never block message processing
- [01-02]: app.exit(0) is the only way to quit; window close always hides to tray
- [01-02]: macOS unsigned dev builds silently drop notifications; works in release builds

### Pending Todos

None yet.

### Blockers/Concerns

- [Research]: Broadcast polling uses try_recv() with 100ms intervals -- CPU/battery concern. Address before mobile (Phase 4).
- [Research]: Design user_public_keys table with device_id from the start to avoid E2EE retrofit in v2.

## Session Continuity

Last session: 2026-03-01
Stopped at: Completed 01-02-PLAN.md -- Phase 1 (Desktop App) complete
Resume file: None
