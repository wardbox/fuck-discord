<script lang="ts">
	import { page } from '$app/stores';
	import { channelStore } from '$lib/stores/channels.svelte';
	import { messageStore } from '$lib/stores/messages.svelte';
	import { connection } from '$lib/stores/connection.svelte';
	import ChannelHeader from '$lib/components/ChannelHeader.svelte';
	import MessageList from '$lib/components/MessageList.svelte';
	import MessageInput from '$lib/components/MessageInput.svelte';
	import MemberList from '$lib/components/MemberList.svelte';
	import SearchPanel from '$lib/components/SearchPanel.svelte';

	let showSearch = $state(false);

	// Sync URL param to active channel
	$effect(() => {
		const channelId = $page.params.channelId;
		if (channelId && channelId !== channelStore.activeChannelId) {
			channelStore.setActive(channelId);
		}
	});

	// Load messages when channel changes
	$effect(() => {
		const channelId = channelStore.activeChannelId;
		if (channelId && connection.connected) {
			messageStore.loadHistory(channelId);
		}
	});
</script>

<div class="flex h-full">
	<div class="flex min-w-0 flex-1 flex-col">
		<ChannelHeader onToggleSearch={() => (showSearch = !showSearch)} />
		<MessageList />
		<MessageInput />
	</div>

	{#if showSearch}
		<SearchPanel onClose={() => (showSearch = false)} />
	{/if}

	{#if !showSearch}
		<MemberList />
	{/if}
</div>
