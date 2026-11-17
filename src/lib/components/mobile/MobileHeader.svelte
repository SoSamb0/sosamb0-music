<script lang="ts">
	import { app } from '$lib/app.svelte';
	import Icon from '../Icon.svelte';

	/** True once the page is scrolled: the buttons get a small round backing to stay readable. */
	let { scrolled = false }: { scrolled?: boolean } = $props();

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

<header class="mh" class:scrolled class:overlay={root && !searching}>
	<div class="bar">
		{#if !root}
			<button class="icon-btn float" onclick={() => app.back()} aria-label="Назад"><Icon name="left" size={24} /></button>
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
			<span class="grow"></span>
			<button class="icon-btn float" onclick={() => (app.soundOpen = true)} aria-label="Звук"><Icon name="sliders" size={20} /></button>
		{/if}
	</div>
</header>

<style>
	/* No bar of its own: just the buttons, floating at the top while the page scrolls under them. */
	.mh {
		position: sticky;
		top: 0;
		z-index: 10;
		padding-top: var(--sat, env(safe-area-inset-top, 0px));
	}

	.mh.overlay {
		margin-bottom: -50px;
		pointer-events: none;
	}

	.mh.overlay .bar > * {
		pointer-events: auto;
	}

	.bar {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 50px;
		padding: 0 8px;
	}

	.grow {
		flex: 1;
	}

	.float {
		width: 40px;
		height: 40px;
		color: var(--text);
		transition:
			background 0.2s,
			box-shadow 0.2s;
	}

	.scrolled .float {
		background: rgb(16 16 21 / 0.72);
		backdrop-filter: blur(12px);
		box-shadow: 0 2px 10px rgb(0 0 0 / 0.35);
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
		background: var(--card);
		border: 1px solid var(--line);
		box-shadow: 0 4px 14px rgb(0 0 0 / 0.3);
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
