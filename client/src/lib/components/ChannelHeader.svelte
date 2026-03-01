<script lang="ts">
	import { channelStore } from '$lib/stores/channels.svelte';
	import { Hash, Search } from 'lucide-svelte';
	import { fetchWithAuth } from '$lib/config';

	interface Props {
		onToggleSearch?: () => void;
	}

	let { onToggleSearch }: Props = $props();

	let editingTopic = $state(false);
	let topicInput = $state('');
	let savingTopic = $state(false);

	function startEditTopic() {
		topicInput = channelStore.activeChannel?.topic ?? '';
		editingTopic = true;
	}

	async function saveTopic() {
		const channel = channelStore.activeChannel;
		if (!channel || savingTopic) return;
		const channelId = channel.id;
		const newTopic = topicInput.trim() || null;
		if (newTopic === (channel.topic ?? null)) {
			editingTopic = false;
			return;
		}

		savingTopic = true;
		try {
			const res = await fetchWithAuth(`/api/channels/${channelId}`, {
				method: 'PATCH',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ topic: newTopic })
			});
			if (res.ok) {
				const current = channelStore.activeChannel;
				if (current && current.id === channelId) {
					channelStore.updateChannel({ ...current, topic: newTopic });
				}
				editingTopic = false;
			}
		} finally {
			savingTopic = false;
		}
	}

	function handleTopicKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			saveTopic();
		} else if (e.key === 'Escape') {
			editingTopic = false;
		}
	}
</script>

<header class="flex items-center justify-between border-b border-border px-4 py-2.5">
	<div class="flex min-w-0 flex-1 items-center gap-2">
		{#if channelStore.activeChannel}
			<Hash size={18} class="shrink-0 text-text-muted" />
			<h2 class="shrink-0 font-semibold text-text-primary">{channelStore.activeChannel.name}</h2>
			{#if editingTopic}
				<input
					type="text"
					bind:value={topicInput}
					onkeydown={handleTopicKeydown}
					onblur={saveTopic}
					placeholder="Set a topic..."
					class="min-w-0 flex-1 border-b border-border bg-transparent px-1 text-sm text-text-secondary focus:border-accent focus:outline-none"
					autofocus
				/>
			{:else}
				<button
					onclick={startEditTopic}
					class="min-w-0 truncate text-left text-sm text-text-muted hover:text-text-secondary"
					title="Click to edit topic"
				>
					{channelStore.activeChannel.topic ? `— ${channelStore.activeChannel.topic}` : '(set topic)'}
				</button>
			{/if}
		{/if}
	</div>

	<div class="flex shrink-0 items-center gap-2">
		{#if onToggleSearch}
			<button
				onclick={onToggleSearch}
				class="rounded p-1.5 text-text-muted hover:text-text-primary hover:bg-bg-hover"
				title="Search messages (Ctrl+F)"
				aria-label="Search messages"
			>
				<Search size={18} />
			</button>
		{/if}
	</div>
</header>
