# Phase 1 Research: Desktop App

**Phase:** 1 of 5 (Desktop App)
**Goal:** Relay lives as a native desktop application that stays connected in the system tray and delivers OS notifications
**Requirements:** DESK-01, DESK-02, DESK-03, DESK-04
**Researched:** 2026-02-28

---

## What This Phase Must Deliver

| Req ID | Requirement | Success Criteria |
|--------|-------------|------------------|
| DESK-01 | Desktop app runs via Tauri v2 with system tray icon | User can install and launch a native desktop app |
| DESK-02 | App shows native OS notifications for messages | User receives native OS notifications (toast/banner) when messages arrive in channels they are active in |
| DESK-03 | App persists in system tray when window is closed | User can close the window and the app stays connected in the system tray |
| DESK-04 | App connects to a configurable server URL | User enters their Relay server URL and the app connects to it |

---

## Current State Analysis

### What Already Works (No Changes Needed)

1. **SvelteKit SPA is Tauri-ready.** The client already uses `@sveltejs/adapter-static` with `fallback: 'index.html'` and `ssr = false` in the root layout. This is exactly what Tauri requires. Zero changes to the SvelteKit build pipeline.

2. **WebSocket authentication uses session tokens, not cookies.** The WS handler (`server/src/ws/handler.rs`) authenticates via a token sent as the first message, not via cookie headers. The client stores the session token in `localStorage` and passes it via `{ type: 'authenticate', token }`. This works identically in a Tauri webview as in a browser.

3. **Client stores session in localStorage.** `auth.svelte.ts` reads/writes `relay_session` from `localStorage`, which persists in Tauri's webview storage between app restarts.

### What Must Change

#### 1. WebSocket URL (connection.svelte.ts) -- DESK-04

**Current code (lines 23-24):**
```typescript
const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const url = `${protocol}//${window.location.host}/ws`;
```

This hardcodes the WebSocket URL to same-origin. In Tauri, `window.location.host` resolves to `tauri.localhost` or `localhost` (the local webview origin), not the remote Relay server. The URL must become configurable.

**Required change:** Create a platform-aware configuration module that:
- In browser: uses `window.location.host` (current behavior, unchanged)
- In Tauri: reads a user-configured server URL from the Tauri store plugin

**Detection:** `window.__TAURI__` or `window.__TAURI_INTERNALS__` is defined when running inside Tauri.

#### 2. HTTP API Base URL (auth.svelte.ts) -- DESK-04

**Current code:** All `fetch()` calls use relative paths like `/api/auth/login`. In browser, these resolve to the same origin. In Tauri, they would hit `tauri.localhost` which serves the embedded SPA, not the Relay server.

**Required change:** All API `fetch()` calls must use an absolute URL derived from the configured server URL. This affects:
- `auth.svelte.ts`: `/api/auth/register`, `/api/auth/login`, `/api/auth/logout`, `/api/auth/me`
- `messages.svelte.ts`: `/api/channels/{id}/messages`, `/api/search`
- `uploads.svelte.ts`: `/api/upload`, `/uploads/{filename}`
- Any other store that calls `fetch()`

**Approach:** Create a `getApiBase()` function that returns `""` in browser (relative URLs work) or `"https://relay.example.com"` in Tauri (absolute URLs to the configured server).

#### 3. Server CORS Configuration (handlers/mod.rs) -- DESK-04

**Current code (lines 61-67):**
```rust
.layer(if cfg!(debug_assertions) {
    CorsLayer::permissive()
} else {
    CorsLayer::new()
}
```

In release mode, CORS is restrictive (empty allowlist), which blocks all cross-origin requests. Tauri's webview sends requests from `tauri://localhost` (macOS) or `https://tauri.localhost` (Windows/Linux) origin. These will be rejected.

**Required change:** The server must accept requests from Tauri's origin(s). Two options:
- **Option A (simple):** Make CORS origins configurable via server config/env var. The server operator adds their Tauri origin to the allowlist.
- **Option B (permissive):** In release mode, allow `tauri://localhost` and `https://tauri.localhost` by default alongside same-origin requests.

**Note on cookies:** The session cookie is set with `SameSite=Lax`. Cross-origin requests from Tauri will not send this cookie. However, this is fine because the client already sends auth via the JSON body (login returns `session_id` in the response, which the client stores in localStorage and sends via WS authenticate message). The `relay_session` cookie is used by the `require_auth` middleware for REST API calls. For Tauri, these REST calls will need to include the session token as an `Authorization` header instead of relying on cookies, OR the cookie `SameSite` policy needs adjustment for Tauri origins.

**Recommendation:** Add `Authorization: Bearer {session_id}` support to the `require_auth` middleware as a fallback when no cookie is present. This is cleaner than fighting cookie policies across origins and also prepares for mobile (Phase 4).

#### 4. Notification Permission UX -- DESK-02

The app must not request notification permission immediately on launch. Per the pitfalls research, users reflexively deny permission if asked too early. The app should wait until the user has logged in and is actively using the app before requesting permission -- ideally after they receive their first message or explicitly enable notifications in settings.

---

## Technology Stack for This Phase

### Core

| Technology | Version | Purpose | Confidence |
|-----------|---------|---------|------------|
| tauri | 2.10.x | Desktop app shell (macOS, Linux, Windows) | HIGH -- v2.10.2 verified via docs.rs, published 2026-02-04 |
| @tauri-apps/cli | 2.x | Build and dev tooling | HIGH |
| @tauri-apps/api | 2.x | Frontend JS bridge to Tauri APIs | HIGH |

### Plugins

| Plugin | Purpose | Req |
|--------|---------|-----|
| `tauri-plugin-notification` (2.3.x) | Native OS notifications (toast/banner) | DESK-02 |
| `tauri-plugin-store` (2.x) | Persistent settings (server URL, notification prefs) | DESK-04 |
| `tray-icon` feature (built into Tauri core) | System tray icon with menu | DESK-01, DESK-03 |

### Not Needed Yet

| Technology | Why Deferred |
|-----------|--------------|
| `tauri-plugin-updater` | Auto-update is a "looks done but isn't" item. Ship the app first, add auto-update as a follow-up. Not in DESK-01 through DESK-04. |
| `tauri-plugin-deep-link` | Deep linking (`relay://open/channel/general`) is useful but not required by any DESK requirement. |
| `tauri-plugin-websocket` | The browser's native `WebSocket` API works in Tauri's webview for connecting to remote servers. The Tauri plugin is only needed if browser WS is blocked by CORS/security, which is unlikely for WSS connections. Start with native `WebSocket`; add the plugin only if problems arise. |

---

## Architecture: How Tauri Wraps Relay

```
+-------------------------------------------------------+
|  Tauri Desktop App                                     |
|  +-------------------------------------------------+   |
|  |  System Tray (Rust)                             |   |
|  |  - Tray icon (relay logo)                       |   |
|  |  - Menu: Show/Hide Window, Quit                 |   |
|  |  - Click: toggle window visibility              |   |
|  +-------------------------------------------------+   |
|  |  WebView Window                                 |   |
|  |  +-----------------------------------------+    |   |
|  |  |  SvelteKit SPA (same build as server)   |    |   |
|  |  |  - Reads server URL from store plugin   |    |   |
|  |  |  - Connects via WSS to remote server    |    |   |
|  |  |  - Calls Tauri notification API for     |    |   |
|  |  |    desktop notifications                |    |   |
|  |  +-----------------------------------------+    |   |
|  +-------------------------------------------------+   |
+-------------------------------------------------------+
         |                                    |
         | WSS/HTTPS                          | IPC (Tauri commands)
         v                                    v
+-------------------+                  +-------------------+
| Remote Relay      |                  | Local Filesystem  |
| Server            |                  | - store.json      |
| (existing binary) |                  |   (server URL,    |
+-------------------+                  |    preferences)   |
                                       +-------------------+
```

**Key architectural property:** The Tauri app does NOT embed or run the Relay server. It is a pure client that connects to a remote Relay server, identical to the browser but with native OS integration. The existing Relay server binary is completely unchanged in its deployment model.

---

## Detailed Implementation Analysis

### DESK-01: Desktop App Runs via Tauri v2 with System Tray Icon

**Scope:** Initialize Tauri project within the existing monorepo, configure it to load the SvelteKit SPA, and display a tray icon.

**Directory structure:**
```
fuck-discord/
  client/           # Existing SvelteKit
  server/           # Existing Rust server
  src-tauri/         # NEW: Tauri desktop app
    Cargo.toml
    tauri.conf.json
    src/
      lib.rs         # Tauri setup (tray, plugins)
      main.rs        # Entry point
    icons/           # App icons (auto-generated by tauri)
    capabilities/    # Permission config
```

**tauri.conf.json key settings:**
```json
{
  "build": {
    "beforeDevCommand": "cd client && npm run dev",
    "beforeBuildCommand": "cd client && npm run build",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../client/build"
  },
  "app": {
    "windows": [
      {
        "title": "Relay",
        "width": 1200,
        "height": 800,
        "minWidth": 400,
        "minHeight": 300
      }
    ]
  }
}
```

**Tray icon implementation (Rust):**
```rust
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Show Relay", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up, ..
            } = event {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
```

**Cargo workspace integration:** The `src-tauri` crate should NOT be in the root Cargo workspace (it has its own dependency tree managed by the Tauri CLI). The root `Cargo.toml` workspace members list should remain `["server"]` only.

**Risks:**
- Tauri dev mode proxying: In dev, the SvelteKit dev server runs on :5173 and Tauri loads from `devUrl`. The Vite proxy configuration (`/api` -> `:3000`, `/ws` -> `:3000`) still works because the Tauri webview loads from the Vite dev server URL. No changes needed for dev mode.
- Icon generation: Tauri requires app icons in multiple formats. Use `tauri icon` CLI command with a source PNG (1024x1024).

### DESK-02: Native OS Notifications for Messages

**Scope:** When a message arrives in a channel the user has open (is subscribed to), show a native OS notification if the window is not focused.

**Implementation approach:**

1. **Listen for `message_create` events in `connection.svelte.ts`.** This already happens (line 159-161).

2. **Check if the window is focused.** Use `document.hasFocus()` or Tauri's window focus API. Only show notifications when the window is NOT focused (the user is doing something else).

3. **Call Tauri notification API from the SvelteKit client:**
```typescript
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';

async function showNotification(title: string, body: string) {
    if (!window.__TAURI__) return; // No-op in browser
    let granted = await isPermissionGranted();
    if (!granted) {
        const permission = await requestPermission();
        granted = permission === 'granted';
    }
    if (granted) {
        sendNotification({ title, body });
    }
}
```

4. **When to notify:** On `message_create`, if:
   - The window is not focused (`!document.hasFocus()`)
   - The message is not from the current user (`msg.author_id !== auth.user?.id`)
   - The user has not muted notifications (stored in Tauri store, but this is a Phase 2 feature -- for now, notify on all messages)

**Notification content:**
- Title: `#{channel_name}` (or `@username` for DMs in future)
- Body: `username: message content` (truncated to ~100 chars)

**Platform behavior:**
- macOS: Notification Center toast with app icon
- Windows: Toast notification (requires installed app; shows PowerShell info during dev)
- Linux: Desktop notification via `notify-rust`

**Permission timing:** Do not request notification permission on launch. Request it:
- When the user receives their first message while the window is unfocused, OR
- When the user explicitly enables notifications in a settings panel

**Risks:**
- Windows dev mode: Notifications during development show PowerShell name/icon, not the app's. This is a known Tauri limitation; only affects dev, not production builds.
- Notification spam: Without per-channel mute settings (Phase 2), active channels will generate many notifications. Mitigation: rate-limit notifications (max 1 per channel per N seconds) and only notify when window is unfocused.

### DESK-03: App Persists in System Tray When Window Is Closed

**Scope:** When the user clicks the window close button (X), hide the window instead of quitting the app. The app remains connected via WebSocket in the background. The tray icon stays visible with menu options to show the window or quit.

**Implementation:**

In Tauri v2, intercept the window close event to prevent the app from exiting:

```rust
// In lib.rs setup
app.get_webview_window("main")
    .unwrap()
    .on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            // Prevent the window from closing; hide it instead
            api.prevent_close();
            let _ = window.hide();
        }
    });
```

Also prevent the app from exiting when all windows are closed:

```rust
tauri::Builder::default()
    .on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
        }
    })
    // ...
```

**WebSocket persistence:** The WebSocket connection lives in the SvelteKit SPA's JavaScript runtime inside the webview. When the window is hidden (not destroyed), the webview's JS engine continues running. The WebSocket stays connected. Messages continue to arrive, and the `message_create` handler can trigger notifications even while the window is hidden.

**Critical detail:** Hiding the window does NOT stop the webview's JS. The `ConnectionStore` in `connection.svelte.ts` keeps running. The reconnection logic keeps working. This is the key behavior that makes "persist in system tray" work without any changes to the WebSocket layer.

**Tray menu updates:**
- When window is visible: "Hide" menu item
- When window is hidden: "Show" menu item
- Always: "Quit" menu item

**Quitting:** The "Quit" menu item calls `app.exit(0)`, which destroys the window, closes the WebSocket (triggering the server's disconnect handler), and terminates the process.

**Risks:**
- Memory: The hidden webview still consumes RAM. For Relay's SPA this is acceptable (typical Tauri app uses ~50-100MB).
- Sleep/lid close: When the machine sleeps, the WebSocket will disconnect. On wake, the existing reconnection logic in `connection.svelte.ts` (exponential backoff up to 30s) will reconnect automatically. No changes needed.
- macOS dock icon: By default, hiding the window still shows the app in the dock. To make it tray-only, the app can use `app.set_activation_policy(tauri::ActivationPolicy::Accessory)` on macOS, but this is optional polish.

### DESK-04: Configurable Server URL

**Scope:** The user can enter the URL of their Relay server (e.g., `https://relay.myserver.com`) and the app connects to it.

**Implementation:**

1. **First-launch flow:** When the app starts for the first time (no server URL stored), show a "Connect to Server" screen before the login screen. The user enters their server URL. This is stored via the Tauri store plugin.

2. **Tauri store plugin for settings:**
```typescript
import { load } from '@tauri-apps/plugin-store';

const store = await load('settings.json', { autoSave: true });

// Save server URL
await store.set('serverUrl', 'https://relay.myserver.com');

// Read server URL
const serverUrl = await store.get<string>('serverUrl');
```

3. **Platform-aware config module (new file: `client/src/lib/config.ts`):**
```typescript
export async function getServerUrl(): Promise<string> {
    if (window.__TAURI__) {
        const { load } = await import('@tauri-apps/plugin-store');
        const store = await load('settings.json');
        const url = await store.get<string>('serverUrl');
        return url || '';  // Empty means "not configured yet"
    }
    // Browser: same-origin
    return window.location.origin;
}

export function getWsUrl(serverUrl: string): string {
    const url = new URL(serverUrl);
    const protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${protocol}//${url.host}/ws`;
}
```

4. **Server URL validation:** Before storing, validate the URL by hitting `GET /api/auth/me` (expects 401 if the server is reachable). Show an error if the server is unreachable.

5. **Changing server URL:** A settings page (or settings button on the login screen) allows the user to change the server URL. Changing it clears the stored session (localStorage) and reconnects.

**Server-side impact:**

The Relay server needs CORS changes to accept requests from the Tauri webview origin. The `require_auth` middleware also needs to support `Authorization: Bearer` header as a cookie alternative.

**CORS change (server/src/handlers/mod.rs):**
```rust
use tower_http::cors::{Any, CorsLayer};
use axum::http::{header, Method};

let cors = if cfg!(debug_assertions) {
    CorsLayer::permissive()
} else {
    CorsLayer::new()
        .allow_origin([
            "tauri://localhost".parse().unwrap(),
            "https://tauri.localhost".parse().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
        .allow_credentials(true)
};
```

**Auth middleware change (server/src/auth/middleware.rs):**
```rust
pub fn extract_session_id(request: &Request) -> Option<String> {
    // Try Authorization header first (for Tauri/mobile clients)
    if let Some(auth_header) = request.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(value) = auth_header.to_str() {
            if let Some(token) = value.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }
    // Fall back to cookie (browser clients)
    let cookie_header = request.headers().get(COOKIE)?.to_str().ok()?;
    cookie_header.split(';').find_map(|cookie| {
        let cookie = cookie.trim();
        cookie.strip_prefix("relay_session=").map(|v| v.to_string())
    })
}
```

**Client-side fetch wrapper:** All `fetch()` calls in the SPA must:
- Use the configured server URL as the base (not relative paths)
- Include `Authorization: Bearer {session_id}` header when authenticated
- Handle CORS preflight (the browser/webview handles this automatically with the proper CORS headers on the server)

**Risks:**
- Mixed content: If the server runs on HTTP and the Tauri webview uses HTTPS (`https://tauri.localhost`), the browser may block "mixed content" requests. The server should run HTTPS in production. For development/local use, `http://localhost:3000` works because `localhost` is treated as secure context.
- Certificate validation: Self-signed certs on the Relay server will cause connection failures in the Tauri webview. Self-hosters need valid TLS (Let's Encrypt) or must use HTTP for local network deployments.

---

## Codebase Changes Summary

### New Files

| File | Purpose |
|------|---------|
| `src-tauri/Cargo.toml` | Tauri desktop app dependencies |
| `src-tauri/tauri.conf.json` | Tauri configuration |
| `src-tauri/src/lib.rs` | Tauri setup: tray, plugins, window events |
| `src-tauri/src/main.rs` | Entry point |
| `src-tauri/capabilities/default.json` | Permission grants for plugins |
| `src-tauri/icons/*` | App icons (generated) |
| `client/src/lib/config.ts` | Platform-aware server URL / API base |

### Modified Files

| File | Change |
|------|--------|
| `client/src/lib/stores/connection.svelte.ts` | Use configurable WS URL from `config.ts` instead of `window.location.host` |
| `client/src/lib/stores/auth.svelte.ts` | Use configurable API base URL; add `Authorization` header to fetch calls |
| `client/src/lib/stores/messages.svelte.ts` | Use configurable API base URL for message history fetch |
| `server/src/handlers/mod.rs` | CORS: allow `tauri://localhost` and `https://tauri.localhost` origins in release mode |
| `server/src/auth/middleware.rs` | Support `Authorization: Bearer` header as fallback to cookie |
| `client/package.json` | Add `@tauri-apps/api`, `@tauri-apps/plugin-notification`, `@tauri-apps/plugin-store` |
| `package.json` (root) | Add `dev:desktop` and `build:desktop` scripts |

### Unchanged

| Component | Why No Changes |
|-----------|---------------|
| `svelte.config.js` | Already configured for `adapter-static` with SPA fallback |
| `client/src/routes/+layout.ts` | Already has `ssr = false` |
| `server/src/ws/handler.rs` | WS auth uses token-in-message, works across origins |
| `server/src/ws/protocol.rs` | No protocol changes needed |
| Database/migrations | No schema changes |
| `Cargo.toml` (root workspace) | `src-tauri` is not part of the workspace; Tauri CLI manages it |

---

## Build & Development Workflow

### Development

Run three terminals:
1. `cd server && cargo watch -x run` -- Relay server on :3000
2. `cd client && npm run dev` -- Vite dev server on :5173
3. `cd src-tauri && cargo tauri dev` -- Tauri app loading from :5173

Tauri dev mode uses the Vite dev server (`devUrl: "http://localhost:5173"`), so HMR works normally. The Vite proxy still forwards `/api` and `/ws` to `:3000`.

**Note:** In Tauri dev mode, the SPA is loaded from `http://localhost:5173`, so same-origin API calls work (through Vite proxy). The configurable server URL feature is only needed in production builds. However, it should still be testable in dev by overriding the server URL in the store.

### Production Build

```bash
# Build SvelteKit client
cd client && npm run build

# Build Tauri desktop app (bundles the SvelteKit build)
cd src-tauri && cargo tauri build
```

Output: platform-specific installer (`.dmg` on macOS, `.msi` on Windows, `.deb`/`.AppImage` on Linux).

### Root package.json Scripts

```json
{
  "scripts": {
    "dev:client": "cd client && npm run dev",
    "dev:server": "cd server && cargo watch -x run",
    "dev:desktop": "cd src-tauri && cargo tauri dev",
    "build:desktop": "cd src-tauri && cargo tauri build",
    "build:client": "cd client && npm run build",
    "build:server": "cargo build --release",
    "build": "npm run build:client && npm run build:server"
  }
}
```

---

## Pitfalls Specific to This Phase

### 1. CORS Origin Differences Across Platforms

| Platform | WebView Origin |
|----------|---------------|
| macOS | `tauri://localhost` |
| Windows | `https://tauri.localhost` |
| Linux | `https://tauri.localhost` |

The server CORS allowlist must include BOTH origin formats. Test on at least two platforms before shipping.

### 2. Cookie SameSite Policy

The current session cookie uses `SameSite=Lax`, which prevents cookies from being sent on cross-origin requests (which is what Tauri-to-server requests are). Adding `Authorization: Bearer` header support is the correct fix. Do NOT change `SameSite` to `None` as that weakens browser security.

### 3. WebSocket Disconnect on Machine Sleep

When a laptop lid closes, the OS suspends the process. The WebSocket connection drops ungracefully (no close frame). The existing reconnection logic handles this:
- `onclose` fires when the OS wakes
- `scheduleReconnect()` triggers with exponential backoff
- Connection restores within 1-30 seconds

No code changes needed, but test this scenario explicitly.

### 4. Notification Permission Denied Permanently

On macOS, if the user denies notification permission, the app cannot re-request it. The user must go to System Settings > Notifications to enable it. The app should detect `permission === 'denied'` and show a help message directing the user to system settings rather than silently failing.

### 5. First-Time Server URL UX

The first-launch experience must be smooth:
1. App opens -> "Connect to Server" screen
2. User enters URL -> App validates by hitting the server
3. Validation succeeds -> Show login/register screen
4. User authenticates -> WebSocket connects -> Chat loads

If step 2 fails (wrong URL, server down, CORS misconfigured), show a clear error message. Do not show a blank screen.

### 6. Broadcast Polling Performance

The current WS handler (`ws/handler.rs` lines 178-201) uses a 100ms polling interval (`try_recv()` in a loop). This is noted as a concern in the project state. For a desktop app this is tolerable but wastes CPU. Fixing this is not required for Phase 1 but should be tracked.

---

## Questions Resolved by Research

| Question | Answer |
|----------|--------|
| Does Tauri support the browser WebSocket API for remote connections? | Yes. The webview's native `WebSocket` API works for connecting to remote servers. The Tauri WebSocket plugin is only needed for bypassing CORS issues, which should not apply to WSS connections. |
| Where does Tauri store webview data (localStorage, cookies)? | In the app's data directory (`~/Library/Application Support/` on macOS, `%APPDATA%` on Windows). This persists between app restarts. |
| Can the webview JS continue running when the window is hidden? | Yes. Hiding the window (not closing/destroying it) keeps the webview's JS engine active. The WebSocket connection and all stores continue operating. |
| Does the SvelteKit build need to change for Tauri? | No. The existing `adapter-static` + `ssr = false` + `fallback: 'index.html'` configuration is exactly what Tauri requires. |
| Should `src-tauri` be in the Cargo workspace? | No. Tauri manages its own dependency tree via the Tauri CLI. Adding it to the workspace would cause dependency conflicts (Tauri pins specific versions of tokio, serde, etc.). |
| Do we need to handle the Tauri webview's CSP? | Tauri v2 sets a default CSP that allows connecting to `tauri.localhost` and `https://*`. The default policy should allow WSS/HTTPS connections to the remote Relay server. If not, the CSP can be adjusted in `tauri.conf.json`. |
| Will `fetch()` with relative URLs work in Tauri? | No. Relative URLs resolve to the webview's local origin, not the remote server. All API calls must use absolute URLs when running in Tauri. |

---

## Open Questions for Planning

| Question | Impact | Suggested Resolution |
|----------|--------|---------------------|
| Should the "server URL" screen support multiple servers? | Low -- friend-group scale, likely one server | Single server for now. Multi-server is a v2+ feature. |
| Should we build a settings page now or defer? | Medium -- needed for server URL changes, notification prefs | Build a minimal settings page with just "Server URL" and "Disconnect" for Phase 1. Expand in Phase 2. |
| How should the app behave on auto-start at login? | Low -- nice to have | Defer auto-start to a polish pass. Not in DESK requirements. |
| Should we create a shared `fetchWithAuth()` wrapper? | High -- affects all API calls | Yes. All API calls should go through a single wrapper that adds the base URL and auth header. This is required for DESK-04 and simplifies future mobile work. |

---

## Estimated Plan Split

Based on the analysis, this phase naturally splits into two plans:

**Plan 01-01: Tauri Shell + Configurable Server URL**
- Initialize Tauri project (`src-tauri/`)
- Platform-aware config module (`config.ts`)
- Server URL entry screen (first-launch UX)
- `fetchWithAuth()` wrapper for all API calls
- Server CORS changes for Tauri origins
- `Authorization: Bearer` support in auth middleware
- Update connection store to use configurable WS URL
- System tray icon (basic -- just the icon, no close-to-tray yet)

**Plan 01-02: System Tray Persistence + Notifications**
- Close-to-tray behavior (hide window on close, persist in tray)
- Tray menu (Show/Hide, Quit)
- Notification permission flow
- Desktop notifications on incoming messages (when window unfocused)
- Notification rate limiting

This split ensures Plan 01-01 delivers a functional app (DESK-01, DESK-04) while Plan 01-02 adds the "feels native" behaviors (DESK-02, DESK-03).

---

## Sources

- [Tauri v2 SvelteKit Guide](https://v2.tauri.app/start/frontend/sveltekit/) -- Official setup docs (HIGH confidence)
- [Tauri v2 System Tray](https://v2.tauri.app/learn/system-tray/) -- Tray icon, menus, events (HIGH confidence)
- [Tauri v2 Notification Plugin](https://v2.tauri.app/plugin/notification/) -- Native notifications API (HIGH confidence)
- [Tauri v2 Store Plugin](https://v2.tauri.app/plugin/store/) -- Persistent key-value storage (HIGH confidence)
- [Tauri v2 Configuration Reference](https://v2.tauri.app/reference/config/) -- tauri.conf.json schema (HIGH confidence)
- [Tauri v2 HTTP Headers](https://v2.tauri.app/security/http-headers/) -- CSP and security headers (HIGH confidence)
- [Tauri v2 Window Customization](https://v2.tauri.app/learn/window-customization/) -- Close/hide behavior (HIGH confidence)
- [Tauri CORS Discussion](https://github.com/tauri-apps/tauri/discussions/6898) -- Origin handling for remote APIs (MEDIUM confidence)
- [Tauri Close-to-Tray Discussion](https://github.com/tauri-apps/tauri/discussions/2684) -- Community patterns for tray persistence (MEDIUM confidence)
- [Building a System Tray App with Tauri](https://tauritutorials.com/blog/building-a-system-tray-app-with-tauri) -- Tutorial with code examples (MEDIUM confidence)
- [Tauri v2 Stable Release](https://v2.tauri.app/blog/tauri-20/) -- Mobile support, v2 feature overview (HIGH confidence)
- [System Tray Only App Discussion](https://github.com/tauri-apps/tauri/discussions/11489) -- Keep app alive without windows (MEDIUM confidence)
- Project research: `.planning/research/ARCHITECTURE.md`, `STACK.md`, `PITFALLS.md` -- Prior research on this project (HIGH confidence)

---
*Phase 1 research completed: 2026-02-28*
