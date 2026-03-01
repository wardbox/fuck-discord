/**
 * Platform-aware configuration for Relay.
 *
 * In browser: all URLs are same-origin (relative paths work).
 * In Tauri: URLs must be absolute, pointing to the user-configured server.
 */

let _serverUrl: string | null = null;
let _initialized = false;

/** Check if running inside Tauri webview */
export function isTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** Initialize config. Must be called once at app startup before any API calls. */
export async function initConfig(): Promise<void> {
	if (_initialized) return;

	if (isTauri()) {
		const { load } = await import('@tauri-apps/plugin-store');
		const store = await load('settings.json', { autoSave: true, defaults: {} });
		_serverUrl = (await store.get<string>('serverUrl')) ?? null;
	} else {
		_serverUrl = window.location.origin;
	}
	_initialized = true;
}

/** Get the configured server URL. Returns null if not configured (Tauri first launch). */
export function getServerUrl(): string | null {
	return _serverUrl;
}

/** Check if a server URL has been configured */
export function isServerConfigured(): boolean {
	return _serverUrl !== null && _serverUrl !== '';
}

/** Set server URL (Tauri only). Persists to Tauri store. */
export async function setServerUrl(url: string): Promise<void> {
	if (!isTauri()) return;
	const { load } = await import('@tauri-apps/plugin-store');
	const store = await load('settings.json', { autoSave: true, defaults: {} });
	// Normalize: remove trailing slash
	const normalized = url.replace(/\/+$/, '');
	await store.set('serverUrl', normalized);
	_serverUrl = normalized;
}

/** Clear server URL (disconnect/reset). */
export async function clearServerUrl(): Promise<void> {
	if (!isTauri()) return;
	const { load } = await import('@tauri-apps/plugin-store');
	const store = await load('settings.json', { autoSave: true, defaults: {} });
	await store.delete('serverUrl');
	_serverUrl = null;
}

/** Build the WebSocket URL from the server URL. */
export function getWsUrl(): string {
	if (!_serverUrl) throw new Error('Server URL not configured');
	const url = new URL(_serverUrl);
	const protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
	return `${protocol}//${url.host}/ws`;
}

/** Build an absolute API URL from a relative path like '/api/auth/login'. */
export function getApiUrl(path: string): string {
	if (!_serverUrl) throw new Error('Server URL not configured');
	return new URL(path, _serverUrl).href;
}

/**
 * Authenticated fetch wrapper. Adds:
 * - Absolute URL (from server config)
 * - Authorization: Bearer header (when session exists)
 */
export async function fetchWithAuth(path: string, options: RequestInit = {}): Promise<Response> {
	const url = getApiUrl(path);
	const headers = new Headers(options.headers);

	// Add auth header if we have a session
	const stored = localStorage.getItem('relay_session');
	if (stored) {
		try {
			const data = JSON.parse(stored);
			if (data.session_id) {
				headers.set('Authorization', `Bearer ${data.session_id}`);
			}
		} catch {
			/* ignore parse errors */
		}
	}

	return fetch(url, { ...options, headers });
}
