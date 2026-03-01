<script lang="ts">
	import { messageStore } from '$lib/stores/messages.svelte';
	import { channelStore } from '$lib/stores/channels.svelte';
	import { typingStore } from '$lib/stores/connection.svelte';
	import Message from './Message.svelte';
	import { tick } from 'svelte';

	let container: HTMLDivElement;
	let shouldAutoScroll = $state(true);
	let loadingMore = $state(false);

	// Get messages for active channel
	let messages = $derived(
		channelStore.activeChannelId
			? messageStore.getMessages(channelStore.activeChannelId)
			: []
	);

	let typingUsers = $derived(
		channelStore.activeChannelId
			? typingStore.getTyping(channelStore.activeChannelId)
			: []
	);

	// Auto-scroll when new messages arrive
	$effect(() => {
		// Reference messages to track changes
		const _ = messages.length;
		if (shouldAutoScroll) {
			tick().then(() => {
				if (container) {
					container.scrollTop = container.scrollHeight;
				}
			});
		}
	});

	function handleScroll() {
		if (!container) return;

		// Check if user scrolled to bottom
		const { scrollTop, scrollHeight, clientHeight } = container;
		shouldAutoScroll = scrollHeight - scrollTop - clientHeight < 50;

		// Load more history when scrolled to top
		if (scrollTop < 100 && !loadingMore && channelStore.activeChannelId) {
			const channelMessages = messageStore.getMessages(channelStore.activeChannelId);
			if (channelMessages.length > 0 && messageStore.hasMoreHistory(channelStore.activeChannelId)) {
				loadMore();
			}
		}
	}

	async function loadMore() {
		if (!channelStore.activeChannelId || loadingMore) return;
		const channelMessages = messageStore.getMessages(channelStore.activeChannelId);
		if (channelMessages.length === 0) return;

		loadingMore = true;
		const oldScrollHeight = container.scrollHeight;

		await messageStore.loadHistory(channelStore.activeChannelId, channelMessages[0].id);

		// Maintain scroll position after prepending
		await tick();
		if (container) {
			container.scrollTop = container.scrollHeight - oldScrollHeight;
		}
		loadingMore = false;
	}
</script>

<div
	class="flex-1 overflow-y-auto px-4 py-2"
	bind:this={container}
	onscroll={handleScroll}
>
	{#if loadingMore}
		<div class="py-2 text-center text-xs text-text-muted">Loading older messages...</div>
	{/if}

	{#if messages.length === 0}
		<div class="flex h-full items-center justify-center">
			<div class="text-center">
				<p class="text-lg text-text-secondary">No messages yet</p>
				<p class="mt-1 text-sm text-text-muted">Be the first to say something.</p>
			</div>
		</div>
	{:else}
		{#each messages as message, i (message.id)}
			{@const prevMessage = i > 0 ? messages[i - 1] : null}
			{@const isGrouped = prevMessage
				&& prevMessage.author_id === message.author_id
				&& new Date(message.created_at).getTime() - new Date(prevMessage.created_at).getTime() < 300000}
			<Message {message} compact={isGrouped} />
		{/each}
	{/if}

	{#if typingUsers.length > 0}
		<div class="py-1 text-xs text-text-muted">
			{typingUsers.join(', ')} {typingUsers.length === 1 ? 'is' : 'are'} typing...
		</div>
	{/if}
</div>
