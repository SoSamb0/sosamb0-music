<script lang="ts">
	import { app, formatDuration, plural } from '$lib/app.svelte';
	import Hero from '$lib/components/Hero.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SongList from '$lib/components/SongList.svelte';
	import { loader } from '$lib/loader.svelte';
	import { albumMenu, menu } from '$lib/menu.svelte';
	import { formatQuality, player } from '$lib/player.svelte';
	import type { Song } from '$lib/types';

	let { id }: { id: string } = $props();

	const album = loader(() => app.api!.album(id));
	const songs = $derived(album.value?.song ?? []);

	const discs = $derived.by(() => {
		const map = new Map<number, Song[]>();
		for (const s of songs) {
			const d = s.discNumber ?? 1;
			if (!map.has(d)) map.set(d, []);
			map.get(d)!.push(s);
		}
		return [...map.entries()].sort((a, b) => a[0] - b[0]);
	});

	let starred = $state(false);
	$effect(() => {
		starred = !!album.value?.starred;
	});

	async function toggleStar() {
		starred = !starred;
		const ok = await app.attempt(async () => {
			await app.api!.star('album', id, starred);
			return true;
		});
		if (!ok) starred = !starred;
	}
</script>

<div class="page">
	{#if album.value}
		{@const a = album.value}
		<Hero cover={a.coverArt} kind="Альбом" title={a.name}>
			{#snippet meta()}
				{#if a.artist}
					<button class="link" onclick={() => a.artistId && app.go({ name: 'artist', id: a.artistId })}>{a.artist}</button><span>·</span>
				{/if}
				{#if a.year}<span>{a.year}</span><span>·</span>{/if}
				<span>{plural(songs.length, 'трек', 'трека', 'треков')}</span><span>·</span>
				<span>{formatDuration(a.duration ?? songs.reduce((t, s) => t + (s.duration ?? 0), 0))}</span>
				{#if songs[0]}<span class="badge hi">{formatQuality(songs[0])}</span>{/if}
			{/snippet}
			{#snippet actions()}
				<button class="btn primary" onclick={() => player.play(songs)} disabled={!songs.length}><Icon name="play" size={16} />Слушать</button>
				<button class="btn" onclick={() => player.play(songs, 0, true)} disabled={!songs.length}><Icon name="shuffle" size={16} />Перемешать</button>
				<button class="icon-btn" class:on={starred} onclick={toggleStar} aria-label={starred ? 'Убрать из избранного' : 'В избранное'}>
					<Icon name={starred ? 'heartFill' : 'heart'} />
				</button>
				<button class="icon-btn" onclick={(e) => menu.open(e, albumMenu(a))} aria-label="Ещё"><Icon name="more" /></button>
			{/snippet}
		</Hero>

		{#each discs as [disc, list] (disc)}
			{#if discs.length > 1}<h2 class="disc">Диск {disc}</h2>{/if}
			<SongList songs={list} numbering="track" showCover={false} showAlbum={false} />
		{/each}
	{:else if album.loading}
		<div class="empty"><span class="spinner"></span></div>
	{:else}
		<div class="empty"><p>Не удалось открыть альбом</p></div>
	{/if}
</div>

<style>
	.link:hover {
		text-decoration: underline;
		color: var(--text);
	}

	.link {
		font-weight: 600;
		color: var(--text);
	}

	.disc {
		margin: 22px 0 8px 10px;
		font-size: 14px;
		color: var(--muted);
	}
</style>
