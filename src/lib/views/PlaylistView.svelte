<script lang="ts">
	import { app, formatDuration, plural } from '$lib/app.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SongList from '$lib/components/SongList.svelte';
	import { loader } from '$lib/loader.svelte';
	import { player } from '$lib/player.svelte';

	let { id }: { id: string } = $props();

	// Reloads when tracks are added to or removed from any playlist.
	const playlist = loader(() => {
		app.playlistRevision;
		return app.api!.playlist(id);
	});
	const songs = $derived(playlist.value?.entry ?? []);

	/** Show the new order at once, then save it; on failure the reload puts the old order back. */
	let pending = $state<typeof songs | null>(null);
	const shown = $derived(pending ?? songs);

	async function move(order: number[]) {
		const before = songs;
		pending = order.map((i) => before[i]);
		const ok = await app.attempt(async () => {
			await app.api!.reorderPlaylist(id, pending!.map((s) => s.id), before.length);
			return true;
		}, 'Не удалось сохранить порядок треков');
		if (ok) app.refreshPlaylists();
		else playlist.reload();
	}

	// Clear the optimistic order once the server's copy arrives.
	$effect(() => {
		songs;
		pending = null;
	});

	async function remove(indexes: number | number[]) {
		const count = [indexes].flat().length;
		const ok = await app.attempt(async () => {
			await app.api!.removeFromPlaylist(id, indexes);
			return true;
		});
		if (ok) {
			app.toast(count === 1 ? 'Трек убран из плейлиста' : `Убрано из плейлиста: ${count}`);
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
				<button class="btn primary" onclick={() => player.play(shown)} disabled={!songs.length}><Icon name="play" size={16} />Слушать</button>
				<button class="btn" onclick={() => player.play(shown, 0, true)} disabled={!songs.length}><Icon name="shuffle" size={16} />Перемешать</button>
			{/snippet}
		</Hero>
		{#if songs.length}
			<SongList
				songs={shown}
				extraMenu={(_, i) => [{ label: '', separator: true }, { label: 'Убрать из плейлиста', icon: 'trash', danger: true, action: () => remove(i) }]}
				onremove={(indexes) => remove(indexes)}
				onreorder={(order) => move(order)}
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
