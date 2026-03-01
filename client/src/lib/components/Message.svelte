<script lang="ts">
	import type { Message as MessageType } from '$lib/protocol/types';
	import { renderMarkdown } from '$lib/utils/markdown';
	import { formatCompactTimestamp } from '$lib/utils/time';
	import { auth } from '$lib/stores/auth.svelte';
	import { connection } from '$lib/stores/connection.svelte';
	import { messageStore } from '$lib/stores/messages.svelte';
	import { Pencil, Trash2, SmilePlus } from 'lucide-svelte';

	const QUICK_EMOJIS = ['👍', '❤️', '😂', '🎉', '🔥', '👀', '💯', '✅'];

	interface Props {
		message: MessageType;
		compact?: boolean;
	}

	let { message, compact = false }: Props = $props();

	let renderedContent = $derived(renderMarkdown(message.content));
	let isOwn = $derived(auth.user?.id === message.author_id);
	let confirmingDelete = $state(false);
	let deleteTimerId: ReturnType<typeof setTimeout> | undefined;
	let showEmojiPicker = $state(false);

	$effect(() => {
		return () => {
			if (deleteTimerId) {
				clearTimeout(deleteTimerId);
				deleteTimerId = undefined;
			}
		};
	});
	let emojiPickerEl: HTMLDivElement | undefined = $state();
	let reactions = $derived(message.reactions ?? []);

	$effect(() => {
		if (!showEmojiPicker) return;
		function onClick(e: MouseEvent) {
			if (emojiPickerEl && !emojiPickerEl.contains(e.target as Node)) {
				showEmojiPicker = false;
			}
		}
		// Defer to avoid the same click that opened it from closing it
		const id = setTimeout(() => {
			document.addEventListener('click', onClick);
			// Focus first emoji button when picker opens
			const first = emojiPickerEl?.querySelector<HTMLButtonElement>('[role="menuitem"]');
			first?.focus();
		}, 0);
		return () => {
			clearTimeout(id);
			document.removeEventListener('click', onClick);
		};
	});

	function handlePickerKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			showEmojiPicker = false;
			return;
		}
		if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
			e.preventDefault();
			const items = emojiPickerEl?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]');
			if (!items) return;
			const idx = Array.from(items).indexOf(document.activeElement as HTMLButtonElement);
			items[(idx + 1) % items.length]?.focus();
		}
		if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
			e.preventDefault();
			const items = emojiPickerEl?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]');
			if (!items || items.length === 0) return;
			const idx = Array.from(items).indexOf(document.activeElement as HTMLButtonElement);
			const next = idx < 0 ? items.length - 1 : (idx - 1 + items.length) % items.length;
			items[next]?.focus();
		}
	}

	function startEdit() {
		messageStore.startEditing(message);
	}

	function handleDelete() {
		if (!confirmingDelete) {
			confirmingDelete = true;
			clearTimeout(deleteTimerId);
			deleteTimerId = setTimeout(() => {
				confirmingDelete = false;
				deleteTimerId = undefined;
			}, 3000);
			return;
		}
		clearTimeout(deleteTimerId);
		deleteTimerId = undefined;
		connection.deleteMessage(message.id);
		confirmingDelete = false;
	}

	function toggleReaction(emoji: string) {
		const existing = reactions.find((r) => r.emoji === emoji);
		if (existing && auth.user && existing.users.includes(auth.user.id)) {
			connection.removeReaction(message.id, emoji);
		} else {
			connection.addReaction(message.id, emoji);
		}
		showEmojiPicker = false;
	}

	function hasUserReacted(emoji: string): boolean {
		const r = reactions.find((r) => r.emoji === emoji);
		return r ? auth.user != null && r.users.includes(auth.user.id) : false;
	}
</script>

{#snippet actionButtons(topClass: string)}
	<div class="absolute right-2 {topClass} flex gap-0.5 opacity-0 pointer-events-none group-hover:opacity-100 group-hover:pointer-events-auto focus-within:opacity-100 focus-within:pointer-events-auto">
		<button type="button" onclick={() => { showEmojiPicker = !showEmojiPicker; }} class="rounded p-1 text-text-muted hover:bg-white/10 hover:text-text-primary" title="Add reaction" aria-label="Add reaction">
			<SmilePlus size={13} />
		</button>
		{#if isOwn}
			<button type="button" onclick={startEdit} class="rounded p-1 text-text-muted hover:bg-white/10 hover:text-text-primary" title="Edit" aria-label="Edit message">
				<Pencil size={13} />
			</button>
			<button type="button" onclick={handleDelete} class="rounded p-1 hover:bg-white/10 {confirmingDelete ? 'text-red-400' : 'text-text-muted hover:text-text-primary'}" title={confirmingDelete ? 'Click again to delete' : 'Delete'} aria-label={confirmingDelete ? 'Confirm delete' : 'Delete message'}>
				<Trash2 size={13} />
			</button>
		{/if}
	</div>
	{#if showEmojiPicker}
		<div
			bind:this={emojiPickerEl}
			role="menu"
			tabindex="-1"
			onkeydown={handlePickerKeydown}
			class="absolute right-2 {topClass === 'top-0' ? 'top-6' : 'top-8'} z-10 flex gap-0.5 rounded border border-border bg-bg-secondary p-1 shadow-lg"
		>
			{#each QUICK_EMOJIS as emoji}
				<button role="menuitem" onclick={() => toggleReaction(emoji)} class="rounded p-1 text-sm hover:bg-white/10">
					{emoji}
				</button>
			{/each}
		</div>
	{/if}
{/snippet}

{#snippet reactionDisplay()}
	{#if reactions.length > 0}
		<div class="ml-[4.5rem] flex flex-wrap gap-1 pb-0.5">
			{#each reactions as reaction}
				<!-- TODO: reaction.users contains user IDs, not display names.
					 Resolving IDs to usernames requires a lookup against the member store.
					 This is a known limitation — tracked for a future PR. -->
				<button
					onclick={() => toggleReaction(reaction.emoji)}
					class="flex items-center gap-1 rounded-full border px-1.5 py-0.5 text-xs
						{hasUserReacted(reaction.emoji)
							? 'border-accent/50 bg-accent/10 text-text-primary'
							: 'border-border bg-white/5 text-text-muted hover:border-border/80'}"
					title={`${reaction.count} ${reaction.count === 1 ? 'person' : 'people'} reacted`}
				>
					<span>{reaction.emoji}</span>
					<span>{reaction.count}</span>
				</button>
			{/each}
		</div>
	{/if}
{/snippet}

{#if compact}
	<div class="group relative flex items-start gap-0 py-0.5 hover:bg-bg-message-hover">
		<span class="w-[4.5rem] shrink-0 text-right text-xs text-transparent group-hover:text-text-muted font-mono pr-2 pt-0.5">
			{formatCompactTimestamp(message.created_at)}
		</span>
		<div class="min-w-0 flex-1 text-sm text-text-primary message-content">
			{@html renderedContent}
		</div>
		{@render actionButtons('top-0')}
	</div>
	{@render reactionDisplay()}
{:else}
	<div class="group relative flex items-start gap-0 pt-2 pb-0.5 hover:bg-bg-message-hover">
		<span class="w-[4.5rem] shrink-0 text-right text-xs text-text-muted font-mono pr-2 pt-0.5">
			{formatCompactTimestamp(message.created_at)}
		</span>
		<div class="min-w-0 flex-1">
			<span class="text-sm font-medium text-accent">{message.author_username}</span>
			{#if message.edited_at}
				<span class="text-xs text-text-muted">(edited)</span>
			{/if}
			<div class="text-sm text-text-primary message-content">
				{@html renderedContent}
			</div>
		</div>
		{@render actionButtons('top-2')}
	</div>
	{@render reactionDisplay()}
{/if}

<style>
	:global(.message-content p) {
		margin: 0;
	}

	:global(.message-content a) {
		color: var(--color-text-link);
		text-decoration: underline;
	}

	:global(.message-content code) {
		background: rgba(255, 255, 255, 0.06);
		padding: 0.1em 0.3em;
		border-radius: 3px;
		font-family: var(--font-mono);
		font-size: 0.85em;
	}

	:global(.message-content pre) {
		background: rgba(0, 0, 0, 0.3);
		padding: 0.75em 1em;
		border-radius: 4px;
		overflow-x: auto;
		margin: 0.25em 0;
	}

	:global(.message-content pre code) {
		background: none;
		padding: 0;
	}

	:global(.message-content blockquote) {
		border-left: 3px solid var(--color-border);
		padding-left: 0.75em;
		margin: 0.25em 0;
		color: var(--color-text-secondary);
	}

	:global(.message-content img) {
		max-width: 400px;
		max-height: 300px;
		border-radius: 4px;
		margin: 0.25em 0;
		cursor: pointer;
	}
</style>
