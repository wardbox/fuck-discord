<script lang="ts">
	import { channelStore } from '$lib/stores/channels.svelte';
	import { goto } from '$app/navigation';

	// Redirect to first channel
	$effect(() => {
		if (channelStore.activeChannelId) {
			goto(`/app/${channelStore.activeChannelId}`, { replaceState: true });
		}
	});
</script>

<div class="flex h-full items-center justify-center">
	<p class="text-text-muted">Select a channel</p>
</div>
