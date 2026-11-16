<script lang="ts">
	import { app } from '$lib/app.svelte';
	import { albumMenu, menu } from '$lib/menu.svelte';
	import { player } from '$lib/player.svelte';
	import type { Album } from '$lib/types';
	import Cover from './Cover.svelte';
	import Icon from './Icon.svelte';

	let { album, subtitle = 'artist' }: { album: Album; subtitle?: 'artist' | 'year' } = $props();

	async function play(e: MouseEvent) {
		e.stopPropagation();
		const full = await app.attempt(() => app.api!.album(album.id));
		if (full?.song?.length) player.play(full.song);
	}
</script>

<div class="card" oncontextmenu={(e) => menu.open(e, albumMenu(album))} role="group" aria-label={album.name}>
	<div class="art">
		<button class="open" onclick={() => app.go({ name: 'album', id: album.id })} aria-label="Открыть альбом {album.name}">
			<Cover id={album.coverArt} size={320} />
		</button>
		<button class="play" onclick={play} aria-label="Воспроизвести {album.name}"><Icon name="play" size={20} /></button>
	</div>
	<button class="name ellipsis" onclick={() => app.go({ name: 'album', id: album.id })}>{album.name}</button>
	{#if subtitle === 'artist'}
		<button class="sub ellipsis" onclick={() => album.artistId && app.go({ name: 'artist', id: album.artistId })}>{album.artist ?? ''}</button>
	{:else}
		<div class="sub ellipsis">{album.year ?? ''}</div>
	{/if}
</div>

<style>
	.card {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		min-width: 0;
	}

	.art {
		position: relative;
		width: 100%;
		margin-bottom: 10px;
		border-radius: var(--radius);
		box-shadow: 0 8px 24px rgb(0 0 0 / 0.35);
		transition: transform 0.25s var(--ease);
	}

	.card:hover .art {
		transform: translateY(-3px);
	}

	.open {
		display: block;
		width: 100%;
		border-radius: var(--radius);
		overflow: hidden;
	}

	.play {
		position: absolute;
		right: 10px;
		bottom: 10px;
		display: grid;
		place-items: center;
		width: 44px;
		height: 44px;
		border-radius: 50%;
		background: var(--accent);
		color: #0b0b10;
		box-shadow: 0 6px 18px rgb(0 0 0 / 0.5);
		opacity: 0;
		transform: translateY(6px) scale(0.9);
		transition:
			opacity 0.2s,
			transform 0.2s var(--ease);
	}

	.card:hover .play,
	.play:focus-visible {
		opacity: 1;
		transform: none;
	}

	.play:hover {
		filter: brightness(1.1);
		transform: scale(1.06);
	}

	.name {
		max-width: 100%;
		font-weight: 600;
		text-align: left;
	}

	.sub {
		max-width: 100%;
		color: var(--muted);
		font-size: 13px;
		text-align: left;
	}

	button.sub:hover,
	.name:hover {
		text-decoration: underline;
	}
</style>
