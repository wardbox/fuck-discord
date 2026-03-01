# Roadmap: Relay

## Overview

Relay has a working chat app -- Rust/Axum backend, SvelteKit frontend, WebSocket real-time messaging, FTS5 search, reactions, file uploads. The gap is making it feel like a real product instead of a browser-tab experiment. This milestone delivers: a desktop app so Relay lives in the system tray first, then unread tracking and mentions so you know what you missed, threads and pinning so conversations don't drown, encryption at rest so the server admin can't read messages, a mobile app so you're reachable away from the desk, and voice channels so the friend group can stop opening Discord for gaming sessions. When all five phases ship, there is no reason to keep Discord installed.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Desktop App** - Tauri v2 wrapper with system tray, native notifications, and configurable server URL
- [ ] **Phase 2: Chat & Awareness** - Unread tracking, @mentions, threads, pinning, link previews, compact mode, and keyboard shortcuts
- [ ] **Phase 3: Encryption at Rest** - ChaCha20-Poly1305 encryption of message content in SQLite
- [ ] **Phase 4: Mobile App** - iOS and Android via Tauri v2 mobile with push notifications and reconnection handling
- [ ] **Phase 5: Voice Channels** - LiveKit-powered voice with join/leave, presence, and mute/deafen controls

## Phase Details

### Phase 1: Desktop App
**Goal**: Relay lives as a native desktop application that stays connected in the system tray and delivers OS notifications
**Depends on**: Nothing (first phase -- wraps existing working web app)
**Requirements**: DESK-01, DESK-02, DESK-03, DESK-04
**Success Criteria** (what must be TRUE):
  1. User can install and launch a native desktop app that connects to their Relay server via a configurable URL
  2. User receives native OS notifications (toast/banner) when messages arrive in channels they are active in
  3. User can close the window and the app persists in the system tray with an icon, staying connected to the server
**Plans**: 2 plans

Plans:
- [x] 01-01-PLAN.md -- Tauri shell + configurable server URL (DESK-01, DESK-04)
- [x] 01-02-PLAN.md -- System tray persistence + native notifications (DESK-02, DESK-03)

### Phase 2: Chat & Awareness
**Goal**: Users have full situational awareness of channel activity and can organize conversations without missing anything
**Depends on**: Nothing (builds on existing working chat, independent of desktop wrapper)
**Requirements**: NOTIF-01, NOTIF-02, NOTIF-03, NOTIF-04, NOTIF-05, CHAT-01, CHAT-02, CHAT-03, CHAT-04, CHAT-05, CHAT-06
**Success Criteria** (what must be TRUE):
  1. User can glance at the sidebar and immediately see which channels have unread messages (dot) and which have unread mentions (count badge)
  2. User can type @ and see an autocomplete list of users, and @everyone/@here mentions notify the appropriate people
  3. User can start a thread from any message and read/reply in a side panel without leaving the channel
  4. User can pin a message to a channel and view all pinned messages
  5. User can toggle compact/IRC display mode, and common actions (channel switching, message navigation) work via keyboard shortcuts
**Plans**: TBD

Plans:
- [ ] 02-01: TBD
- [ ] 02-02: TBD
- [ ] 02-03: TBD

### Phase 3: Encryption at Rest
**Goal**: Message content stored in SQLite is encrypted so database access alone cannot reveal conversation content
**Depends on**: Phase 2 (encryption key input mechanism and schema design should be settled after core features stabilize)
**Requirements**: EAR-01, EAR-02
**Success Criteria** (what must be TRUE):
  1. Message content in the SQLite database file is encrypted with ChaCha20-Poly1305 and unreadable without the encryption key
  2. Encryption key is provided at server startup via environment variable or separate secrets file, not stored alongside the database
**Plans**: TBD

Plans:
- [ ] 03-01: TBD

### Phase 4: Mobile App
**Goal**: Users can chat on their phone with background push notifications and graceful handling of mobile network conditions
**Depends on**: Phase 1 (Tauri native app approach validated on desktop first), Phase 3 (encryption settled before mobile complicates key management)
**Requirements**: MOB-01, MOB-02, MOB-03
**Success Criteria** (what must be TRUE):
  1. User can install the app on iOS or Android and use all core chat features (channels, messages, threads, mentions)
  2. User receives push notifications when the app is backgrounded or closed
  3. App reconnects automatically when the device wakes from sleep, switches networks, or regains connectivity
**Plans**: TBD

Plans:
- [ ] 04-01: TBD
- [ ] 04-02: TBD

### Phase 5: Voice Channels
**Goal**: Users can talk to each other in real-time voice channels hosted entirely on the self-hosted server
**Depends on**: Phase 2 (channel infrastructure exists), Phase 1 (desktop app provides primary voice client)
**Requirements**: VOICE-01, VOICE-02, VOICE-03
**Success Criteria** (what must be TRUE):
  1. User can join a voice channel and speak with other connected users with acceptable latency
  2. Users in a voice channel are visible to everyone in the server (voice presence indicator)
  3. User can mute and deafen themselves while in a voice channel
**Plans**: TBD

Plans:
- [ ] 05-01: TBD
- [ ] 05-02: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Desktop App | 2/2 | Complete | 2026-03-01 |
| 2. Chat & Awareness | 0/3 | Not started | - |
| 3. Encryption at Rest | 0/1 | Not started | - |
| 4. Mobile App | 0/2 | Not started | - |
| 5. Voice Channels | 0/2 | Not started | - |
