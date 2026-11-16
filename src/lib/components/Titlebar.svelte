<script lang="ts">
	import { app } from '$lib/app.svelte';
	import { inTauri } from '$lib/tauri';
	import { closeWindow, minimize, setMiniMode, toggleMaximize } from '$lib/window';
	import Icon from './Icon.svelte';

	let query = $state('');
	let input = $state<HTMLInputElement>();
	let timer: ReturnType<typeof setTimeout>;

	export function focusSearch() {
		input?.focus();
		input?.select();
	}

	function onInput() {
		clearTimeout(timer);
		timer = setTimeout(() => {
			const q = query.trim();
			if (q) app.go({ name: 'search', q });
		}, 220);
	}

	// Keep the box in sync when navigating back to an earlier search.
	$effect(() => {
		const r = app.route;
		if (r.name === 'search' && document.activeElement !== input) query = r.q;
	});

	async function mini() {
		app.mini = true;
		await setMiniMode(true);
	}
</script>

<header class="titlebar" data-tauri-drag-region>
	<div class="nav">
		<button class="icon-btn" onclick={() => app.back()} disabled={app.cursor === 0 && !app.nowPlaying} aria-label="Назад"><Icon name="left" /></button>
		<button class="icon-btn" onclick={() => app.forward()} disabled={app.cursor >= app.history.length - 1} aria-label="Вперёд"><Icon name="right" /></button>
	</div>

	<label class="search">
		<Icon name="search" size={16} />
		<input
			bind:this={input}
			bind:value={query}
			oninput={onInput}
			onkeydown={(e) => {
				if (e.key === 'Enter' && query.trim()) app.go({ name: 'search', q: query.trim() });
				if (e.key === 'Escape') input?.blur();
				e.stopPropagation();
			}}
			placeholder="Поиск по трекам, альбомам, исполнителям"
			spellcheck="false"
			aria-label="Поиск"
		/>
		{#if query}
			<button class="clear" onclick={() => ((query = ''), input?.focus())} aria-label="Очистить"><Icon name="x" size={14} /></button>
		{:else}
			<kbd>Ctrl K</kbd>
		{/if}
	</label>

	<div class="window">
		{#if inTauri}
			<button class="icon-btn" onclick={mini} aria-label="Мини-плеер" title="Мини-плеер"><Icon name="mini" size={16} /></button>
			<span class="gap"></span>
			<button class="wb" onclick={minimize} aria-label="Свернуть"><Icon name="min" size={15} /></button>
			<button class="wb" onclick={toggleMaximize} aria-label="Развернуть"><Icon name="max" size={13} /></button>
			<button class="wb close" onclick={closeWindow} aria-label="Закрыть"><Icon name="x" size={16} /></button>
		{/if}
	</div>
</header>

<style>
	.titlebar {
		display: grid;
		grid-template-columns: 1fr minmax(240px, 460px) 1fr;
		align-items: center;
		gap: 16px;
		height: 52px;
		padding: 0 0 0 14px;
	}

	.nav {
		display: flex;
		gap: 2px;
	}

	.search {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 36px;
		padding: 0 12px;
		border-radius: 999px;
		background: var(--surface);
		border: 1px solid transparent;
		color: var(--muted);
		transition:
			background 0.15s,
			border-color 0.15s;
	}

	.search:focus-within {
		background: rgb(0 0 0 / 0.35);
		border-color: rgb(var(--accent-rgb) / 0.6);
		color: var(--text);
	}

	.search input {
		flex: 1;
		min-width: 0;
		background: none;
		border: 0;
		outline: 0;
		user-select: text;
	}

	.search input::placeholder {
		color: var(--faint);
	}

	kbd {
		font: 600 10.5px/1 inherit;
		padding: 4px 6px;
		border-radius: 5px;
		background: var(--surface-hover);
		color: var(--faint);
		white-space: nowrap;
	}

	.clear {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		color: var(--muted);
	}

	.clear:hover {
		background: var(--surface-hover);
		color: var(--text);
	}

	.window {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		height: 100%;
	}

	.gap {
		width: 8px;
	}

	.wb {
		display: grid;
		place-items: center;
		width: 46px;
		height: 100%;
		color: var(--muted);
		transition:
			background 0.12s,
			color 0.12s;
	}

	.wb:hover {
		background: var(--surface-hover);
		color: var(--text);
	}

	.wb.close:hover {
		background: #e5484d;
		color: #fff;
	}
</style>
