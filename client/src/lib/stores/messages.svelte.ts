import type { Message, Reaction } from '$lib/protocol/types';
import { fetchWithAuth } from '$lib/config';

class MessageStore {
	// channelId -> messages
	private messagesByChannel = $state<Record<string, Message[]>>({});
	// Track which channels have been loaded
	private loadedChannels = new Set<string>();
	// Track if more history exists
	private hasMore = $state<Record<string, boolean>>({});
	// Currently editing message
	editingMessage = $state<Message | null>(null);

	reset() {
		this.messagesByChannel = {};
		this.hasMore = {};
		this.loadedChannels.clear();
		this.editingMessage = null;
	}

	startEditing(message: Message) {
		this.editingMessage = message;
	}

	cancelEditing() {
		this.editingMessage = null;
	}

	getMessages(channelId: string): Message[] {
		return this.messagesByChannel[channelId] ?? [];
	}

	hasMoreHistory(channelId: string): boolean {
		return this.hasMore[channelId] ?? true;
	}

	addMessage(message: Message) {
		const channelId = message.channel_id;
		const existing = this.messagesByChannel[channelId] ?? [];
		// Deduplicate by id
		if (existing.some((m) => m.id === message.id)) return;
		this.messagesByChannel[channelId] = [...existing, message];
	}

	updateMessage(message: Message) {
		const channelId = message.channel_id;
		const existing = this.messagesByChannel[channelId] ?? [];
		this.messagesByChannel[channelId] = existing.map((m) =>
			m.id === message.id ? message : m
		);
	}

	deleteMessage(channelId: string, messageId: string) {
		const existing = this.messagesByChannel[channelId] ?? [];
		this.messagesByChannel[channelId] = existing.filter((m) => m.id !== messageId);
	}

	updateReactions(channelId: string, messageId: string, reactions: Reaction[]) {
		const existing = this.messagesByChannel[channelId] ?? [];
		this.messagesByChannel[channelId] = existing.map((m) =>
			m.id === messageId ? { ...m, reactions } : m
		);
	}

	prependMessages(channelId: string, messages: Message[]) {
		const existing = this.messagesByChannel[channelId] ?? [];
		const existingIds = new Set(existing.map((m) => m.id));
		const newMessages = messages.filter((m) => !existingIds.has(m.id));
		this.messagesByChannel[channelId] = [...newMessages, ...existing];

		if (messages.length < 50) {
			this.hasMore[channelId] = false;
		}
	}

	async loadHistory(channelId: string, before?: string): Promise<void> {
		if (!before && this.loadedChannels.has(channelId)) return;

		const params = new URLSearchParams({ limit: '50' });
		if (before) params.set('before', before);

		try {
			const res = await fetchWithAuth(`/api/channels/${channelId}/messages?${params}`);
			if (!res.ok) return;

			const messages: Message[] = await res.json();
			if (!before) {
				this.messagesByChannel[channelId] = messages;
				this.loadedChannels.add(channelId);
				if (messages.length < 50) {
					this.hasMore[channelId] = false;
				}
			} else {
				this.prependMessages(channelId, messages);
			}
		} catch {
			// Network error — silently return so the UI can retry later
			return;
		}
	}
}

export const messageStore = new MessageStore();
