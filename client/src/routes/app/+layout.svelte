<script lang="ts">
	import { auth } from '$lib/stores/auth.svelte';
	import { connection } from '$lib/stores/connection.svelte';
	import { goto } from '$app/navigation';
	import { onMount, onDestroy } from 'svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';

	let { children } = $props();

	onMount(async () => {
		if (!auth.sessionId) {
			goto('/');
			return;
		}

		const valid = await auth.checkSession();
		if (!valid) {
			goto('/');
			return;
		}

		connection.connect();
	});

	onDestroy(() => {
		connection.disconnect();
	});
</script>

{#if auth.isAuthenticated}
	<div class="flex h-screen">
		<Sidebar />
		<main class="flex min-w-0 flex-1 flex-col">
			{@render children()}
		</main>
	</div>
{/if}
