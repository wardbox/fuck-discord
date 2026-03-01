<script lang="ts">
	import { goto } from '$app/navigation';
	import { onDestroy } from 'svelte';
	import type { Message as MessageType } from '$lib/protocol/types';
	import { channelStore } from '$lib/stores/channels.svelte';
	import { formatCompactTimestamp } from '$lib/utils/time';
	import { X, Search } from 'lucide-svelte';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();

	let query = $state('');
	let results = $state<MessageType[]>([]);
	let searching = $state(false);
	let searched = $state(false);
	let error = $state('');
	let abortController: AbortController | null = null;

	async function handleSearch(e: Event) {
		e.preventDefault();
		const q = query.trim();
		if (!q) return;

		abortController?.abort();
		const controller = new AbortController();
		abortController = controller;

		searching = true;
		searched = true;
		error = '';

		try {
			const res = await fetch(`/api/search?q=${encodeURIComponent(q)}`, { signal: controller.signal });
			if (abortController !== controller) return;
			if (res.ok) {
				results = await res.json();
			} else {
				results = [];
				error = `Search failed (${res.status})`;
			}
		} catch (err) {
			if (err instanceof DOMException && err.name === 'AbortError') return;
			if (abortController !== controller) return;
			results = [];
			error = 'Network error — could not reach server.';
		} finally {
			if (abortController === controller) {
				searching = false;
			}
		}
	}

	function navigateToMessage(msg: MessageType) {
		channelStore.setActive(msg.channel_id);
		goto(`/app/${msg.channel_id}`);
		onClose();
	}

	onDestroy(() => {
		abortController?.abort();
		abortController = null;
	});
</script>

<div class="flex h-full w-80 flex-col border-l border-border bg-bg-secondary">
	<div class="flex items-center justify-between border-b border-border px-3 py-2">
		<span class="text-sm font-semibold text-text-primary">Search</span>
		<button onclick={onClose} class="rounded p-1 text-text-muted hover:text-text-primary" aria-label="Close search">
			<X size={16} />
		</button>
	</div>

	<form onsubmit={handleSearch} class="border-b border-border p-3">
		<div class="flex gap-2">
			<input
				type="text"
				bind:value={query}
				placeholder="Search messages..."
				class="min-w-0 flex-1 rounded border border-border bg-bg-input px-2 py-1.5 text-sm text-text-primary placeholder-text-muted focus:border-accent focus:outline-none"
				autofocus
			/>
			<button type="submit" disabled={searching} class="rounded bg-accent px-2 py-1.5 text-white hover:bg-accent-hover disabled:opacity-50" aria-label="Search">
				<Search size={14} />
			</button>
		</div>
	</form>

	<div class="flex-1 overflow-y-auto">
		{#if searching}
			<div class="p-4 text-center text-sm text-text-muted">Searching...</div>
		{:else if error}
			<div class="p-4 text-center text-sm text-danger">{error}</div>
		{:else if searched && results.length === 0}
			<div class="p-4 text-center text-sm text-text-muted">No results found.</div>
		{:else}
			{#each results as msg (msg.id)}
				<button
					type="button"
					onclick={() => navigateToMessage(msg)}
					class="block w-full border-b border-border p-3 text-left hover:bg-bg-hover cursor-pointer"
				>
					<div class="flex items-baseline gap-2">
						<span class="text-xs font-medium text-accent">{msg.author_username}</span>
						<span class="text-xs text-text-muted">{formatCompactTimestamp(msg.created_at)}</span>
					</div>
					<p class="mt-0.5 text-sm text-text-secondary">{msg.content}</p>
				</button>
			{/each}
		{/if}
	</div>
</div>
