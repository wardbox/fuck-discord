<script lang="ts">
	import { memberStore } from '$lib/stores/members.svelte';

	function statusColor(status: string): string {
		switch (status) {
			case 'online': return 'bg-online';
			case 'idle': return 'bg-idle';
			case 'dnd': return 'bg-dnd';
			default: return 'bg-text-muted';
		}
	}
</script>

<aside class="hidden w-52 flex-col border-l border-border bg-bg-secondary py-3 lg:flex">
	{#if memberStore.onlineMembers.length > 0}
		<div class="px-3 py-1">
			<span class="text-xs font-semibold uppercase text-text-muted">
				Online — {memberStore.onlineMembers.length}
			</span>
		</div>
		{#each memberStore.onlineMembers as member (member.id)}
			<div class="flex items-center gap-2 px-3 py-1 hover:bg-bg-hover">
				<div class="relative">
					<div class="h-7 w-7 rounded-full bg-accent/50 flex items-center justify-center text-xs font-medium text-white">
						{member.username.charAt(0).toUpperCase()}
					</div>
					<div class="absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full border-2 border-bg-secondary {statusColor(member.status)}"></div>
				</div>
				<span class="truncate text-sm text-text-secondary">{member.display_name ?? member.username}</span>
			</div>
		{/each}
	{/if}

	{#if memberStore.offlineMembers.length > 0}
		<div class="px-3 py-1 mt-2">
			<span class="text-xs font-semibold uppercase text-text-muted">
				Offline — {memberStore.offlineMembers.length}
			</span>
		</div>
		{#each memberStore.offlineMembers as member (member.id)}
			<div class="flex items-center gap-2 px-3 py-1 opacity-50 hover:bg-bg-hover">
				<div class="relative">
					<div class="h-7 w-7 rounded-full bg-accent/30 flex items-center justify-center text-xs font-medium text-white/50">
						{member.username.charAt(0).toUpperCase()}
					</div>
				</div>
				<span class="truncate text-sm text-text-muted">{member.display_name ?? member.username}</span>
			</div>
		{/each}
	{/if}
</aside>
