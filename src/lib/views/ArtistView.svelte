<script lang="ts">
	import { app, plural } from '$lib/app.svelte';
	import AlbumCard from '$lib/components/AlbumCard.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SongList from '$lib/components/SongList.svelte';
	import { loader } from '$lib/loader.svelte';
	import { player } from '$lib/player.svelte';
	import type { Song } from '$lib/types';

	let { id }: { id: string } = $props();

	const artist = loader(() => app.api!.artist(id));
	const top = loader(async () => {
		const a = await app.api!.artist(id);
		return app.api!.topSongs(a.name, 10).catch(() => [] as Song[]);
	});
	const albums = $derived([...(artist.value?.album ?? [])].sort((a, b) => (b.year ?? 0) - (a.year ?? 0)));
	let showAllTop = $state(false);
	let busy = $state(false);

	async function allSongs(): Promise<Song[]> {
		const full = await Promise.all(albums.map((a) => app.api!.album(a.id)));
		return full.flatMap((a) => a.song ?? []);
	}

	async function play(shuffled: boolean) {
		busy = true;
		const songs = await app.attempt(allSongs);
		busy = false;
		if (songs?.length) player.play(songs, 0, shuffled);
	}

	async function radio() {
		const seed = top.value?.[0] ?? (await app.attempt(allSongs))?.[0];
		if (seed) player.radio(seed);
	}
</script>

<div class="page">
	{#if artist.value}
		{@const a = artist.value}
		<Hero cover={a.coverArt} kind="Исполнитель" title={a.name} round icon="artist">
			{#snippet meta()}<span>{plural(albums.length, 'альбом', 'альбома', 'альбомов')}</span>{/snippet}
			{#snippet actions()}
				<button class="btn primary" onclick={() => play(false)} disabled={busy || !albums.length}><Icon name="play" size={16} />Слушать</button>
				<button class="btn" onclick={() => play(true)} disabled={busy || !albums.length}><Icon name="shuffle" size={16} />Перемешать</button>
				<button class="btn" onclick={radio} disabled={busy || !albums.length}><Icon name="radio" size={16} />Радио</button>
			{/snippet}
		</Hero>

		{#if top.value?.length}
			<div class="section-head">
				<h2>Популярное</h2>
				{#if top.value.length > 5}
					<button class="btn small" onclick={() => (showAllTop = !showAllTop)}>{showAllTop ? 'Свернуть' : 'Показать все'}</button>
				{/if}
			</div>
			<SongList songs={showAllTop ? top.value : top.value.slice(0, 5)} showArtist={false} />
		{/if}

		<div class="section-head"><h2>Альбомы</h2></div>
		<div class="grid">
			{#each albums as album (album.id)}<AlbumCard {album} subtitle="year" />{/each}
		</div>
	{:else if artist.loading}
		<div class="empty"><span class="spinner"></span></div>
	{:else}
		<div class="empty"><p>Не удалось открыть исполнителя</p></div>
	{/if}
</div>
