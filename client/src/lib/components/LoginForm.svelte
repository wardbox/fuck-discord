<script lang="ts">
	import { auth } from '$lib/stores/auth.svelte';
	import { goto } from '$app/navigation';

	let mode = $state<'login' | 'register'>('login');
	let username = $state('');
	let password = $state('');
	let inviteCode = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;

		try {
			if (mode === 'login') {
				await auth.login(username, password);
			} else {
				await auth.register(username, password, inviteCode);
			}
			goto('/app');
		} catch (err) {
			error = err instanceof Error ? err.message : 'Something went wrong';
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex min-h-screen items-center justify-center bg-bg-primary">
	<div class="w-full max-w-md space-y-8 p-8">
		<div class="text-center">
			<h1 class="text-4xl font-bold text-text-primary">Relay</h1>
			<p class="mt-2 text-sm text-text-secondary">
				{mode === 'login' ? 'Welcome back.' : 'Join the conversation.'}
			</p>
		</div>

		<form onsubmit={handleSubmit} class="space-y-4">
			{#if error}
				<div class="rounded bg-danger/10 px-4 py-2 text-sm text-danger">{error}</div>
			{/if}

			<div>
				<label for="username" class="block text-sm text-text-secondary">Username</label>
				<input
					id="username"
					type="text"
					bind:value={username}
					required
					minlength={2}
					maxlength={32}
					autocomplete="username"
					class="mt-1 w-full rounded border border-border bg-bg-input px-3 py-2 text-text-primary placeholder-text-muted focus:border-accent focus:outline-none"
					placeholder="your username"
				/>
			</div>

			<div>
				<label for="password" class="block text-sm text-text-secondary">Passphrase</label>
				<input
					id="password"
					type="password"
					bind:value={password}
					required
					minlength={8}
					autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
					class="mt-1 w-full rounded border border-border bg-bg-input px-3 py-2 text-text-primary placeholder-text-muted focus:border-accent focus:outline-none"
					placeholder="at least 8 characters"
				/>
			</div>

			{#if mode === 'register'}
				<div>
					<label for="invite" class="block text-sm text-text-secondary">Invite Code</label>
					<input
						id="invite"
						type="text"
						bind:value={inviteCode}
						required
						class="mt-1 w-full rounded border border-border bg-bg-input px-3 py-2 text-text-primary placeholder-text-muted focus:border-accent focus:outline-none"
						placeholder="enter your invite code"
					/>
				</div>
			{/if}

			<button
				type="submit"
				disabled={loading}
				class="w-full rounded bg-accent py-2.5 font-medium text-white transition hover:bg-accent-hover disabled:opacity-50"
			>
				{loading ? '...' : mode === 'login' ? 'Log In' : 'Create Account'}
			</button>
		</form>

		<p class="text-center text-sm text-text-muted">
			{#if mode === 'login'}
				Need an account?
				<button
					onclick={() => (mode = 'register')}
					class="text-text-link hover:underline"
				>
					Register
				</button>
			{:else}
				Already have an account?
				<button
					onclick={() => (mode = 'login')}
					class="text-text-link hover:underline"
				>
					Log In
				</button>
			{/if}
		</p>
	</div>
</div>
