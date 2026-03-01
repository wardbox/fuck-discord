<script lang="ts">
	import { channelStore } from '$lib/stores/channels.svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { connection } from '$lib/stores/connection.svelte';
	import { goto } from '$app/navigation';
	import { Hash, LogOut, Plus, Wifi, WifiOff, ChevronDown } from 'lucide-svelte';

	let showCreateChannel = $state(false);
	let newChannelName = $state('');
	let newChannelCategory = $state('');
	let createError = $state('');

	// Group channels by category
	let channelGroups = $derived.by(() => {
		const groups: { category: string | null; channels: typeof channelStore.channels }[] = [];
		const byCategory = new Map<string | null, typeof channelStore.channels>();

		for (const ch of channelStore.channels) {
			const cat = ch.category ?? null;
			if (!byCategory.has(cat)) byCategory.set(cat, []);
			byCategory.get(cat)!.push(ch);
		}

		// Uncategorized channels first
		const uncategorized = byCategory.get(null);
		if (uncategorized) {
			groups.push({ category: null, channels: uncategorized });
			byCategory.delete(null);
		}

		// Then alphabetical categories
		for (const [cat, channels] of [...byCategory.entries()].sort((a, b) =>
			(a[0] ?? '').localeCompare(b[0] ?? '')
		)) {
			groups.push({ category: cat, channels });
		}

		return groups;
	});

	async function createChannel() {
		const name = newChannelName.trim();
		if (!name) return;

		const body: Record<string, string> = { name };
		if (newChannelCategory.trim()) {
			body.category = newChannelCategory.trim();
		}

		createError = '';
		try {
			const res = await fetch('/api/channels', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(body)
			});

			if (res.ok) {
				newChannelName = '';
				newChannelCategory = '';
				showCreateChannel = false;
			} else {
				const err = await res.json().catch(() => ({}));
				createError = err.error || `Failed (${res.status})`;
			}
		} catch (e) {
			createError = 'Network error';
		}
	}

	async function handleLogout() {
		connection.disconnect();
		try {
			await auth.logout();
		} finally {
			goto('/');
		}
	}
</script>

<aside class="flex h-full w-60 flex-col border-r border-border bg-bg-secondary">
	<!-- Server header -->
	<div class="flex items-center justify-between border-b border-border px-4 py-3">
		<h2 class="text-sm font-semibold text-text-primary">Relay</h2>
		<div class="flex items-center gap-1">
			{#if connection.connected}
				<Wifi size={14} class="text-online" />
			{:else}
				<WifiOff size={14} class="text-danger" />
			{/if}
		</div>
	</div>

	<!-- Channel list -->
	<div class="flex-1 overflow-y-auto py-2">
		<div class="flex items-center justify-between px-3 py-1">
			<span class="text-xs font-semibold uppercase text-text-muted">Channels</span>
			<button
				onclick={() => (showCreateChannel = !showCreateChannel)}
				class="rounded p-0.5 text-text-muted hover:text-text-primary"
				title="Create channel"
			>
				<Plus size={14} />
			</button>
		</div>

		{#if showCreateChannel}
			<div class="px-2 py-1">
				<form onsubmit={(e) => { e.preventDefault(); createChannel(); }} class="flex flex-col gap-1">
					<input
						type="text"
						bind:value={newChannelName}
						placeholder="channel-name"
						class="w-full rounded border border-border bg-bg-input px-2 py-1 text-sm text-text-primary placeholder-text-muted focus:border-accent focus:outline-none"
						autofocus
					/>
					<input
						type="text"
						bind:value={newChannelCategory}
						placeholder="category (optional)"
						class="w-full rounded border border-border bg-bg-input px-2 py-1 text-xs text-text-primary placeholder-text-muted focus:border-accent focus:outline-none"
					/>
					<button
						type="submit"
						disabled={!newChannelName.trim()}
						class="w-full rounded bg-accent px-2 py-1 text-xs text-white hover:bg-accent-hover disabled:opacity-30"
					>
						Create
					</button>
					{#if createError}
						<p class="text-xs text-red-400">{createError}</p>
					{/if}
				</form>
			</div>
		{/if}

		{#each channelGroups as group}
			{#if group.category}
				<div class="mt-3 flex items-center gap-1 px-3 py-0.5">
					<ChevronDown size={10} class="text-text-muted" />
					<span class="text-[10px] font-semibold uppercase tracking-wide text-text-muted">{group.category}</span>
				</div>
			{/if}
			{#each group.channels as channel (channel.id)}
				<button
					onclick={() => {
						channelStore.setActive(channel.id);
						goto(`/app/${channel.id}`);
					}}
					class="flex w-full items-center gap-1.5 rounded-sm px-3 py-1 text-sm
						{channelStore.activeChannelId === channel.id
							? 'bg-bg-hover text-text-primary'
							: channelStore.hasUnread(channel.id)
								? 'text-text-primary font-medium hover:bg-bg-hover'
								: 'text-text-secondary hover:bg-bg-hover hover:text-text-primary'}"
				>
					<Hash size={16} class="shrink-0 {channelStore.hasUnread(channel.id) ? 'text-text-primary' : 'text-text-muted'}" />
					<span class="truncate">{channel.name}</span>
					{#if channelStore.hasUnread(channel.id) && channelStore.activeChannelId !== channel.id}
						<div class="ml-auto h-2 w-2 shrink-0 rounded-full bg-accent"></div>
					{/if}
				</button>
			{/each}
		{/each}
	</div>

	<!-- User panel -->
	<div class="flex items-center gap-2 border-t border-border px-3 py-2">
		<div class="relative">
			<div class="h-8 w-8 rounded-full bg-accent flex items-center justify-center text-xs font-bold text-white">
				{auth.user?.username?.charAt(0).toUpperCase() ?? '?'}
			</div>
			<div class="absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full border-2 border-bg-secondary bg-online"></div>
		</div>
		<div class="min-w-0 flex-1">
			<p class="truncate text-sm font-medium text-text-primary">{auth.user?.username}</p>
		</div>
		<button
			onclick={handleLogout}
			class="rounded p-1 text-text-muted hover:text-danger"
			title="Log out"
		>
			<LogOut size={16} />
		</button>
	</div>
</aside>
