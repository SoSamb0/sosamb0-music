<script lang="ts">
	import { app, plural } from '$lib/app.svelte';
	import AlbumCard from '$lib/components/AlbumCard.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SongList from '$lib/components/SongList.svelte';
	import { loader } from '$lib/loader.svelte';
	import { player } from '$lib/player.svelte';

	const starred = loader(() => app.api!.starred());
	const songs = $derived(starred.value?.songs ?? []);
	const albums = $derived(starred.value?.albums ?? []);
</script>

<div class="page">
	<div class="head">
		<div>
			<h1>Избранное</h1>
			<p class="muted">{plural(songs.length, 'трек', 'трека', 'треков')}</p>
		</div>
		<div class="actions">
			<button class="btn primary" onclick={() => player.play(songs)} disabled={!songs.length}><Icon name="play" size={16} />Слушать</button>
			<button class="btn" onclick={() => player.play(songs, 0, true)} disabled={!songs.length}><Icon name="shuffle" size={16} />Перемешать</button>
		</div>
	</div>

	{#if starred.loading}
		<div class="empty"><span class="spinner"></span></div>
	{:else if !songs.length && !albums.length}
		<div class="empty"><Icon name="heart" size={30} /><p>Отмечайте треки сердечком — они появятся здесь</p></div>
	{:else}
		{#if albums.length}
			<div class="section-head"><h2>Альбомы</h2></div>
			<div class="grid">{#each albums as album (album.id)}<AlbumCard {album} />{/each}</div>
			<div class="section-head"><h2>Треки</h2></div>
		{/if}
		<SongList {songs} />
	{/if}
</div>

<style>
	.head {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 20px;
	}

	.actions {
		display: flex;
		gap: 8px;
	}
</style>
