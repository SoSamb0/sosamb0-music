<script lang="ts">
	import { app } from '$lib/app.svelte';
	import Cover from '$lib/components/Cover.svelte';
	import HomeEditor from '$lib/components/HomeEditor.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import Shelf from '$lib/components/Shelf.svelte';
	import { SMART, type SmartKind } from '$lib/library.svelte';
	import { loader } from '$lib/loader.svelte';
	import { menu } from '$lib/menu.svelte';
	import { player } from '$lib/player.svelte';
	import { settings, type HomeSection } from '$lib/settings.svelte';
	import type { AlbumListType, Song } from '$lib/types';

	const SHELVES: Partial<Record<HomeSection, { type: AlbumListType; title: string }>> = {
		newest: { type: 'newest', title: 'Недавно добавленные' },
		recent: { type: 'recent', title: 'Вы недавно слушали' },
		frequent: { type: 'frequent', title: 'Слушаете чаще всего' },
		random: { type: 'random', title: 'Случайная подборка' }
	};

	const home = $derived(settings.data.home);
	const sections = $derived(home.order.filter((s) => !home.hidden.includes(s)));

	const shelves = loader(async () => {
		const entries = await Promise.all(
			Object.entries(SHELVES).map(async ([key, s]) => [key, await app.api!.albumList(s!.type, 14).catch(() => [])] as const)
		);
		return Object.fromEntries(entries);
	});
	const genres = loader(() => app.api!.genres().then((list) => list.sort((a, b) => (b.songCount ?? 0) - (a.songCount ?? 0)).slice(0, 10)));

	/** Pinned tiles, resolved against the current playlists (deleted ones disappear). */
	const pinned = $derived(
		home.pinned.flatMap((id) => {
			if (id.startsWith('smart:')) {
				const kind = id.slice(6) as SmartKind;
				return SMART[kind] ? [{ id, title: SMART[kind].title, icon: SMART[kind].icon, cover: undefined as string | undefined, open: () => app.go({ name: 'smart', id: kind }) }] : [];
			}
			const p = app.playlists.find((p) => p.id === id);
			return p ? [{ id, title: p.name, icon: 'playlist', cover: p.coverArt, open: () => app.go({ name: 'playlist', id: p.id }) }] : [];
		})
	);

	const hour = new Date().getHours();
	const greeting = hour < 5 ? 'Доброй ночи' : hour < 12 ? 'Доброе утро' : hour < 18 ? 'Добрый день' : 'Добрый вечер';

	let busy = $state<string | null>(null);
	let editing = $state(false);

	async function mix(key: string, load: () => Promise<Song[]>, empty: string) {
		busy = key;
		const songs = await app.attempt(load);
		busy = null;
		if (!songs) return;
		if (songs.length === 0) return app.toast(empty);
		player.play(songs, 0, true);
	}

	function pinMenu(e: MouseEvent, id: string, open: () => void) {
		menu.open(e, [
			{ label: 'Открыть', icon: 'playlist', action: open },
			{ label: 'Открепить с главной', icon: 'x', action: () => settings.togglePin(id) }
		]);
	}
</script>

<div class="page">
	<h1>{greeting}</h1>

	{#each sections as section (section)}
		{#if section === 'pinned' && pinned.length}
			<div class="pins">
				{#each pinned as p (p.id)}
					<button class="pin" onclick={p.open} oncontextmenu={(e) => pinMenu(e, p.id, p.open)}>
						<span class="pin-art">
							{#if p.cover}<Cover id={p.cover} size={120} icon="playlist" />{:else}<Icon name={p.icon} size={20} />{/if}
						</span>
						<span class="ellipsis">{p.title}</span>
					</button>
				{/each}
			</div>
		{:else if section === 'mixes'}
			<div class="mixes">
				<button class="mix primary" onclick={() => mix('random', () => app.api!.randomSongs(100), 'В библиотеке пока нет треков')} disabled={busy !== null}>
					<span class="ico">{#if busy === 'random'}<span class="spinner"></span>{:else}<Icon name="shuffle" size={20} />{/if}</span>
					<span class="text"><b>Случайный микс</b><small>100 треков</small></span>
				</button>
				<button class="mix" onclick={() => mix('fav', () => app.api!.starred().then((s) => s.songs), 'В избранном пока пусто')} disabled={busy !== null}>
					<span class="ico">{#if busy === 'fav'}<span class="spinner"></span>{:else}<Icon name="heart" size={20} />{/if}</span>
					<span class="text"><b>Избранное</b><small>Вперемешку</small></span>
				</button>
			</div>
		{:else if section === 'genres' && genres.value?.length}
			<div class="section-head"><h2>Миксы по жанрам</h2></div>
			<div class="chips">
				{#each genres.value as genre (genre.value)}
					<button class="chip-btn" onclick={() => mix(genre.value, () => app.api!.randomSongs(80, genre.value), 'В этом жанре нет треков')} disabled={busy !== null}>
						{#if busy === genre.value}<span class="spinner"></span>{:else}<Icon name="radio" size={15} />{/if}
						<span class="ellipsis">{genre.value}</span>
					</button>
				{/each}
			</div>
		{:else if section === 'playlists' && app.playlists.length}
			<div class="section-head"><h2>Ваши плейлисты</h2></div>
			<div class="row">
				{#each app.playlists as p (p.id)}
					<button class="card" onclick={() => app.go({ name: 'playlist', id: p.id })}>
						<span class="card-art"><Cover id={p.coverArt} size={300} icon="playlist" /></span>
						<span class="card-title ellipsis">{p.name}</span>
						{#if p.songCount !== undefined}<span class="card-sub">{p.songCount} тр.</span>{/if}
					</button>
				{/each}
			</div>
		{:else if section === 'smart'}
			<div class="section-head"><h2>Умные плейлисты</h2></div>
			<div class="smart">
				{#each Object.entries(SMART) as [id, s] (id)}
					<button class="smart-tile" onclick={() => app.go({ name: 'smart', id: id as SmartKind })}>
						<span class="ico"><Icon name={s.icon} size={18} /></span>
						<span class="ellipsis">{s.title}</span>
					</button>
				{/each}
			</div>
		{:else if SHELVES[section]}
			{@const albums = shelves.value?.[section] ?? []}
			{#if albums.length}
				<Shelf title={SHELVES[section]!.title} {albums} onrefresh={section === 'random' ? () => shelves.reload() : undefined} />
			{/if}
		{/if}
	{/each}

	{#if shelves.loading}
		<div class="empty"><span class="spinner"></span></div>
	{/if}

	<button class="customize" onclick={() => (editing = true)}><Icon name="sliders" size={16} />Настроить главную</button>
</div>

{#if editing}<HomeEditor onclose={() => (editing = false)} />{/if}

<style>
	h1 {
		margin-bottom: 18px;
	}

	/* Quick access: two columns of compact tiles. */
	.pins {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(100%, 220px), 1fr));
		gap: 8px;
		margin-bottom: 14px;
	}

	@container main (max-width: 560px) {
		.pins {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	.pin {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 56px;
		padding-right: 10px;
		border-radius: 10px;
		overflow: hidden;
		background: var(--card);
		border: 1px solid var(--line);
		font-weight: 600;
		font-size: 14px;
		text-align: left;
		min-width: 0;
	}

	.pin-art {
		flex: none;
		display: grid;
		place-items: center;
		width: 56px;
		height: 56px;
		background: var(--accent-soft);
		color: var(--accent);
	}

	.mixes {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 10px;
		max-width: 760px;
	}

	.mix {
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 68px;
		padding: 12px 14px 12px 12px;
		border-radius: var(--radius);
		background: var(--card);
		border: 1px solid var(--line);
		text-align: left;
		min-width: 0;
		transition:
			background 0.15s,
			transform 0.2s var(--ease);
	}

	.mix:hover:not(:disabled) {
		background: var(--card-hover);
	}

	.mix:active:not(:disabled) {
		transform: scale(0.98);
	}

	.mix.primary {
		background: linear-gradient(135deg, rgb(var(--accent-rgb) / 0.32), rgb(var(--accent-rgb) / 0.12)), var(--card);
		border-color: rgb(var(--accent-rgb) / 0.3);
	}

	.ico {
		display: grid;
		place-items: center;
		flex: none;
		width: 40px;
		height: 40px;
		border-radius: 10px;
		background: var(--accent);
		color: var(--on-accent);
	}

	.mix:not(.primary) .ico {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.ico .spinner,
	.chip-btn .spinner {
		width: 15px;
		height: 15px;
	}

	.text {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	small {
		font-size: 12.5px;
		color: var(--muted);
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.chip-btn {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		flex: none;
		max-width: 220px;
		height: 38px;
		padding: 0 16px 0 13px;
		border-radius: 999px;
		background: var(--card);
		border: 1px solid var(--line);
		font-weight: 550;
		transition: background 0.15s;
	}

	.chip-btn :global(svg) {
		color: var(--accent);
	}

	.chip-btn:hover:not(:disabled) {
		background: var(--card-hover);
	}

	.row {
		display: grid;
		grid-auto-flow: column;
		grid-auto-columns: minmax(130px, 150px);
		gap: 14px;
		overflow-x: auto;
		padding-bottom: 6px;
		scrollbar-width: none;
	}

	.card {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 0;
		text-align: left;
	}

	.card-art {
		display: block;
		margin-bottom: 4px;
		border-radius: 10px;
		overflow: hidden;
		box-shadow: 0 6px 18px rgb(0 0 0 / 0.35);
	}

	.card-title {
		font-weight: 600;
	}

	.card-sub {
		font-size: 12.5px;
		color: var(--muted);
	}

	.smart {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 8px;
	}

	.smart-tile {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 52px;
		padding: 0 12px 0 8px;
		border-radius: 10px;
		background: var(--card);
		border: 1px solid var(--line);
		font-weight: 600;
		text-align: left;
		min-width: 0;
	}

	.smart-tile .ico {
		width: 36px;
		height: 36px;
		background: var(--accent-soft);
		color: var(--accent);
	}

	.customize {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 30px auto 0;
		height: 40px;
		padding: 0 18px;
		border-radius: 999px;
		background: var(--card);
		border: 1px solid var(--line);
		color: var(--muted);
		font-weight: 550;
	}

	/* Phones: genres in one row that scrolls sideways; edge to edge rows. */
	@container main (max-width: 560px) {
		.chips,
		.row {
			flex-wrap: nowrap;
			overflow-x: auto;
			margin: 0 -16px;
			padding: 0 16px 6px;
			scrollbar-width: none;
		}

		.row {
			grid-auto-columns: 132px;
			gap: 12px;
		}

		.smart {
			grid-template-columns: 1fr;
		}
	}
</style>
