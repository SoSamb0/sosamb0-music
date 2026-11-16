<script lang="ts">
	import { app, formatTime } from '$lib/app.svelte';
	import { menu, songMenu, type MenuItem } from '$lib/menu.svelte';
	import { isLossless, player } from '$lib/player.svelte';
	import type { Song } from '$lib/types';
	import Cover from './Cover.svelte';
	import Icon from './Icon.svelte';

	let {
		songs,
		numbering = 'index',
		showCover = true,
		showAlbum = true,
		showArtist = true,
		extraMenu
	}: {
		songs: Song[];
		numbering?: 'index' | 'track';
		showCover?: boolean;
		showAlbum?: boolean;
		showArtist?: boolean;
		extraMenu?: (song: Song, index: number) => MenuItem[];
	} = $props();

	const currentId = $derived(player.current?.id);

	function quality(song: Song): string | null {
		if (!isLossless(song)) return null;
		return (song.bitDepth ?? 16) > 16 || (song.samplingRate ?? 44100) > 48000 ? 'Hi-Res' : 'Lossless';
	}

	function openMenu(e: MouseEvent, song: Song, i: number) {
		menu.open(e, songMenu(song, extraMenu?.(song, i) ?? []));
	}
</script>

<div class="list" class:with-album={showAlbum} role="list">
	{#each songs as song, i (song.id + ':' + i)}
		{@const active = song.id === currentId}
		<div
			class="row"
			class:active
			role="listitem"
			ondblclick={() => player.play(songs, i)}
			oncontextmenu={(e) => openMenu(e, song, i)}
		>
			<button class="num tnum" onclick={() => (active ? player.toggle() : player.play(songs, i))} aria-label="Воспроизвести {song.title}">
				{#if active && player.playing}
					<span class="bars" aria-hidden="true"><i></i><i></i><i></i></span>
				{:else}
					<span class="n">{numbering === 'track' ? (song.track ?? i + 1) : i + 1}</span>
				{/if}
				<span class="p"><Icon name={active && player.playing ? 'pause' : 'play'} size={15} /></span>
			</button>

			<div class="main">
				{#if showCover}
					<div class="thumb"><Cover id={song.coverArt} size={80} /></div>
				{/if}
				<div class="text">
					<div class="title ellipsis">{song.title}</div>
					{#if showArtist && song.artist}
						<button
							class="artist ellipsis"
							onclick={() => song.artistId && app.go({ name: 'artist', id: song.artistId })}
							ondblclick={(e) => e.stopPropagation()}>{song.artist}</button
						>
					{/if}
				</div>
			</div>

			{#if showAlbum}
				<button
					class="album ellipsis"
					onclick={() => song.albumId && app.go({ name: 'album', id: song.albumId })}
					ondblclick={(e) => e.stopPropagation()}>{song.album ?? ''}</button
				>
			{/if}

			<div class="q">
				{#if quality(song)}<span class="badge" class:hi={quality(song) === 'Hi-Res'}>{quality(song)}</span>{/if}
			</div>

			<button
				class="icon-btn heart"
				class:on={!!song.starred}
				onclick={() => player.toggleStar(song)}
				ondblclick={(e) => e.stopPropagation()}
				aria-label={song.starred ? 'Убрать из избранного' : 'В избранное'}
			>
				<Icon name={song.starred ? 'heartFill' : 'heart'} size={16} />
			</button>
			<div class="dur tnum">{formatTime(song.duration)}</div>
			<button class="icon-btn more" onclick={(e) => openMenu(e, song, i)} ondblclick={(e) => e.stopPropagation()} aria-label="Ещё">
				<Icon name="more" size={17} />
			</button>
		</div>
	{/each}
</div>

<style>
	.list {
		display: flex;
		flex-direction: column;
	}

	.row {
		display: grid;
		grid-template-columns: 38px minmax(0, 1fr) 76px 34px 52px 34px;
		align-items: center;
		gap: 10px;
		height: 54px;
		padding: 0 10px 0 6px;
		border-radius: var(--radius-sm);
		transition: background 0.12s;
		content-visibility: auto;
		contain-intrinsic-size: auto 54px;
	}

	.with-album .row {
		grid-template-columns: 38px minmax(0, 1.3fr) minmax(0, 1fr) 76px 34px 52px 34px;
	}

	.row:hover {
		background: var(--surface);
	}

	.row.active {
		background: var(--accent-soft);
	}

	.num {
		position: relative;
		display: grid;
		place-items: center;
		width: 38px;
		height: 38px;
		color: var(--faint);
		font-size: 13px;
	}

	.num .p {
		position: absolute;
		display: none;
		color: var(--text);
	}

	.row:hover .num .p {
		display: block;
	}

	.row:hover .num .n,
	.row:hover .num .bars {
		visibility: hidden;
	}

	.row.active .num {
		color: var(--accent);
	}

	.bars {
		display: flex;
		align-items: flex-end;
		gap: 2px;
		height: 13px;
	}

	.bars i {
		width: 3px;
		border-radius: 1px;
		background: var(--accent);
		animation: bounce 0.9s ease-in-out infinite;
	}

	.bars i:nth-child(2) {
		animation-delay: -0.3s;
	}

	.bars i:nth-child(3) {
		animation-delay: -0.6s;
	}

	@keyframes bounce {
		0%,
		100% {
			height: 4px;
		}
		50% {
			height: 13px;
		}
	}

	.main {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}

	.thumb {
		flex: none;
		width: 40px;
		border-radius: 6px;
		overflow: hidden;
	}

	.text {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		min-width: 0;
	}

	.title {
		max-width: 100%;
		font-weight: 550;
	}

	.row.active .title {
		color: var(--accent);
	}

	.artist,
	.album {
		max-width: 100%;
		color: var(--muted);
		font-size: 13px;
		text-align: left;
	}

	.artist:hover,
	.album:hover {
		color: var(--text);
		text-decoration: underline;
	}

	.q {
		display: flex;
		justify-content: flex-end;
	}

	.heart:not(.on),
	.more {
		opacity: 0;
	}

	.row:hover .heart,
	.row:hover .more,
	.heart:focus-visible,
	.more:focus-visible {
		opacity: 1;
	}

	.dur {
		color: var(--muted);
		font-size: 13px;
		text-align: right;
	}

	@container main (max-width: 720px) {
		.with-album .row {
			grid-template-columns: 38px minmax(0, 1fr) 34px 52px 34px;
		}

		.row {
			grid-template-columns: 38px minmax(0, 1fr) 34px 52px 34px;
		}

		.album,
		.q {
			display: none;
		}
	}
</style>
