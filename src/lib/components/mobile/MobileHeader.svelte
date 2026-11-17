<script lang="ts">
	import { app } from '$lib/app.svelte';
	import Icon from '../Icon.svelte';

	/** True once the page under the header is scrolled: the header turns frosted and shows a title. */
	let { scrolled = false, title = '' }: { scrolled?: boolean; title?: string } = $props();

	const ROOTS = ['home', 'search', 'library', 'playlists'];
	const route = $derived(app.route);
	const root = $derived(ROOTS.includes(route.name));
	const searching = $derived(route.name === 'search');
	let input = $state<HTMLInputElement>();

	// The search tab opens with the keyboard ready.
	$effect(() => {
		if (searching && route.name === 'search' && !route.q) input?.focus();
	});
</script>

<header class="mh" class:scrolled={scrolled || searching} class:overlay={root && !searching}>
	<div class="bar">
		{#if !root}
			<button class="icon-btn" onclick={() => app.back()} aria-label="Назад"><Icon name="left" size={24} /></button>
		{/if}
		{#if searching && route.name === 'search'}
			<label class="search">
				<Icon name="search" size={18} />
				<input
					bind:this={input}
					type="search"
					enterkeyhint="search"
					placeholder="Треки, альбомы, исполнители"
					value={route.q}
					oninput={(e) => app.go({ name: 'search', q: e.currentTarget.value })}
				/>
				{#if route.q}
					<button class="icon-btn clear" onclick={() => app.go({ name: 'search', q: '' })} aria-label="Очистить"><Icon name="x" size={16} /></button>
				{/if}
			</label>
		{:else}
			<span class="title ellipsis">{scrolled ? title : ''}</span>
			<button class="icon-btn" onclick={() => (app.soundOpen = true)} aria-label="Звук"><Icon name="sliders" size={20} /></button>
		{/if}
	</div>
</header>

<style>
	.mh {
		position: sticky;
		top: 0;
		z-index: 10;
		padding-top: var(--sat, env(safe-area-inset-top, 0px));
		border-bottom: 1px solid transparent;
		transition:
			background 0.2s,
			border-color 0.2s;
	}

	/* Root tabs: the buttons share the line with the page's big title. */
	.mh.overlay {
		margin-bottom: -50px;
		pointer-events: none;
	}

	.mh.overlay .bar > * {
		pointer-events: auto;
	}

	.mh.scrolled {
		margin-bottom: 0;
		background: rgb(14 14 18 / 0.82);
		backdrop-filter: blur(22px) saturate(1.5);
		border-bottom-color: var(--line);
	}

	.mh.overlay.scrolled {
		margin-bottom: -50px;
	}

	.bar {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 50px;
		padding: 0 6px 0 8px;
	}

	.title {
		flex: 1;
		min-width: 0;
		padding-left: 8px;
		font-size: 16px;
		font-weight: 650;
		animation: fade 0.2s;
	}

	@keyframes fade {
		from {
			opacity: 0;
		}
	}

	.search {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 8px;
		height: 42px;
		margin: 0 4px;
		padding: 0 6px 0 14px;
		border-radius: 999px;
		background: var(--surface-strong);
		color: var(--muted);
	}

	.search input {
		flex: 1;
		min-width: 0;
		height: 100%;
		border: 0;
		outline: 0;
		background: none;
		color: var(--text);
		font-size: 16px;
	}

	.search input::-webkit-search-cancel-button {
		display: none;
	}

	.clear {
		width: 32px;
		height: 32px;
	}
</style>
