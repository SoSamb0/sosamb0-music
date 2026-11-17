<script lang="ts">
	import { app } from '$lib/app.svelte';
	import Icon from '../Icon.svelte';

	const route = $derived(app.route);
	const searching = $derived(route.name === 'search');
	let input = $state<HTMLInputElement>();

	// The search tab opens with the keyboard ready.
	$effect(() => {
		if (searching && route.name === 'search' && !route.q) input?.focus();
	});
</script>

<header class="mh">
	{#if app.cursor > 0}
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
		<span class="grow"></span>
		<button class="icon-btn" onclick={() => (app.soundOpen = true)} aria-label="Звук"><Icon name="sliders" size={20} /></button>
	{/if}
</header>

<style>
	.mh {
		flex: none;
		display: flex;
		align-items: center;
		gap: 6px;
		min-height: 52px;
		padding: env(safe-area-inset-top) 8px 0;
	}

	.grow {
		flex: 1;
	}

	.search {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 8px;
		height: 42px;
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
