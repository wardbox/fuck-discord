# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-28)

**Core value:** People can communicate with their friends in real-time across text, voice, and DMs without surrendering personal identity to a corporation.
**Current focus:** Phase 1: Desktop App

## Current Position

Phase: 1 of 5 (Desktop App)
Plan: 0 of 2 in current phase
Status: Ready to plan
Last activity: 2026-02-28 -- Roadmap revised (Desktop App moved to Phase 1)

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**
- Total plans completed: 0
- Average duration: -
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**
- Last 5 plans: -
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

### Pending Todos

None yet.

### Blockers/Concerns

- [Research]: Broadcast polling uses try_recv() with 100ms intervals -- CPU/battery concern. Address before mobile (Phase 4).
- [Research]: Design user_public_keys table with device_id from the start to avoid E2EE retrofit in v2.

## Session Continuity

Last session: 2026-02-28
Stopped at: Roadmap revised, ready to plan Phase 1 (Desktop App)
Resume file: None
