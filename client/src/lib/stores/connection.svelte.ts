import type { ClientMessage, ServerMessage } from '$lib/protocol/types';
import { getWsUrl } from '$lib/config';
import { auth } from './auth.svelte';
import { channelStore } from './channels.svelte';
import { memberStore } from './members.svelte';
import { messageStore } from './messages.svelte';

class ConnectionStore {
	private ws: WebSocket | null = null;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private reconnectDelay = 1000;
	private idleTimer: ReturnType<typeof setTimeout> | null = null;
	private isIdle = false;
	private activityHandler: (() => void) | null = null;
	private authRejected = false;

	connected = $state(false);
	connecting = $state(false);

	connect() {
		if (this.ws || this.connecting || !auth.sessionId) return;

		this.connecting = true;
		const url = getWsUrl();

		try {
			this.ws = new WebSocket(url);
		} catch {
			this.connecting = false;
			this.scheduleReconnect();
			return;
		}

		this.ws.onopen = () => {
			this.connecting = false;
			this.reconnectDelay = 1000;
			this.authRejected = false;
			// Authenticate
			this.send({ type: 'authenticate', token: auth.sessionId! });
		};

		this.ws.onmessage = (event) => {
			try {
				const msg: ServerMessage = JSON.parse(event.data);
				this.handleMessage(msg);
			} catch (e) {
				console.error('Failed to parse message:', e);
			}
		};

		this.ws.onclose = () => {
			this.ws = null;
			this.connected = false;
			this.connecting = false;
			if (auth.isAuthenticated && !this.authRejected) {
				this.scheduleReconnect();
			}
		};

		this.ws.onerror = () => {
			// onclose will fire after this
		};
	}

	disconnect() {
		this.stopIdleDetection();
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}
		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}
		this.connected = false;
		this.connecting = false;
	}

	private startIdleDetection() {
		this.stopIdleDetection();
		const IDLE_TIMEOUT = 5 * 60 * 1000; // 5 minutes

		const resetIdle = () => {
			if (this.idleTimer) clearTimeout(this.idleTimer);
			if (this.isIdle) {
				this.isIdle = false;
				this.send({ type: 'set_status', status: 'online' });
			}
			this.idleTimer = setTimeout(() => {
				this.isIdle = true;
				this.send({ type: 'set_status', status: 'idle' });
			}, IDLE_TIMEOUT);
		};

		this.activityHandler = resetIdle;
		const events = ['mousedown', 'keydown', 'scroll', 'touchstart'] as const;
		events.forEach((e) => { document.addEventListener(e, resetIdle, { passive: true }); });
		resetIdle();
	}

	private stopIdleDetection() {
		if (this.idleTimer) {
			clearTimeout(this.idleTimer);
			this.idleTimer = null;
		}
		if (this.activityHandler) {
			const events = ['mousedown', 'keydown', 'scroll', 'touchstart'] as const;
			events.forEach((e) => { document.removeEventListener(e, this.activityHandler!); });
			this.activityHandler = null;
		}
		this.isIdle = false;
	}

	send(msg: ClientMessage) {
		if (this.ws?.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(msg));
		}
	}

	sendMessage(channelId: string, content: string) {
		const nonce = crypto.randomUUID();
		this.send({ type: 'send_message', channel_id: channelId, content, nonce });
	}

	editMessage(messageId: string, content: string) {
		this.send({ type: 'edit_message', message_id: messageId, content });
	}

	deleteMessage(messageId: string) {
		this.send({ type: 'delete_message', message_id: messageId });
	}

	addReaction(messageId: string, emoji: string) {
		this.send({ type: 'add_reaction', message_id: messageId, emoji });
	}

	removeReaction(messageId: string, emoji: string) {
		this.send({ type: 'remove_reaction', message_id: messageId, emoji });
	}

	sendTyping(channelId: string) {
		this.send({ type: 'typing', channel_id: channelId });
	}

	private handleMessage(msg: ServerMessage) {
		switch (msg.type) {
			case 'ready':
				this.connected = true;
				channelStore.setChannels(msg.channels);
				memberStore.setMembers(msg.members);
				this.startIdleDetection();
				// Load initial messages for active channel
				if (channelStore.activeChannelId) {
					messageStore.loadHistory(channelStore.activeChannelId);
				}
				break;

			case 'message_create':
				messageStore.addMessage(msg.message);
				channelStore.trackMessage(msg.message.channel_id, msg.message.id);
				typingStore.clearTyping(msg.message.channel_id, msg.message.author_id);
				break;

			case 'message_update':
				messageStore.updateMessage(msg.message);
				break;

			case 'message_delete':
				messageStore.deleteMessage(msg.channel_id, msg.message_id);
				break;

			case 'typing_start':
				// Don't show your own typing indicator
				if (msg.user_id !== auth.user?.id) {
					typingStore.addTyping(msg.channel_id, msg.user_id, msg.username);
				}
				break;

			case 'presence_update':
				memberStore.updatePresence(msg.user_id, msg.status);
				break;

			case 'channel_create':
				channelStore.addChannel(msg.channel);
				break;

			case 'channel_update':
				channelStore.updateChannel(msg.channel);
				break;

			case 'channel_delete':
				channelStore.removeChannel(msg.channel_id);
				break;

			case 'member_join':
				memberStore.addMember(msg.user);
				break;

			case 'member_leave':
				memberStore.removeMember(msg.user_id);
				break;

			case 'reaction_update':
				messageStore.updateReactions(msg.channel_id, msg.message_id, msg.reactions);
				break;

			case 'error':
				console.error('Server error:', msg.code, msg.message);
				if (msg.code === 'auth_failed') {
					this.authRejected = true;
					this.disconnect();
				}
				break;
		}
	}

	private scheduleReconnect() {
		if (this.reconnectTimer) return;
		this.reconnectTimer = setTimeout(() => {
			this.reconnectTimer = null;
			this.reconnectDelay = Math.min(this.reconnectDelay * 2, 30000);
			this.connect();
		}, this.reconnectDelay);
	}
}

// Simple typing indicator tracker
class TypingStore {
	// channelId -> { userId: { username, timeout } }
	private typingUsers = $state<Record<string, Record<string, string>>>({});

	getTyping(channelId: string): string[] {
		const users = this.typingUsers[channelId];
		if (!users) return [];
		return Object.values(users);
	}

	private timeouts = new Map<string, ReturnType<typeof setTimeout>>();

	addTyping(channelId: string, userId: string, username: string) {
		if (!this.typingUsers[channelId]) {
			this.typingUsers[channelId] = {};
		}
		this.typingUsers[channelId][userId] = username;

		// Clear any existing timeout for this user+channel
		const key = `${channelId}:${userId}`;
		const existing = this.timeouts.get(key);
		if (existing) clearTimeout(existing);

		// Auto-clear after 3 seconds
		this.timeouts.set(key, setTimeout(() => {
			this.timeouts.delete(key);
			if (this.typingUsers[channelId]?.[userId]) {
				const updated = { ...this.typingUsers[channelId] };
				delete updated[userId];
				this.typingUsers[channelId] = updated;
			}
		}, 3000));
	}

	clearTyping(channelId: string, userId: string) {
		const key = `${channelId}:${userId}`;
		const existing = this.timeouts.get(key);
		if (existing) {
			clearTimeout(existing);
			this.timeouts.delete(key);
		}
		if (this.typingUsers[channelId]?.[userId]) {
			const updated = { ...this.typingUsers[channelId] };
			delete updated[userId];
			this.typingUsers[channelId] = updated;
		}
	}
}

export const connection = new ConnectionStore();
export const typingStore = new TypingStore();
