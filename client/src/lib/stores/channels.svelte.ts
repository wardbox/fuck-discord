import type { Channel } from '$lib/protocol/types';

class ChannelStore {
	channels = $state<Channel[]>([]);
	activeChannelId = $state<string | null>(null);
	activeChannel = $derived(this.channels.find((c) => c.id === this.activeChannelId) ?? null);

	// Unread tracking: channelId -> latest message id
	private latestMessageId = $state<Record<string, string>>({});
	// channelId -> last read message id (persisted)
	private lastReadId = $state<Record<string, string>>({});

	constructor() {
		try {
			const stored = localStorage.getItem('relay_last_read');
			if (stored) this.lastReadId = JSON.parse(stored);
		} catch { /* ignore */ }
	}

	hasUnread(channelId: string): boolean {
		const latest = this.latestMessageId[channelId];
		const read = this.lastReadId[channelId];
		if (!latest) return false;
		if (!read) return true;
		return latest > read; // ULIDs are lexicographically sortable by time
	}

	markRead(channelId: string) {
		const latest = this.latestMessageId[channelId];
		if (latest) {
			this.lastReadId = { ...this.lastReadId, [channelId]: latest };
			try {
				localStorage.setItem('relay_last_read', JSON.stringify(this.lastReadId));
			} catch { /* localStorage may be full or unavailable */ }
		}
	}

	trackMessage(channelId: string, messageId: string) {
		const current = this.latestMessageId[channelId];
		if (!current || messageId > current) {
			this.latestMessageId = { ...this.latestMessageId, [channelId]: messageId };
		}
		// Auto-mark read if this is the active channel
		if (channelId === this.activeChannelId) {
			this.markRead(channelId);
		}
	}

	setChannels(channels: Channel[]) {
		this.channels = channels;
		const hasActive = this.activeChannelId !== null && channels.some((c) => c.id === this.activeChannelId);
		if (!hasActive) {
			this.activeChannelId = channels[0]?.id ?? null;
		}
	}

	setActive(channelId: string) {
		this.activeChannelId = channelId;
		this.markRead(channelId);
	}

	addChannel(channel: Channel) {
		if (!this.channels.find((c) => c.id === channel.id)) {
			this.channels = [...this.channels, channel];
		}
	}

	updateChannel(channel: Channel) {
		this.channels = this.channels.map((c) => (c.id === channel.id ? channel : c));
	}

	removeChannel(channelId: string) {
		this.channels = this.channels.filter((c) => c.id !== channelId);
		if (this.activeChannelId === channelId) {
			this.activeChannelId = this.channels[0]?.id ?? null;
		}
	}
}

export const channelStore = new ChannelStore();
