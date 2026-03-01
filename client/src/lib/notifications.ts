import { isTauri } from '$lib/config';

/** Rate limiter: channelId -> last notification timestamp */
const lastNotified = new Map<string, number>();
const RATE_LIMIT_MS = 3000; // Max 1 notification per channel per 3 seconds

/** Whether we have ever requested permission (to avoid repeat prompts) */
let permissionRequested = false;

/**
 * Show a native OS notification for an incoming message.
 * No-ops silently in browser mode or when window is focused.
 *
 * @param channelName - Channel name for the notification title
 * @param authorUsername - Message author's display name
 * @param content - Message content (will be truncated)
 * @param channelId - Channel ID for rate limiting
 */
export async function showMessageNotification(
	channelName: string,
	authorUsername: string,
	content: string,
	channelId: string
): Promise<void> {
	// Only in Tauri
	if (!isTauri()) return;

	// Only when window is unfocused
	if (document.hasFocus()) return;

	// Rate limit per channel
	const now = Date.now();
	const lastTime = lastNotified.get(channelId) ?? 0;
	if (now - lastTime < RATE_LIMIT_MS) return;
	lastNotified.set(channelId, now);

	try {
		const {
			isPermissionGranted,
			requestPermission,
			sendNotification
		} = await import('@tauri-apps/plugin-notification');

		let granted = await isPermissionGranted();

		// Request permission on first notification attempt (not on app launch)
		if (!granted && !permissionRequested) {
			permissionRequested = true;
			const permission = await requestPermission();
			granted = permission === 'granted';
		}

		if (!granted) return;

		// Truncate content for notification body
		const truncated = content.length > 100 ? content.slice(0, 100) + '...' : content;

		sendNotification({
			title: `#${channelName}`,
			body: `${authorUsername}: ${truncated}`
		});
	} catch (e) {
		// Silently fail -- notification errors should never break the app
		console.warn('Notification error:', e);
	}
}
