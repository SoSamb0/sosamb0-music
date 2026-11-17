<script lang="ts">
	import { app, type Route } from '$lib/app.svelte';
	import { player } from '$lib/player.svelte';
	import Cover from '../Cover.svelte';
	import Icon from '../Icon.svelte';

	const TABS: { route: Route; label: string; icon: string; match: string[] }[] = [
		{ route: { name: 'home' }, label: 'Главная', icon: 'home', match: ['home'] },
		{ route: { name: 'search', q: '' }, label: 'Поиск', icon: 'search', match: ['search'] },
		{
			route: { name: 'library' },
			label: 'Библиотека',
			icon: 'album',
			match: ['library', 'albums', 'album', 'songs', 'artists', 'artist', 'genres', 'genre', 'favorites', 'stats', 'settings']
		},
		{ route: { name: 'playlists' }, label: 'Плейлисты', icon: 'playlist', match: ['playlists', 'playlist', 'smart'] }
	];

	const song = $derived(player.current);
	const current = $derived(app.route.name);
	let now = $state(0);

	$effect(() => {
		now = player.livePosition();
		const id = setInterval(() => (now = player.livePosition()), 1000);
		return () => clearInterval(id);
	});
	const progress = $derived(player.duration > 0 ? Math.min(1, now / player.duration) : 0);

	// Swipe the mini player up to open the full player.
	let startY: number | null = null;
	function down(e: PointerEvent) {
		startY = e.clientY;
	}
	function up(e: PointerEvent) {
		if (startY !== null && startY - e.clientY > 40) app.nowPlaying = true;
		startY = null;
	}

</script>

<div class="dock">
	{#if song}
		<div class="mini" onpointerdown={down} onpointerup={up} role="presentation">
			<button class="open" onclick={() => (app.nowPlaying = true)} aria-label="Открыть плеер">
				<span class="art"><Cover id={song.coverArt} size={120} /></span>
				<span class="meta">
					<span class="title ellipsis">{song.title}</span>
					<span class="artist ellipsis">{song.artist ?? ''}</span>
				</span>
			</button>
			<button class="icon-btn big" onclick={() => player.toggle()} aria-label={player.playing ? 'Пауза' : 'Играть'}>
				{#if player.state === 'loading' || player.state === 'buffering'}
					<span class="spinner"></span>
				{:else}
					<Icon name={player.playing ? 'pause' : 'play'} size={24} />
				{/if}
			</button>
			<button class="icon-btn big" onclick={() => player.next()} aria-label="Следующий"><Icon name="next" size={22} /></button>
			<span class="progress" style:--p={progress}></span>
		</div>
	{/if}
	<nav class="tabs">
		{#each TABS as t (t.label)}
			{@const active = t.match.includes(current)}
			<button class="tab" class:active onclick={() => app.go(t.route)}>
				<Icon name={t.icon} size={22} />
				<span>{t.label}</span>
			</button>
		{/each}
	</nav>
</div>

<style>
	.dock {
		flex: none;
		background: rgb(14 14 18 / 0.86);
		backdrop-filter: blur(30px) saturate(1.4);
		border-top: 1px solid var(--line);
		padding-bottom: env(safe-area-inset-bottom);
	}

	.mini {
		position: relative;
		display: flex;
		align-items: center;
		gap: 4px;
		height: 62px;
		padding: 0 8px 0 10px;
		touch-action: pan-x;
	}

	.open {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
		height: 100%;
		text-align: left;
	}

	.art {
		flex: none;
		width: 44px;
		border-radius: 7px;
		overflow: hidden;
	}

	.meta {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.title {
		font-weight: 600;
	}

	.artist {
		font-size: 13px;
		color: var(--muted);
	}

	.big {
		width: 46px;
		height: 46px;
	}

	.progress {
		position: absolute;
		left: 10px;
		right: 10px;
		bottom: 0;
		height: 2px;
		border-radius: 1px;
		background: linear-gradient(90deg, var(--accent) calc(var(--p) * 100%), rgb(255 255 255 / 0.1) 0);
	}

	.tabs {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		height: 58px;
	}

	.tab {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 3px;
		font-size: 11px;
		color: var(--muted);
	}

	.tab.active {
		color: var(--accent);
	}
</style>
