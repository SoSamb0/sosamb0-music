<script lang="ts">
	import { app } from '$lib/app.svelte';
	import AlbumCard from '$lib/components/AlbumCard.svelte';
	import Cover from '$lib/components/Cover.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SongList from '$lib/components/SongList.svelte';
	import { loader } from '$lib/loader.svelte';

	let { q }: { q: string } = $props();

	const result = loader(() => app.api!.search(q));
	const r = $derived(result.value);
	const empty = $derived(!!r && !r.songs.length && !r.albums.length && !r.artists.length);
</script>

<div class="page">
	<h1>«{q}»</h1>
	{#if result.loading && !r}
		<div class="empty"><span class="spinner"></span></div>
	{:else if empty}
		<div class="empty"><Icon name="search" size={30} /><p>Ничего не найдено. Попробуйте другое написание.</p></div>
	{:else if r}
		{#if r.artists.length}
			<div class="section-head"><h2>Исполнители</h2></div>
			<div class="artists">
				{#each r.artists as artist (artist.id)}
					<button class="artist" onclick={() => app.go({ name: 'artist', id: artist.id })}>
						<span class="art"><Cover id={artist.coverArt} size={120} round icon="artist" /></span>
						<span class="ellipsis">{artist.name}</span>
					</button>
				{/each}
			</div>
		{/if}
		{#if r.albums.length}
			<div class="section-head"><h2>Альбомы</h2></div>
			<div class="grid">{#each r.albums as album (album.id)}<AlbumCard {album} />{/each}</div>
		{/if}
		{#if r.songs.length}
			<div class="section-head"><h2>Треки</h2></div>
			<SongList songs={r.songs} />
		{/if}
	{/if}
</div>

<style>
	.artists {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.artist {
		display: flex;
		align-items: center;
		gap: 10px;
		max-width: 260px;
		height: 52px;
		padding: 0 18px 0 6px;
		border-radius: 999px;
		background: var(--surface);
		font-weight: 600;
		transition: background 0.15s;
	}

	.artist:hover {
		background: var(--surface-hover);
	}

	.art {
		flex: none;
		width: 40px;
	}
</style>
