# Feature Research

**Domain:** Self-hosted Discord-replacement group chat platform
**Researched:** 2026-02-28
**Confidence:** HIGH

## Current State (Already Built)

Before categorizing what's next, here is what Relay already has:

- Text channels with real-time WebSocket messaging
- Channel categories and ordering
- Message editing and deletion
- Reactions (emoji)
- Typing indicators
- User presence (online/offline status)
- File uploads (images, clips)
- Full-text search (FTS5 with prefix matching)
- Invite-based registration (no email/phone)
- Session-based auth with Argon2id
- User profiles (username, display name, avatar, status)
- Channel subscribe/unsubscribe in WS protocol

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = the friend group stays on Discord or fragments across platforms.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Direct Messages (1:1)** | Every chat platform has DMs. Users will not discuss private matters in channels. | MEDIUM | New channel type "dm", separate from server channels. Needs its own sidebar section and routing. |
| **Group DMs** | Discord, Slack, and Mattermost all support ad-hoc group conversations outside channels. Expected for subgroups within the friend group. | MEDIUM | Extension of DM system with multi-participant rooms. Cap at 10 participants (Discord's limit) is sensible. |
| **Desktop App (Tauri v2)** | A browser tab is not a chat app. Users expect a persistent, always-running application with system tray and native notifications. | MEDIUM | SvelteKit already builds to static; Tauri wraps it. Main complexity is system tray, auto-start, and notification integration. |
| **Mobile App (Capacitor v6)** | Users check messages from their phone. Without mobile, they will keep Discord installed "just for mobile" and never fully switch. | HIGH | Push notifications require FCM/APNs infrastructure. Capacitor wraps the web app but push notifications need a relay service since the server is self-hosted. |
| **Push Notifications (Mobile)** | Users expect to be pinged when mentioned or DMed, even when the app is backgrounded. Without push, mobile is useless. | HIGH | Requires FCM (Android) and APNs (iOS) integration. Self-hosted complication: needs a push relay or the server operator must configure their own Firebase project. |
| **Desktop Notifications** | OS-native toast notifications for mentions and DMs when the app is not focused. | LOW | Tauri has `tauri-plugin-notification` with cross-platform support. Web Notification API works for browser fallback. |
| **Unread Indicators** | Users need to see at a glance which channels have new messages and which have mentions. Without this, the app feels dead. | MEDIUM | Server-side: track last-read message ID per user per channel. Client-side: compare against latest message. Badge counts for mentions. |
| **Message Pinning** | Every major chat platform supports pinning important messages to a channel. Expected for sharing rules, links, info. | LOW | Simple DB flag on messages + API endpoint. UI: pin icon on messages, "Pinned Messages" panel in channel header. |
| **Link Previews / URL Embeds** | When a user pastes a URL, showing a title/description/image preview is standard behavior in Discord, Slack, and every modern chat. | MEDIUM | Server-side: fetch URL, parse Open Graph / oEmbed metadata. Cache results. Security: sanitize URLs, timeout fetches, block private IPs (SSRF prevention). |
| **@Mentions** | Mentioning specific users (and @everyone/@here) to get their attention. Fundamental to any multi-user chat. | MEDIUM | Parse `@username` in messages. Server-side: resolve mentions, trigger notifications. Client-side: autocomplete dropdown, highlight mentions in message display. |
| **Notification Settings (Per-Channel)** | Users need to mute noisy channels and ensure they get notified for important ones. Discord offers: All Messages, @Mentions Only, Nothing, with per-channel overrides. | MEDIUM | Server-side: user notification preferences per channel. Options: all, mentions, nothing. Interacts with unread tracking and push notification delivery. |
| **Voice Channels** | The friend group uses Discord for gaming voice chat. Without voice, Relay cannot fully replace Discord. | HIGH | Do NOT build an SFU. Use LiveKit (open-source, self-hostable, Go-based SFU with client SDKs for web/mobile/desktop). Requires separate LiveKit server process or embedded integration. |
| **Encryption at Rest** | Server admin (or anyone with DB access) should not be able to read message plaintext. Core to the privacy promise. | HIGH | ChaCha20-Poly1305 for message content in SQLite. Key management is the hard part: derive from server secret, rotate keys, handle re-encryption. |
| **Image/Media Gallery** | Inline image display, image lightbox, and ability to browse images shared in a channel. | LOW | Already have file uploads. Need: inline image rendering in messages, click-to-expand lightbox, optional "media" panel per channel. |

### Differentiators (Competitive Advantage)

Features that set Relay apart from Discord and other alternatives. These align with the privacy-first, IRC-reborn philosophy.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **E2EE for DMs** | No other self-hosted Discord alternative ships E2EE DMs out of the box. Matrix/Element has it but is notoriously complex to set up. Relay having E2EE DMs by default is a genuine differentiator. | HIGH | X25519 key exchange + AES-256-GCM via Web Crypto API + @noble/curves. Per-user keypair, session key per DM. Key backup via mnemonic recovery phrase. |
| **Zero-Knowledge Registration** | No email, no phone, no ID verification. Just an invite code and a username/passphrase. Discord now requires identity verification; this is the exact pain point driving the migration. | LOW | Already implemented. The differentiator is marketing it as a feature, not a limitation. |
| **Single Binary Deployment** | Download one file, run it, you have a chat server. No Docker, no Kubernetes, no external databases. No competitor offers this simplicity. | LOW | Already implemented via rust-embed. This is a deployment differentiator that matters for self-hosters. |
| **Compact/IRC Mode** | Dense, text-first message display. No avatar bubbles, no card layouts. Keyboard-driven navigation. Appeals to the IRC nostalgia crowd that Discord lost. | LOW | CSS-only toggle between "compact" and "cozy" layouts. Keyboard shortcuts for channel switching, message navigation. |
| **Keyboard-First Navigation** | Vim-style or IRC-style keyboard shortcuts. Navigate channels, scroll messages, send reactions without touching the mouse. | LOW | Client-side keybinding system. `Ctrl+K` command palette, `Alt+Up/Down` channel navigation, `/` commands. |
| **Self-Hosted Voice (LiveKit)** | Voice channels that run on YOUR server. Discord routes all voice through their servers. With LiveKit self-hosted, voice data never leaves your infrastructure. | HIGH | LiveKit is the only realistic option here. It is open-source, self-hostable, and has client SDKs for web, iOS, Android, and desktop. |
| **No Telemetry, No Tracking** | Zero analytics, zero tracking, zero data collection. Relay does not phone home. | LOW | This is a design decision, not a feature to build. Document it prominently. Enforce it by not including any analytics dependencies. |
| **Message Export** | Users can export their own message history. Server admins can export full channel history. Standard formats (JSON, CSV). | LOW | SQLite makes this trivial. API endpoint + CLI command. Differentiates from Discord where data export is cumbersome. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems at friend-group scale or conflict with the project's values.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Federation** | "Let different servers talk to each other like Matrix." | Massive protocol complexity, identity management across servers, spam vectors, moderation becomes exponentially harder. Matrix has spent years on this and it is still rough. Friend-group scale does not need federation. | Single server. If someone wants their own server, they run their own instance. No cross-server communication in v1. |
| **Bot/Plugin API** | "I want a music bot / moderation bot / dice roller." | API surface area is enormous. Security implications of running untrusted code. Maintenance burden of a stable API. At 10-25 users, bots are a novelty, not a need. | Defer to post-v1. Build the chat app first. If needed, start with webhook integrations (incoming/outgoing) which are simpler and safer. |
| **Video/Screen Sharing** | "Discord has screen share." | WebRTC video is significantly more complex than audio. Bandwidth requirements for self-hosted are high. LiveKit supports it, but the client-side UI complexity is substantial. | Defer until voice is solid and validated. LiveKit supports video, so the infrastructure path exists, but UI work is the bottleneck. |
| **Rich Text / Markdown Editor** | "Give me a WYSIWYG editor with formatting toolbar." | Adds complexity to message input, increases message storage overhead, creates rendering inconsistencies across clients. IRC philosophy is plain text. | Support basic Markdown rendering (bold, italic, code, links) in display. Keep the input as a plain text field. No toolbar. |
| **Custom Emoji / Sticker Packs** | "Let us upload custom server emojis like Discord." | Storage management, moderation concerns, rendering complexity across platforms, CDN requirements for self-hosted. | Support standard Unicode emoji first. Custom emoji is a v2+ feature. Reactions already work with Unicode. |
| **OAuth / SSO Integration** | "Let me sign in with Google/GitHub/LDAP." | Contradicts the zero-knowledge auth model. Adds external dependencies. Introduces complexity for self-hosters who would need to configure OAuth providers. | Invite codes + username/passphrase is the auth model. It is intentional, not a gap. |
| **Forum-Style Channels** | "Give me threaded, long-form discussion channels like Discord's Forum channels." | Significant UI complexity. Different message display paradigm. At friend-group scale, regular channels + threads cover this use case. | Basic threads (see Table Stakes) cover the need. If long-form discussion is needed, the group can use a separate tool. |
| **Activity Status / Rich Presence** | "Show what game I'm playing or what I'm listening to." | Requires OS-level hooks, game detection, Spotify integration. Privacy concerns with broadcasting activity. Significant client complexity. | Basic status (online/away/offline/custom text) is sufficient. Users can set a custom status message manually. |
| **Server Discovery / Public Directory** | "Let people find and join my server." | Relay is for private friend groups. Public discovery introduces spam, moderation scale problems, and conflicts with the invite-only model. | Invite codes are the discovery mechanism. If the server operator wants to share access, they generate invite codes. |
| **AI Features / Chatbots** | "Add AI summaries, smart replies, chatbot integration." | Requires external API dependencies (OpenAI, etc.), adds latency, costs money, privacy concerns with sending messages to third parties. | Out of scope. The platform is for human-to-human communication. |

## Feature Dependencies

```
Direct Messages (DMs)
    |-- requires --> New "dm" channel type in DB schema
    |-- requires --> DM-specific routing in client
    |-- enables --> Group DMs
    |-- enables --> E2EE for DMs (X25519 + AES-256-GCM)

E2EE for DMs
    |-- requires --> DMs working
    |-- requires --> Client-side key generation (Web Crypto API)
    |-- requires --> Key exchange protocol (X25519)
    |-- requires --> Key backup/recovery (mnemonic phrase)

Desktop App (Tauri v2)
    |-- requires --> Working web client (already exists)
    |-- enables --> Desktop Notifications (tauri-plugin-notification)
    |-- enables --> System Tray with unread badges
    |-- enables --> Auto-start on login
    |-- enables --> Voice channels (WebRTC works in Tauri webview)

Mobile App (Capacitor v6)
    |-- requires --> Working web client (already exists)
    |-- requires --> Push notification infrastructure (FCM/APNs)
    |-- enables --> Mobile push notifications
    |-- enables --> Voice channels on mobile

Unread Tracking
    |-- requires --> Server-side last-read tracking per user per channel
    |-- enables --> Unread indicators in sidebar
    |-- enables --> Badge counts (mentions vs unread)
    |-- enables --> Notification settings (per-channel)
    |-- enhances --> Desktop notifications (what to notify about)
    |-- enhances --> Push notifications (what to push)

@Mentions
    |-- requires --> Username autocomplete in message input
    |-- requires --> Mention parsing in message content
    |-- enhances --> Unread tracking (mention counts)
    |-- enhances --> Notifications (mention triggers notification)

Voice Channels
    |-- requires --> LiveKit server deployment
    |-- requires --> LiveKit client SDK integration (web)
    |-- requires --> Desktop app OR browser support for WebRTC
    |-- enhances --> Mobile app (voice on mobile)

Notification Settings
    |-- requires --> Unread tracking
    |-- requires --> @Mentions
    |-- enhances --> Push notifications (filter what gets pushed)
    |-- enhances --> Desktop notifications (filter what gets shown)

Link Previews
    |-- requires --> Server-side URL fetching
    |-- requires --> OG/oEmbed metadata parsing
    |-- independent of other features

Message Pinning
    |-- requires --> DB flag on messages
    |-- independent of other features

Encryption at Rest
    |-- independent of other features (server-side only)
    |-- does NOT conflict with E2EE (different layers)

Threads
    |-- requires --> Thread data model (parent_message_id on messages)
    |-- enhances --> Channels (side panel thread view)
    |-- enhances --> Unread tracking (thread-level unreads)
```

### Dependency Notes

- **DMs before E2EE:** E2EE only makes sense for DMs (channel messages are shared; encrypting them E2E for all members is Matrix-level complexity). Build DMs first, then layer E2EE on top.
- **Unread tracking before notifications:** You cannot build meaningful notification settings without knowing what's unread. Unread tracking is the foundation.
- **Desktop app before mobile:** Tauri is simpler than Capacitor (no push notification infrastructure needed). Desktop validates the native app experience before tackling mobile's harder problems.
- **@Mentions before notification settings:** Mention detection is what makes per-channel notification settings meaningful. "Notify on mentions only" requires mentions to exist.
- **Voice channels are independent but HIGH effort:** LiveKit integration does not depend on DMs or threads, but it is the single highest-complexity feature and should not block other work.

## MVP Definition

### Launch With (Next Milestone)

The features that will make the friend group actually switch from Discord.

- [ ] **Direct Messages (1:1 and group)** -- Private conversation is non-negotiable. This is the #1 missing feature.
- [ ] **Desktop App (Tauri v2)** -- System tray, native notifications, always-on. This makes Relay a real app, not a browser tab.
- [ ] **Unread Tracking + Indicators** -- Users need to know what they missed. Sidebar dots and mention badges.
- [ ] **@Mentions** -- `@username`, `@everyone`, `@here` with autocomplete. Triggers notifications.
- [ ] **Desktop Notifications** -- OS-native toast when mentioned or DMed while the app is not focused.
- [ ] **Message Pinning** -- Quick win, low complexity, high utility.

### Add After Validation (v1.x)

Features to add once core DMs + desktop app are working and the group is using Relay daily.

- [ ] **Mobile App (Capacitor v6)** -- Add when the group says "I want this on my phone." Push notifications via FCM/APNs.
- [ ] **Voice Channels (LiveKit)** -- Add when the group says "we still use Discord for voice." Requires LiveKit server.
- [ ] **Notification Settings (Per-Channel)** -- Add when users complain about too many or too few notifications.
- [ ] **Link Previews** -- Add when users paste URLs and complain they cannot see what they are.
- [ ] **Encryption at Rest** -- Add when the server is deployed and the admin wants DB-level encryption.
- [ ] **E2EE for DMs** -- Add after DMs are stable and key management UX is designed.
- [ ] **Threads** -- Add when channel conversations get tangled. Not critical at 10-25 users.

### Future Consideration (v2+)

Features to defer until the platform is solid and the group is fully switched over.

- [ ] **Video / Screen Sharing** -- LiveKit supports it, but UI complexity is high. Defer until voice is proven.
- [ ] **Custom Emoji** -- Nice to have, not critical. Unicode emoji covers 99% of use cases.
- [ ] **Webhook Integrations** -- Incoming/outgoing webhooks for basic automation. Simpler than a full bot API.
- [ ] **Admin Dashboard** -- Server management UI (user management, invite codes, server settings). Currently admin tasks can be done via CLI.
- [ ] **Message Export** -- JSON/CSV export for data portability.
- [ ] **Multi-server Support** -- Client-side ability to connect to multiple Relay servers (like Discord's server list). Only relevant if multiple friend groups run separate servers.

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Direct Messages (1:1) | HIGH | MEDIUM | P1 |
| Group DMs | HIGH | MEDIUM | P1 |
| Desktop App (Tauri v2) | HIGH | MEDIUM | P1 |
| Unread Tracking | HIGH | MEDIUM | P1 |
| @Mentions | HIGH | MEDIUM | P1 |
| Desktop Notifications | HIGH | LOW | P1 |
| Message Pinning | MEDIUM | LOW | P1 |
| Mobile App (Capacitor) | HIGH | HIGH | P2 |
| Voice Channels (LiveKit) | HIGH | HIGH | P2 |
| Push Notifications (Mobile) | HIGH | HIGH | P2 |
| Link Previews | MEDIUM | MEDIUM | P2 |
| Notification Settings | MEDIUM | MEDIUM | P2 |
| Encryption at Rest | MEDIUM | HIGH | P2 |
| E2EE for DMs | MEDIUM | HIGH | P2 |
| Threads | MEDIUM | MEDIUM | P2 |
| Image Gallery/Lightbox | LOW | LOW | P2 |
| Compact/IRC Mode | LOW | LOW | P2 |
| Keyboard Shortcuts | LOW | LOW | P2 |
| Video/Screen Sharing | MEDIUM | HIGH | P3 |
| Custom Emoji | LOW | MEDIUM | P3 |
| Webhook Integrations | LOW | MEDIUM | P3 |
| Admin Dashboard | LOW | MEDIUM | P3 |
| Message Export | LOW | LOW | P3 |

**Priority key:**
- P1: Must have for next milestone (gets the group to switch)
- P2: Should have, add iteratively (makes the group stay)
- P3: Nice to have, future consideration (makes the platform mature)

## Competitor Feature Analysis

| Feature | Discord | Slack | Matrix/Element | Revolt/Stoat | Relay (Current) | Relay (Target) |
|---------|---------|-------|----------------|--------------|-----------------|----------------|
| Text Channels | Yes | Yes | Yes (Rooms) | Yes | **Yes** | Yes |
| DMs | Yes | Yes | Yes | Yes | No | **P1** |
| Group DMs | Yes (10 max) | Yes | Yes | Yes | No | **P1** |
| Threads | Yes | Yes | Partial | No | No | **P2** |
| Voice | Yes | Yes (Huddles) | Yes (Element Call) | Partial | No | **P2** |
| Video/Screen Share | Yes | Yes | Yes | No | No | **P3** |
| Reactions | Yes | Yes | Yes | Yes | **Yes** | Yes |
| File Upload | Yes (25MB free) | Yes | Yes | Yes | **Yes** | Yes |
| Search | Yes | Yes (limited free) | Partial | Yes | **Yes (FTS5)** | Yes |
| Unread/Badges | Yes | Yes | Yes | Yes | No | **P1** |
| @Mentions | Yes | Yes | Yes | Yes | No | **P1** |
| Pin Messages | Yes (50/channel) | Yes | No | Yes | No | **P1** |
| Link Previews | Yes | Yes (unfurling) | Yes | Yes | No | **P2** |
| Desktop App | Yes (Electron) | Yes (Electron) | Yes (Electron) | Yes (Tauri) | No | **P1** |
| Mobile App | Yes | Yes | Yes | Yes (partial) | No | **P2** |
| Push Notifications | Yes | Yes | Yes | Partial | No | **P2** |
| E2EE | DMs only (DAVE) | No | Yes (Megolm) | Planned | No | **P2** |
| Encryption at Rest | Unknown | Enterprise | Partial | No | No | **P2** |
| Self-Hosted | No | No | Yes | Yes | **Yes** | Yes |
| No Email/Phone | No (requires ID now) | No | Optional | Yes | **Yes** | Yes |
| Single Binary | No | No | No | No (Docker) | **Yes** | Yes |
| Federation | No | No | Yes | No | No | No (anti-feature) |
| Bots/Plugins | Yes | Yes | Yes | Yes | No | No (v2+) |
| Custom Emoji | Yes | Yes | No | Yes | No | No (v2+) |
| Roles/Permissions | Yes (complex) | Yes | Yes | Yes | No | **P2** |

## Sources

- [Zap-Hosting: Best Self-Hosted Discord Alternatives 2026](https://zap-hosting.com/en/blog/2026/02/the-best-self-hosted-discord-alternatives-2026-ranking-pros-cons/)
- [Taggart Tech: Discord Alternatives, Ranked](https://taggart-tech.com/discord-alternatives/)
- [How-To Geek: 5 Self-Hosted Discord Alternatives](https://www.howtogeek.com/5-self-hosted-discord-alternatives-that-are-actually-great/)
- [Discord Support: Threads FAQ](https://support.discord.com/hc/en-us/articles/4403205878423-Threads-FAQ)
- [Discord Support: Channel Notification Settings](https://support.discord.com/hc/en-us/articles/209791877-How-do-I-mute-and-disable-notifications-for-specific-channels)
- [Discord Blog: Voice Architecture](https://discord.com/blog/how-discord-handles-two-and-half-million-concurrent-voice-users-using-webrtc)
- [Tauri v2: Notification Plugin](https://v2.tauri.app/plugin/notification/)
- [Capacitor: Push Notifications Plugin](https://capacitorjs.com/docs/apis/push-notifications)
- [LiveKit GitHub](https://github.com/livekit/livekit)
- [Rocket.Chat: E2EE Specifications](https://docs.rocket.chat/docs/end-to-end-encryption-specifications)
- [Mattermost: Threaded Discussions](https://docs.mattermost.com/end-user-guide/collaborate/organize-conversations.html)
- [Slack: Thread Notifications](https://slack.com/help/articles/115000769927-Use-threads-to-organize-discussions)
- [PC Gamer: Discord Alternatives Tested](https://www.pcgamer.com/hardware/ive-tested-three-free-discord-alternatives-in-a-desperate-attempt-not-to-offer-up-my-personal-data-just-to-talk-to-my-favorite-weirdos/)
- [ThePCEnthusiast: Privacy-Friendly Discord Alternatives](https://thepcenthusiast.com/best-discord-alternatives-privacy-friendly-minimal-verification/)

---
*Feature research for: self-hosted Discord-replacement group chat*
*Researched: 2026-02-28*
