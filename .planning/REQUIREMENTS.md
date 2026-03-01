# Requirements: Relay

**Defined:** 2026-02-28
**Core Value:** People can communicate with their friends in real-time across text, voice, and DMs without surrendering personal identity to a corporation.

## v1 Requirements

Requirements for the next milestone. Each maps to roadmap phases.

### Desktop App

- [x] **DESK-01**: Desktop app runs via Tauri v2 with system tray icon
- [x] **DESK-02**: App shows native OS notifications for mentions and activity
- [x] **DESK-03**: App persists in system tray when window is closed
- [x] **DESK-04**: App connects to a configurable server URL

### Mobile App

- [ ] **MOB-01**: Mobile app runs on iOS and Android
- [ ] **MOB-02**: User receives push notifications when app is backgrounded
- [ ] **MOB-03**: App handles intermittent connectivity gracefully (reconnect on wake)

### Voice

- [ ] **VOICE-01**: User can join a voice channel and speak with other users
- [ ] **VOICE-02**: Voice channels show who is currently connected
- [ ] **VOICE-03**: User can mute/deafen themselves in voice

### Notifications & Tracking

- [ ] **NOTIF-01**: Unread channels show a dot indicator in the sidebar
- [ ] **NOTIF-02**: Channels with unread mentions show a mention count badge
- [ ] **NOTIF-03**: User can @mention other users with autocomplete
- [ ] **NOTIF-04**: @everyone and @here mentions work in channels
- [ ] **NOTIF-05**: User can configure per-channel notification level (all/mentions/nothing)

### Encryption at Rest

- [ ] **EAR-01**: Message content is encrypted in SQLite using ChaCha20-Poly1305
- [ ] **EAR-02**: Encryption key is derived from server configuration (not stored in DB)

### Chat Enhancements

- [ ] **CHAT-01**: User can pin messages to a channel
- [ ] **CHAT-02**: Pasted URLs show inline link previews (title, description, image)
- [ ] **CHAT-03**: User can start a thread from any message
- [ ] **CHAT-04**: Thread replies appear in a side panel without leaving the channel
- [ ] **CHAT-05**: User can toggle compact/IRC display mode
- [ ] **CHAT-06**: Keyboard shortcuts for channel navigation and common actions

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Direct Messages

- **DM-01**: User can start a 1:1 direct message with another user
- **DM-02**: User can create a group DM with up to 10 participants
- **DM-03**: DM conversations appear in a separate sidebar section

### End-to-End Encryption

- **E2EE-01**: DM messages are encrypted client-side before transmission
- **E2EE-02**: User's keypair is generated on first login (X25519 + AES-256-GCM)
- **E2EE-03**: Key exchange happens automatically when starting a DM
- **E2EE-04**: User can back up encryption keys via recovery phrase

### Media & Polish

- **MEDIA-01**: Image gallery/lightbox for shared images
- **MEDIA-02**: Message export (JSON/CSV)
- **MEDIA-03**: Custom server emoji

### Moderation

- **MOD-01**: Role-based permissions
- **MOD-02**: Admin dashboard for user/invite management
- **MOD-03**: User can report content
- **MOD-04**: Admin can ban users

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Federation | Massive protocol complexity, friend-group scale doesn't need it |
| Bot/Plugin API | Enormous API surface, security implications, novelty at 10-25 users |
| Video/Screen Sharing | High complexity, defer until voice is proven solid |
| OAuth / SSO | Contradicts zero-knowledge auth model |
| Rich Text Editor | IRC philosophy is plain text; Markdown rendering in display is enough |
| Custom Emoji | Unicode emoji covers 99% of use cases at friend-group scale |
| Activity Status / Rich Presence | Requires OS-level hooks, privacy concerns |
| Server Discovery | Relay is for private groups; invite codes are the discovery mechanism |
| AI Features | External API dependencies, privacy concerns, not the product |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DESK-01 | Phase 1: Desktop App | Complete |
| DESK-02 | Phase 1: Desktop App | Complete |
| DESK-03 | Phase 1: Desktop App | Complete |
| DESK-04 | Phase 1: Desktop App | Complete |
| NOTIF-01 | Phase 2: Chat & Awareness | Pending |
| NOTIF-02 | Phase 2: Chat & Awareness | Pending |
| NOTIF-03 | Phase 2: Chat & Awareness | Pending |
| NOTIF-04 | Phase 2: Chat & Awareness | Pending |
| NOTIF-05 | Phase 2: Chat & Awareness | Pending |
| CHAT-01 | Phase 2: Chat & Awareness | Pending |
| CHAT-02 | Phase 2: Chat & Awareness | Pending |
| CHAT-03 | Phase 2: Chat & Awareness | Pending |
| CHAT-04 | Phase 2: Chat & Awareness | Pending |
| CHAT-05 | Phase 2: Chat & Awareness | Pending |
| CHAT-06 | Phase 2: Chat & Awareness | Pending |
| EAR-01 | Phase 3: Encryption at Rest | Pending |
| EAR-02 | Phase 3: Encryption at Rest | Pending |
| MOB-01 | Phase 4: Mobile App | Pending |
| MOB-02 | Phase 4: Mobile App | Pending |
| MOB-03 | Phase 4: Mobile App | Pending |
| VOICE-01 | Phase 5: Voice Channels | Pending |
| VOICE-02 | Phase 5: Voice Channels | Pending |
| VOICE-03 | Phase 5: Voice Channels | Pending |

**Coverage:**
- v1 requirements: 23 total
- Mapped to phases: 23
- Unmapped: 0

---
*Requirements defined: 2026-02-28*
*Last updated: 2026-02-28 after roadmap revision (Desktop App moved to Phase 1)*
