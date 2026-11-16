<script lang="ts">
	import { app } from '$lib/app.svelte';
	import { formatQuality, player } from '$lib/player.svelte';
	import Cover from './Cover.svelte';
	import Icon from './Icon.svelte';
	import LyricsView from './LyricsView.svelte';

	const song = $derived(player.current);
	const hasLyrics = $derived(player.lyricsLoading || !!player.lyrics?.lines.length);
</script>

<div class="np" class:solo={!hasLyrics}>
	<button class="icon-btn close" onclick={() => (app.nowPlaying = false)} aria-label="Свернуть"><Icon name="down" size={22} /></button>
	{#if song}
		<div class="left">
			<div class="art"><Cover id={song.coverArt} size={800} eager /></div>
			<div class="info">
				<h1 class="ellipsis">{song.title}</h1>
				<button class="artist ellipsis" onclick={() => song.artistId && app.go({ name: 'artist', id: song.artistId })}>{song.artist ?? ''}</button>
				<button class="album ellipsis" onclick={() => song.albumId && app.go({ name: 'album', id: song.albumId })}>
					{song.album ?? ''}{song.year ? ` · ${song.year}` : ''}
				</button>
				<div class="tags">
					<span class="badge hi">{formatQuality(song)}</span>
					{#if player.signal?.bitPerfect}<span class="badge good">Bit-perfect</span>{/if}
				</div>
			</div>
		</div>
		{#if hasLyrics}
			<div class="right"><LyricsView large /></div>
		{/if}
	{:else}
		<div class="empty"><p>Ничего не играет</p></div>
	{/if}
</div>

<style>
	.np {
		position: absolute;
		inset: 0;
		z-index: 20;
		display: grid;
		grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
		gap: 5vw;
		align-items: center;
		padding: 5vh 6vw;
		background: rgb(10 10 13 / 0.55);
		backdrop-filter: blur(50px) saturate(1.6);
		animation: rise 0.35s var(--ease);
	}

	.np.solo {
		grid-template-columns: 1fr;
		justify-items: center;
	}

	@keyframes rise {
		from {
			opacity: 0;
			transform: translateY(24px);
		}
	}

	.close {
		position: absolute;
		top: 14px;
		left: 18px;
		z-index: 2;
	}

	.left {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 26px;
		min-width: 0;
		max-height: 100%;
	}

	.art {
		width: min(100%, 52vh);
		border-radius: 18px;
		box-shadow: 0 30px 80px rgb(0 0 0 / 0.6);
	}

	.info {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 3px;
		max-width: 100%;
		text-align: center;
	}

	.info h1 {
		max-width: 100%;
		font-size: 28px;
	}

	.artist {
		max-width: 100%;
		font-size: 17px;
		color: var(--accent);
		font-weight: 600;
	}

	.album {
		max-width: 100%;
		color: var(--muted);
	}

	.artist:hover,
	.album:hover {
		text-decoration: underline;
	}

	.tags {
		display: flex;
		gap: 6px;
		margin-top: 10px;
	}

	.badge.good {
		color: var(--good);
		background: rgb(74 222 128 / 0.14);
	}

	.right {
		height: 100%;
		min-height: 0;
		min-width: 0;
	}

	@media (max-width: 900px) {
		.np {
			grid-template-columns: 1fr;
		}

		.right {
			display: none;
		}
	}
</style>
