<script lang="ts">
	import { app } from '$lib/app.svelte';
	import AlbumCard from '$lib/components/AlbumCard.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import { loader } from '$lib/loader.svelte';
	import { player } from '$lib/player.svelte';
	import type { AlbumListType } from '$lib/types';

	const SHELVES: { type: AlbumListType; title: string }[] = [
		{ type: 'newest', title: 'Недавно добавленные' },
		{ type: 'recent', title: 'Вы недавно слушали' },
		{ type: 'frequent', title: 'Слушаете чаще всего' },
		{ type: 'random', title: 'Случайная подборка' }
	];

	const shelves = loader(() => Promise.all(SHELVES.map((s) => app.api!.albumList(s.type, 14).catch(() => []))));
	const genres = loader(() => app.api!.genres().then((list) => list.sort((a, b) => (b.songCount ?? 0) - (a.songCount ?? 0)).slice(0, 8)));

	const hour = new Date().getHours();
	const greeting = hour < 5 ? 'Доброй ночи' : hour < 12 ? 'Доброе утро' : hour < 18 ? 'Добрый день' : 'Добрый вечер';

	let busy = $state<string | null>(null);

	async function mix(key: string, load: () => Promise<import('$lib/types').Song[]>, empty: string) {
		busy = key;
		const songs = await app.attempt(load);
		busy = null;
		if (!songs) return;
		if (songs.length === 0) return app.toast(empty);
		player.play(songs, 0, true);
	}
</script>

<div class="page">
	<h1>{greeting}</h1>

	<div class="mixes">
		<button class="mix primary" onclick={() => mix('random', () => app.api!.randomSongs(100), 'В библиотеке пока нет треков')} disabled={busy !== null}>
			<span class="ico"><Icon name="shuffle" size={20} /></span>
			<span><b>Случайный микс</b><small>100 треков из всей библиотеки</small></span>
		</button>
		<button class="mix" onclick={() => mix('fav', () => app.api!.starred().then((s) => s.songs), 'В избранном пока пусто')} disabled={busy !== null}>
			<span class="ico"><Icon name="heart" size={20} /></span>
			<span><b>Избранное вперемешку</b><small>Всё, что вы отметили</small></span>
		</button>
		{#each genres.value ?? [] as genre (genre.value)}
			<button class="mix" onclick={() => mix(genre.value, () => app.api!.randomSongs(80, genre.value), 'В этом жанре нет треков')} disabled={busy !== null}>
				<span class="ico"><Icon name="radio" size={20} /></span>
				<span class="ellipsis"><b class="ellipsis">{genre.value}</b><small>Микс по жанру</small></span>
			</button>
		{/each}
	</div>

	{#if shelves.loading}
		<div class="empty"><span class="spinner"></span></div>
	{:else}
		{#each SHELVES as shelf, i (shelf.type)}
			{@const albums = shelves.value?.[i] ?? []}
			{#if albums.length}
				<div class="section-head">
					<h2>{shelf.title}</h2>
					{#if shelf.type === 'random'}
						<button class="btn small" onclick={() => shelves.reload()}>Обновить</button>
					{/if}
				</div>
				<div class="shelf">
					{#each albums as album (album.id)}<div class="cell"><AlbumCard {album} /></div>{/each}
				</div>
			{/if}
		{/each}
		{#if shelves.value?.every((s) => s.length === 0)}
			<div class="empty"><p>Библиотека пуста. Добавьте музыку на сервер Navidrome.</p></div>
		{/if}
	{/if}
</div>

<style>
	.mixes {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
		gap: 10px;
		margin-top: 20px;
	}

	.mix {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 64px;
		padding: 0 14px 0 10px;
		border-radius: var(--radius);
		background: var(--surface);
		text-align: left;
		min-width: 0;
		transition:
			background 0.15s,
			transform 0.2s var(--ease);
	}

	.mix:hover:not(:disabled) {
		background: var(--surface-hover);
		transform: translateY(-1px);
	}

	.mix.primary {
		background: linear-gradient(120deg, rgb(var(--accent-rgb) / 0.34), rgb(var(--accent-rgb) / 0.1));
	}

	.ico {
		display: grid;
		place-items: center;
		flex: none;
		width: 44px;
		height: 44px;
		border-radius: 10px;
		background: rgb(var(--accent-rgb) / 0.18);
		color: var(--accent);
	}

	.mix span:last-child {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	small {
		font-size: 12.5px;
		color: var(--muted);
	}

	.shelf {
		display: grid;
		grid-auto-flow: column;
		grid-auto-columns: minmax(150px, 172px);
		gap: 18px;
		overflow-x: auto;
		padding: 6px 4px 14px;
		margin: -6px -4px 0;
		scroll-snap-type: x proximity;
		scrollbar-width: none;
	}

	.shelf::-webkit-scrollbar {
		display: none;
	}

	.cell {
		scroll-snap-align: start;
		min-width: 0;
	}
</style>
