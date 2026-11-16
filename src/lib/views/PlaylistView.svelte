<script lang="ts">
	import { app, formatDuration, plural } from '$lib/app.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SongList from '$lib/components/SongList.svelte';
	import { loader } from '$lib/loader.svelte';
	import { player } from '$lib/player.svelte';

	let { id }: { id: string } = $props();

	const playlist = loader(() => app.api!.playlist(id));
	const songs = $derived(playlist.value?.entry ?? []);

	async function remove(index: number) {
		const ok = await app.attempt(async () => {
			await app.api!.removeFromPlaylist(id, index);
			return true;
		});
		if (ok) {
			playlist.reload();
			app.refreshPlaylists();
		}
	}
</script>

<div class="page">
	{#if playlist.value}
		{@const p = playlist.value}
		<Hero cover={p.coverArt ?? songs[0]?.coverArt} kind="Плейлист" title={p.name} icon="playlist">
			{#snippet meta()}
				<span>{plural(songs.length, 'трек', 'трека', 'треков')}</span>
				{#if songs.length}<span>·</span><span>{formatDuration(songs.reduce((t, s) => t + (s.duration ?? 0), 0))}</span>{/if}
			{/snippet}
			{#snippet actions()}
				<button class="btn primary" onclick={() => player.play(songs)} disabled={!songs.length}><Icon name="play" size={16} />Слушать</button>
				<button class="btn" onclick={() => player.play(songs, 0, true)} disabled={!songs.length}><Icon name="shuffle" size={16} />Перемешать</button>
			{/snippet}
		</Hero>
		{#if songs.length}
			<SongList
				{songs}
				extraMenu={(_, i) => [{ label: '', separator: true }, { label: 'Убрать из плейлиста', icon: 'trash', danger: true, action: () => remove(i) }]}
			/>
		{:else}
			<div class="empty"><p>В плейлисте пока пусто. Добавляйте треки через меню «⋯».</p></div>
		{/if}
	{:else if playlist.loading}
		<div class="empty"><span class="spinner"></span></div>
	{:else}
		<div class="empty"><p>Не удалось открыть плейлист</p></div>
	{/if}
</div>
