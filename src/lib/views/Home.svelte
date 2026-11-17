<script lang="ts">
	import { app } from '$lib/app.svelte';
	import Shelf from '$lib/components/Shelf.svelte';
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
			<span class="ico">{#if busy === 'random'}<span class="spinner"></span>{:else}<Icon name="shuffle" size={20} />{/if}</span>
			<span><b>Случайный микс</b><small>100 треков из всей библиотеки</small></span>
		</button>
		<button class="mix" onclick={() => mix('fav', () => app.api!.starred().then((s) => s.songs), 'В избранном пока пусто')} disabled={busy !== null}>
			<span class="ico">{#if busy === 'fav'}<span class="spinner"></span>{:else}<Icon name="heart" size={20} />{/if}</span>
			<span><b>Избранное</b><small>Всё отмеченное вперемешку</small></span>
		</button>
	</div>

	{#if genres.value?.length}
		<div class="section-head"><h2>Миксы по жанрам</h2></div>
		<div class="genres">
			{#each genres.value as genre (genre.value)}
				<button class="genre" onclick={() => mix(genre.value, () => app.api!.randomSongs(80, genre.value), 'В этом жанре нет треков')} disabled={busy !== null}>
					{#if busy === genre.value}<span class="spinner"></span>{:else}<Icon name="radio" size={15} />{/if}
					<span class="ellipsis">{genre.value}</span>
				</button>
			{/each}
		</div>
	{/if}

	{#if shelves.loading}
		<div class="empty"><span class="spinner"></span></div>
	{:else}
		{#each SHELVES as shelf, i (shelf.type)}
			{@const albums = shelves.value?.[i] ?? []}
			{#if albums.length}
				<Shelf title={shelf.title} {albums} onrefresh={shelf.type === 'random' ? () => shelves.reload() : undefined} />
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
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 10px;
		max-width: 760px;
		margin-top: 20px;
	}

	.genres {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.genre {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		max-width: 220px;
		height: 38px;
		padding: 0 16px 0 13px;
		border-radius: 999px;
		background: var(--surface);
		border: 1px solid var(--line);
		color: var(--text);
		font-weight: 550;
		transition: background 0.15s;
	}

	.genre :global(svg) {
		color: var(--accent);
	}

	.genre .spinner,
	.ico .spinner {
		width: 15px;
		height: 15px;
	}

	.genre:hover:not(:disabled) {
		background: var(--surface-hover);
	}

	/* Phones: two tall tiles, and genres in one row that scrolls sideways. */
	@container main (max-width: 560px) {
		.mixes .mix {
			flex-direction: column;
			align-items: flex-start;
			justify-content: space-between;
			height: 112px;
			padding: 12px 14px;
		}

		.mix small {
			display: -webkit-box;
			-webkit-line-clamp: 2;
			line-clamp: 2;
			-webkit-box-orient: vertical;
			overflow: hidden;
		}

		.genres {
			flex-wrap: nowrap;
			overflow-x: auto;
			margin: 0 -16px;
			padding: 0 16px 4px;
			scrollbar-width: none;
		}

		.genre {
			flex: none;
		}
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

</style>
