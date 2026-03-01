<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { initConfig, isTauri, isServerConfigured, setServerUrl, clearServerUrl } from '$lib/config';
	import { auth } from '$lib/stores/auth.svelte';

	let { children } = $props();

	let configReady = $state(false);
	let needsServerUrl = $state(false);
	let serverUrlInput = $state('');
	let serverUrlError = $state('');
	let connecting = $state(false);

	onMount(async () => {
		try {
			await initConfig();
			needsServerUrl = isTauri() && !isServerConfigured();
		} catch (e) {
			console.error('Failed to initialize config:', e);
			// Show server URL prompt as recovery when config fails in Tauri
			if (isTauri()) needsServerUrl = true;
		} finally {
			configReady = true;
		}
	});

	async function connectToServer() {
		if (!serverUrlInput.trim()) return;
		connecting = true;
		serverUrlError = '';

		// Normalize URL
		let url = serverUrlInput.trim();
		if (!url.startsWith('http://') && !url.startsWith('https://')) {
			url = 'https://' + url;
		}
		url = url.replace(/\/+$/, '');

		// Only allow http:// for local addresses
		try {
			const parsed = new URL(url);
			const host = parsed.hostname;
			if (parsed.protocol === 'http:' && host !== 'localhost' && host !== '127.0.0.1' && host !== '::1') {
				url = url.replace(/^http:\/\//, 'https://');
			}
		} catch {
			serverUrlError = 'Invalid URL format.';
			connecting = false;
			return;
		}

		try {
			// Validate: hit the server's auth endpoint. 200 or 401 = reachable Relay server.
			const res = await fetch(`${url}/api/auth/me`, {
				method: 'GET',
				signal: AbortSignal.timeout(10000)
			});
			if (res.status !== 200 && res.status !== 401) {
				throw new Error(`Unexpected status: ${res.status}`);
			}
			await setServerUrl(url);
			needsServerUrl = false;
		} catch {
			serverUrlError = 'Could not connect to server. Check the URL and try again.';
		} finally {
			connecting = false;
		}
	}

	async function changeServer() {
		try {
			await clearServerUrl();
			needsServerUrl = true;
			serverUrlInput = '';
			serverUrlError = '';
		} catch (e) {
			console.error('Failed to clear server URL:', e);
		}
	}
</script>

{#if !configReady}
	<div class="flex h-screen items-center justify-center bg-bg-primary">
		<div class="text-text-muted">Loading...</div>
	</div>
{:else if needsServerUrl}
	<div class="flex min-h-screen items-center justify-center bg-bg-primary">
		<div class="w-full max-w-md space-y-8 p-8">
			<div class="text-center">
				<h1 class="text-4xl font-bold text-text-primary">Relay</h1>
				<p class="mt-2 text-sm text-text-secondary">
					Enter your Relay server URL to get started.
				</p>
			</div>

			<form onsubmit={(e) => { e.preventDefault(); connectToServer(); }} class="space-y-4">
				{#if serverUrlError}
					<div role="alert" class="rounded bg-danger/10 px-4 py-2 text-sm text-danger">{serverUrlError}</div>
				{/if}

				<div>
					<label for="server-url" class="block text-sm text-text-secondary">Server URL</label>
					<input
						id="server-url"
						type="text"
						bind:value={serverUrlInput}
						placeholder="https://relay.example.com"
						class="mt-1 w-full rounded border border-border bg-bg-input px-3 py-2 text-text-primary placeholder-text-muted focus:border-accent focus:outline-none"
						disabled={connecting}
					/>
				</div>

				<button
					type="submit"
					disabled={connecting || !serverUrlInput.trim()}
					class="w-full rounded bg-accent py-2.5 font-medium text-white transition hover:bg-accent-hover disabled:opacity-50 disabled:cursor-not-allowed"
				>
					{connecting ? 'Connecting...' : 'Connect'}
				</button>
			</form>
		</div>
	</div>
{:else}
	{@render children()}

	{#if isTauri() && !auth.isAuthenticated}
		<div class="fixed bottom-4 right-4">
			<button
				onclick={changeServer}
				class="text-xs text-text-muted hover:text-text-secondary transition-colors"
			>
				Change Server
			</button>
		</div>
	{/if}
{/if}
