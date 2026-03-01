<script lang="ts">
	import { auth } from '$lib/stores/auth.svelte';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import LoginForm from '$lib/components/LoginForm.svelte';

	onMount(async () => {
		if (auth.sessionId) {
			const valid = await auth.checkSession();
			if (valid) {
				goto('/app');
			}
		}
	});
</script>

{#if !auth.isAuthenticated}
	<LoginForm />
{/if}
