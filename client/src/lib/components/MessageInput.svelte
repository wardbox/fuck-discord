<script lang="ts">
	import { connection } from '$lib/stores/connection.svelte';
	import { channelStore } from '$lib/stores/channels.svelte';
	import { messageStore } from '$lib/stores/messages.svelte';
	import { SendHorizonal, X, Check, Paperclip } from 'lucide-svelte';

	let content = $state('');
	let textarea: HTMLTextAreaElement;
	let fileInput: HTMLInputElement;
	let uploading = $state(false);
	let typingTimeout: ReturnType<typeof setTimeout> | null = null;

	let editing = $derived(messageStore.editingMessage);

	// When entering edit mode, populate the textarea
	$effect(() => {
		if (editing) {
			content = editing.content;
			textarea?.focus();
		}
	});

	function handleSubmit(e: Event) {
		e.preventDefault();
		const trimmed = content.trim();
		if (!trimmed) return;

		if (editing) {
			if (trimmed !== editing.content) {
				connection.editMessage(editing.id, trimmed);
			}
			messageStore.cancelEditing();
			content = '';
			return;
		}

		if (!channelStore.activeChannelId) return;
		connection.sendMessage(channelStore.activeChannelId, trimmed);
		content = '';
	}

	function cancelEdit() {
		messageStore.cancelEditing();
		content = '';
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && editing) {
			e.preventDefault();
			cancelEdit();
			return;
		}

		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSubmit(e);
			return;
		}

		// Send typing indicator (throttled) — only when not editing
		if (!editing && !typingTimeout && channelStore.activeChannelId) {
			connection.sendTyping(channelStore.activeChannelId);
			typingTimeout = setTimeout(() => {
				typingTimeout = null;
			}, 2000);
		}
	}

	async function handleFileSelect() {
		const file = fileInput?.files?.[0];
		if (!file || !channelStore.activeChannelId) return;

		uploading = true;
		try {
			const formData = new FormData();
			formData.append('file', file);

			const res = await fetch('/api/upload', {
				method: 'POST',
				body: formData
			});

			if (res.ok) {
				const data = await res.json();
				// Insert the file URL into the message
				const isImage = file.type.startsWith('image/');
				const markdown = isImage ? `![${data.filename}](${data.url})` : `[${data.filename}](${data.url})`;

				if (content.trim()) {
					content += '\n' + markdown;
				} else {
					content = markdown;
				}
				textarea?.focus();
			}
		} catch (e) {
			console.error('Upload failed:', e);
		} finally {
			uploading = false;
			if (fileInput) fileInput.value = '';
		}
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		const file = e.dataTransfer?.files?.[0];
		if (file && fileInput) {
			const dt = new DataTransfer();
			dt.items.add(file);
			fileInput.files = dt.files;
			handleFileSelect();
		}
	}

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
	}
</script>

<form
	onsubmit={handleSubmit}
	ondrop={handleDrop}
	ondragover={handleDragOver}
	class="border-t border-border px-4 py-3"
>
	{#if editing}
		<div class="mb-1.5 flex items-center gap-2 text-xs text-text-muted">
			<span>Editing message</span>
			<button type="button" onclick={cancelEdit} class="text-text-muted hover:text-text-primary">
				(Esc to cancel)
			</button>
		</div>
	{/if}
	<div class="flex items-end gap-2">
		{#if !editing}
			<input
				bind:this={fileInput}
				type="file"
				accept="image/*,.pdf,.txt,.zip,.tar.gz"
				onchange={handleFileSelect}
				class="hidden"
			/>
			<button
				type="button"
				onclick={() => fileInput?.click()}
				disabled={!connection.connected || uploading}
				class="rounded p-2 text-text-muted hover:bg-white/10 hover:text-text-primary disabled:opacity-30"
				title="Attach file"
			>
				<Paperclip size={18} />
			</button>
		{/if}
		<div class="min-w-0 flex-1">
			<textarea
				bind:this={textarea}
				bind:value={content}
				onkeydown={handleKeydown}
				placeholder={uploading
					? 'Uploading...'
					: editing
						? 'Edit your message...'
						: channelStore.activeChannel
							? `Message #${channelStore.activeChannel.name}`
							: 'Select a channel'}
				rows={1}
				class="w-full resize-none rounded border px-3 py-2 text-sm text-text-primary placeholder-text-muted focus:outline-none {editing ? 'border-accent/50 bg-accent/5' : 'border-border bg-bg-input'} focus:border-accent"
				disabled={!connection.connected || uploading}
			></textarea>
		</div>
		{#if editing}
			<button
				type="button"
				onclick={cancelEdit}
				class="rounded bg-white/5 p-2 text-text-muted hover:bg-white/10 hover:text-text-primary"
				title="Cancel"
			>
				<X size={18} />
			</button>
			<button
				type="submit"
				disabled={!content.trim()}
				class="rounded bg-green-600 p-2 text-white hover:bg-green-500 disabled:opacity-30"
				title="Save"
			>
				<Check size={18} />
			</button>
		{:else}
			<button
				type="submit"
				disabled={!content.trim() || !connection.connected}
				class="rounded bg-accent p-2 text-white hover:bg-accent-hover disabled:opacity-30"
			>
				<SendHorizonal size={18} />
			</button>
		{/if}
	</div>
</form>
