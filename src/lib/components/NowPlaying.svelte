<script lang="ts">
	import { app } from '$lib/app.svelte';
	import { formatQuality, player } from '$lib/player.svelte';
	import { settings } from '$lib/settings.svelte';
	import Disc from './Disc.svelte';
	import Icon from './Icon.svelte';
	import LyricsView from './LyricsView.svelte';

	const song = $derived(player.current);
	const look = $derived(settings.data.look);
	const hasLyrics = $derived(look.npLyrics && (player.lyricsLoading || !!player.lyrics?.lines.length));
	const art = $derived(app.api?.coverUrl(song?.coverArt, 600) ?? '');
</script>

<div class="np bg-{look.npBackground}" class:solo={!hasLyrics} class:vinyl={look.coverShape !== 'square'}>
	{#if art && (look.npBackground === 'blur' || look.npBackground === 'motion')}
		<div class="art-bg" aria-hidden="true">
			{#key art}
				<img class="layer a" src={art} alt="" />
				{#if look.npBackground === 'motion'}<img class="layer b" src={art} alt="" />{/if}
			{/key}
		</div>
	{/if}

	<button class="icon-btn close" onclick={() => (app.nowPlaying = false)} aria-label="Свернуть"><Icon name="down" size={22} /></button>
	{#if song}
		<div class="left">
			<div class="art"><Disc id={song.coverArt} size={800} eager /></div>
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
		overflow: hidden;
		isolation: isolate;
		animation: rise 0.35s var(--ease);
	}

	/* "Как раньше": the app behind, frosted. */
	.bg-dim {
		background: rgb(10 10 13 / 0.55);
		backdrop-filter: blur(50px) saturate(1.6);
	}

	.bg-solid {
		background: #0a0a0d;
	}

	.bg-blur,
	.bg-motion {
		background: #0a0a0d;
	}

	.art-bg {
		position: absolute;
		inset: 0;
		z-index: -1;
		overflow: hidden;
	}

	/* Darken towards the text so lyrics stay readable on any cover. */
	.art-bg::after {
		content: '';
		position: absolute;
		inset: 0;
		background:
			radial-gradient(120% 90% at 25% 50%, rgb(0 0 0 / 0.12), rgb(0 0 0 / 0.5)),
			linear-gradient(90deg, rgb(10 10 13 / 0.15), rgb(10 10 13 / 0.45));
	}

	.layer {
		position: absolute;
		left: 50%;
		top: 50%;
		width: 140%;
		height: 140%;
		object-fit: cover;
		translate: -50% -50%;
		filter: blur(70px) saturate(1.6) brightness(0.8);
		animation: fade-in 1s ease;
	}

	@keyframes fade-in {
		from {
			opacity: 0;
		}
	}

	/* "Живой фон": two copies drifting slowly. Only transforms move, so the GPU does the work. */
	.bg-motion .layer {
		will-change: transform;
	}

	.bg-motion .a {
		animation:
			fade-in 1s ease,
			drift-a 38s ease-in-out infinite alternate;
	}

	.bg-motion .b {
		opacity: 0.55;
		mix-blend-mode: screen;
		animation:
			fade-in 1s ease,
			drift-b 52s ease-in-out infinite alternate;
	}

	@keyframes drift-a {
		from {
			transform: rotate(0deg) scale(1);
		}
		to {
			transform: rotate(40deg) scale(1.25);
		}
	}

	@keyframes drift-b {
		from {
			transform: rotate(180deg) scale(1.3) translateX(8%);
		}
		to {
			transform: rotate(110deg) scale(1.05) translateX(-6%);
		}
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

	.vinyl .art {
		border-radius: 50%;
		box-shadow: 0 30px 80px rgb(0 0 0 / 0.7);
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
