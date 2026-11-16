<script lang="ts">
	import { formatDuration, formatTime, plural } from '$lib/app.svelte';
	import { menu, songMenu } from '$lib/menu.svelte';
	import { player } from '$lib/player.svelte';
	import { settings } from '$lib/settings.svelte';
	import Cover from './Cover.svelte';
	import Icon from './Icon.svelte';

	let dragFrom = $state<number | null>(null);
	let dragOver = $state<number | null>(null);
	let list = $state<HTMLElement>();

	const upcoming = $derived(player.queue.length - player.index - 1);
	const cachedCount = $derived(player.queue.filter((q) => player.cached.has(player.cacheKey(q.song))).length);
	const remaining = $derived(player.queue.slice(player.index + 1).reduce((sum, q) => sum + (q.song.duration ?? 0), 0));

	// Open on the track that is playing.
	$effect(() => {
		const i = player.index;
		queueMicrotask(() => list?.querySelector(`[data-i="${i}"]`)?.scrollIntoView({ block: 'nearest' }));
	});

	function drop(to: number) {
		if (dragFrom !== null) player.move(dragFrom, to);
		dragFrom = dragOver = null;
	}
</script>

<div class="panel">
	<header>
		<div>
			<h2>Очередь</h2>
			<p class="muted small">
				{#if upcoming > 0}{plural(upcoming, 'трек', 'трека', 'треков')} впереди · {formatDuration(remaining)}{:else}Дальше пусто{/if}
			</p>
			{#if player.queue.length && !settings.data.demo}
				<p class="muted small cache" title="Скачанные треки переключаются мгновенно и играют без сети">
					<Icon name="download" size={13} />
					В кэше {cachedCount} из {player.queue.length}{#if player.downloading.size} · качается…{/if}{#if player.cacheLimited} · упёрлись в предел кэша{/if}
				</p>
			{/if}
		</div>
		<button class="btn small" onclick={() => player.clearUpcoming()} disabled={upcoming <= 0}>Очистить</button>
	</header>

	{#if player.queue.length === 0}
		<div class="empty">
			<Icon name="queue" size={30} />
			<p>Очередь пуста. Включите альбом или плейлист.</p>
		</div>
	{:else}
		<div class="list" bind:this={list} role="list">
			{#each player.queue as item, i (item.token)}
				{@const key = player.cacheKey(item.song)}
				<div
					class="row"
					class:current={i === player.index}
					class:played={i < player.index}
					class:over={dragOver === i && dragFrom !== i}
					data-i={i}
					role="listitem"
					draggable="true"
					ondragstart={(e) => {
						dragFrom = i;
						e.dataTransfer!.effectAllowed = 'move';
					}}
					ondragover={(e) => {
						e.preventDefault();
						dragOver = i;
					}}
					ondrop={(e) => (e.preventDefault(), drop(i))}
					ondragend={() => (dragFrom = dragOver = null)}
					ondblclick={() => player.jump(i)}
					oncontextmenu={(e) =>
						menu.open(e, songMenu(item.song, i === player.index ? [] : [{ label: '', separator: true }, { label: 'Убрать из очереди', icon: 'x', action: () => player.remove(i) }]))}
				>
					<button class="thumb" onclick={() => (i === player.index ? player.toggle() : player.jump(i))} aria-label="Играть {item.song.title}">
						<Cover id={item.song.coverArt} size={80} />
						<span class="over-icon"><Icon name={i === player.index && player.playing ? 'pause' : 'play'} size={15} /></span>
					</button>
					<div class="text">
						<div class="title ellipsis">{item.song.title}</div>
						<div class="artist ellipsis">{item.song.artist ?? ''}</div>
					</div>
					<span class="cache-state">
						{#if player.cached.has(key)}
							<span class="ok" title="Скачан — играет без сети"><Icon name="saved" size={14} /></span>
						{:else if player.downloading.has(key)}
							<span class="spinner tiny" title="Скачивается"></span>
						{/if}
					</span>
					<span class="dur tnum">{formatTime(item.song.duration)}</span>
					{#if i !== player.index}
						<button class="icon-btn remove" onclick={() => player.remove(i)} aria-label="Убрать из очереди"><Icon name="x" size={15} /></button>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 18px 16px 12px 20px;
	}

	.small {
		font-size: 12.5px;
	}

	.cache {
		display: flex;
		align-items: center;
		gap: 5px;
		margin-top: 2px;
	}

	.cache-state {
		display: grid;
		place-items: center;
	}

	.ok {
		color: rgb(var(--accent-rgb) / 0.75);
		display: grid;
	}

	.spinner.tiny {
		width: 12px;
		height: 12px;
		border-width: 2px;
	}

	.list {
		flex: 1;
		overflow-y: auto;
		padding: 0 8px 12px;
	}

	.row {
		position: relative;
		display: grid;
		grid-template-columns: 40px minmax(0, 1fr) 16px auto 28px;
		align-items: center;
		gap: 10px;
		height: 52px;
		padding: 0 6px;
		border-radius: var(--radius-sm);
		content-visibility: auto;
		contain-intrinsic-size: auto 52px;
	}

	.row:hover {
		background: var(--surface);
	}

	.row.current {
		background: var(--accent-soft);
	}

	.row.played {
		opacity: 0.5;
	}

	.row.over::before {
		content: '';
		position: absolute;
		inset: -1px 6px auto;
		height: 2px;
		border-radius: 2px;
		background: var(--accent);
	}

	.thumb {
		position: relative;
		width: 40px;
		border-radius: 6px;
		overflow: hidden;
	}

	.over-icon {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		background: rgb(0 0 0 / 0.55);
		opacity: 0;
	}

	.row:hover .over-icon {
		opacity: 1;
	}

	.text {
		min-width: 0;
	}

	.title {
		font-weight: 550;
	}

	.current .title {
		color: var(--accent);
	}

	.artist {
		font-size: 12.5px;
		color: var(--muted);
	}

	.dur {
		font-size: 12.5px;
		color: var(--muted);
	}

	.remove {
		width: 28px;
		height: 28px;
		opacity: 0;
	}

	.row:hover .remove,
	.remove:focus-visible {
		opacity: 1;
	}
</style>
