<script lang="ts">
	import '../app.css';
	import { onMount, untrack } from 'svelte';
	import { app } from '$lib/app.svelte';
	import { accentFromImage } from '$lib/color';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import Login from '$lib/components/Login.svelte';
	import LyricsView from '$lib/components/LyricsView.svelte';
	import MiniPlayer from '$lib/components/MiniPlayer.svelte';
	import NowPlaying from '$lib/components/NowPlaying.svelte';
	import Overlays from '$lib/components/Overlays.svelte';
	import PlayerBar from '$lib/components/PlayerBar.svelte';
	import QueuePanel from '$lib/components/QueuePanel.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import SoundPanel from '$lib/components/SoundPanel.svelte';
	import Titlebar from '$lib/components/Titlebar.svelte';
	import MenuSheet from '$lib/components/mobile/MenuSheet.svelte';
	import MobileHeader from '$lib/components/mobile/MobileHeader.svelte';
	import MobileNav from '$lib/components/mobile/MobileNav.svelte';
	import MobilePlayer from '$lib/components/mobile/MobilePlayer.svelte';
	import { isMobile } from '$lib/platform';
	import Library from '$lib/views/Library.svelte';
	import Playlists from '$lib/views/Playlists.svelte';
	import { initNative, isQuitting } from '$lib/native';
	import { player } from '$lib/player.svelte';
	import { settings } from '$lib/settings.svelte';
	import { onCloseRequested, setZoom } from '$lib/window';
	import Albums from '$lib/views/Albums.svelte';
	import AlbumView from '$lib/views/AlbumView.svelte';
	import Artists from '$lib/views/Artists.svelte';
	import ArtistView from '$lib/views/ArtistView.svelte';
	import Favorites from '$lib/views/Favorites.svelte';
	import Genres from '$lib/views/Genres.svelte';
	import Home from '$lib/views/Home.svelte';
	import PlaylistView from '$lib/views/PlaylistView.svelte';
	import Search from '$lib/views/Search.svelte';
	import Settings from '$lib/views/Settings.svelte';
	import Songs from '$lib/views/Songs.svelte';
	import SmartView from '$lib/views/SmartView.svelte';
	import Stats from '$lib/views/Stats.svelte';

	let titlebar = $state<ReturnType<typeof Titlebar>>();
	let mobileScrolled = $state(false);

	// The phone's floating buttons get a backing once the page moves under them.
	function onMobileScroll() {
		const scrolled = (scroller?.scrollTop ?? 0) > 24;
		if (scrolled !== mobileScrolled) mobileScrolled = scrolled;
	}
	let scroller = $state<HTMLElement>();

	const route = $derived(app.route);
	const song = $derived(player.current);
	const cover = $derived(app.api?.coverUrl(song?.coverArt, 300) ?? '');
	const look = $derived(settings.data.look);
	const backdrop = $derived(look.appBackdrop ? cover : '');

	onMount(() => {
		app.restore();
		initNative();
		onCloseRequested(() => settings.data.closeToTray && !isQuitting());
	});

	// The engine and the saved queue come up once we know which server we are talking to.
	$effect(() => {
		// Only the server connection should trigger this, not the queue state init() touches.
		if (app.api) untrack(() => player.init());
	});

	// Tint the whole interface with the colour of the current cover, or a fixed colour.
	$effect(() => {
		const root = document.documentElement.style;
		const fixed = look.accent;
		if (fixed !== 'cover') {
			const n = parseInt(fixed.slice(1), 16);
			root.setProperty('--accent-rgb', `${(n >> 16) & 255} ${(n >> 8) & 255} ${n & 255}`);
			return;
		}
		const url = cover;
		let stale = false;
		accentFromImage(url).then(([r, g, b]) => {
			if (!stale) root.setProperty('--accent-rgb', `${r} ${g} ${b}`);
		});
		return () => {
			stale = true;
		};
	});

	if (isMobile) document.documentElement.dataset.mobile = '';

	$effect(() => {
		document.documentElement.dataset.motion = look.animations ? 'on' : 'off';
	});

	$effect(() => {
		setZoom(look.scale);
	});

	// Each page starts at the top.
	$effect(() => {
		route;
		scroller?.scrollTo({ top: 0 });
		mobileScrolled = false;
	});

	function onKey(e: KeyboardEvent) {
		if (!app.api) return;
		// Key events can come from the window itself, which is not an element.
		const target = e.target instanceof HTMLElement ? e.target : document.body;
		const typing = target.closest('input, textarea, select, [contenteditable]');
		if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
			e.preventDefault();
			titlebar?.focusSearch();
			return;
		}
		if (typing) return;
		const key = e.key.toLowerCase();
		const ctrl = e.ctrlKey || e.metaKey;
		if (e.altKey && e.key === 'ArrowLeft') app.back();
		else if (e.altKey && e.key === 'ArrowRight') app.forward();
		else if (e.code === 'Space') {
			// A focused button would otherwise be "clicked" by the same key press.
			if (target.closest('button')) (target as HTMLElement).blur();
			player.toggle();
		} else if (ctrl && e.key === 'ArrowRight') player.next();
		else if (ctrl && e.key === 'ArrowLeft') player.previous();
		else if (e.key === 'ArrowRight') player.seek(player.livePosition() + 5);
		else if (e.key === 'ArrowLeft') player.seek(player.livePosition() - 5);
		else if (e.key === 'ArrowUp') player.setVolume(settings.data.volume + 0.05);
		else if (e.key === 'ArrowDown') player.setVolume(settings.data.volume - 0.05);
		else if (ctrl || e.altKey) return;
		else if (key === 'm' || key === 'ь') player.setVolume(settings.data.volume > 0 ? 0 : 1);
		else if (key === 'l' || key === 'д') app.panel = app.panel === 'lyrics' ? null : 'lyrics';
		else if (key === 'q' || key === 'й') app.panel = app.panel === 'queue' ? null : 'queue';
		else if (key === 'e' || key === 'у') app.soundOpen = !app.soundOpen;
		else if (key === 'f' || key === 'а') app.nowPlaying = !app.nowPlaying;
		else if (e.key === 'Escape') app.nowPlaying = false;
		else return;
		e.preventDefault();
	}

	function onMouse(e: MouseEvent) {
		// Side buttons of the mouse.
		if (e.button === 3) (e.preventDefault(), app.back());
		else if (e.button === 4) (e.preventDefault(), app.forward());
	}
</script>

{#snippet views()}
	{#key JSON.stringify(route)}
		{#if route.name === 'home'}<Home />
		{:else if route.name === 'albums'}<Albums />
		{:else if route.name === 'songs'}<Songs />
		{:else if route.name === 'smart'}<SmartView kind={route.id} />
		{:else if route.name === 'album'}<AlbumView id={route.id} />
		{:else if route.name === 'artists'}<Artists />
		{:else if route.name === 'artist'}<ArtistView id={route.id} />
		{:else if route.name === 'playlist'}<PlaylistView id={route.id} />
		{:else if route.name === 'favorites'}<Favorites />
		{:else if route.name === 'genres'}<Genres />
		{:else if route.name === 'genre'}<Albums genre={route.id} />
		{:else if route.name === 'search'}<Search q={route.q} />
		{:else if route.name === 'stats'}<Stats />
		{:else if route.name === 'settings'}<Settings />
		{:else if route.name === 'library'}<Library />
		{:else if route.name === 'playlists'}<Playlists />
		{/if}
	{/key}
{/snippet}

<svelte:window onkeydown={onKey} onmouseup={onMouse} oncontextmenu={(e) => e.preventDefault()} />

{#if !app.api}
	<Login />
{:else if isMobile}
	<div class="app mobile">
		{#if backdrop}
			{#key backdrop}<img class="backdrop" src={backdrop} alt="" style:--strength={look.backdropStrength} />{/key}
		{/if}
		<div class="shade"></div>
		<main bind:this={scroller} onscroll={onMobileScroll}>
			<MobileHeader scrolled={mobileScrolled} />
			{@render views()}
		</main>
		<MobileNav />
	</div>
	{#if app.nowPlaying}<MobilePlayer />{/if}
	{#if app.soundOpen}<SoundPanel />{/if}
{:else if app.mini}
	{#if backdrop}<img class="backdrop" src={backdrop} alt="" />{/if}
	<MiniPlayer />
{:else}
	<div class="app">
		{#if backdrop}
			{#key backdrop}<img class="backdrop" src={backdrop} alt="" style:--strength={look.backdropStrength} />{/key}
		{/if}
		<div class="shade"></div>

		<Sidebar />
		<div class="center">
			<Titlebar bind:this={titlebar} />
			<div class="stage">
				<main bind:this={scroller}>
					{@render views()}
				</main>
				{#if app.panel}
					<aside class="panel">
						{#if app.panel === 'queue'}<QueuePanel />{:else}
							<div class="lyrics-head"><h2>Текст</h2></div>
							<LyricsView />
						{/if}
					</aside>
				{/if}
				{#if app.nowPlaying}<NowPlaying />{/if}
			</div>
		</div>
		<div class="bar"><PlayerBar /></div>
	</div>
	{#if app.soundOpen}<SoundPanel />{/if}
{/if}

{#if isMobile}<MenuSheet />{:else}<ContextMenu />{/if}
<Overlays />

<style>
	.app {
		position: relative;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr) auto;
		height: 100vh;
		isolation: isolate;
	}

	.backdrop {
		position: fixed;
		inset: -12%;
		z-index: -2;
		width: 124%;
		height: 124%;
		object-fit: cover;
		filter: blur(90px) saturate(1.7);
		opacity: var(--strength, 0.42);
		pointer-events: none;
		animation: backdrop-in 1.2s ease;
	}

	@keyframes backdrop-in {
		from {
			opacity: 0;
		}
	}

	.shade {
		position: fixed;
		inset: 0;
		z-index: -1;
		background: linear-gradient(180deg, rgb(10 10 13 / 0.5), rgb(10 10 13 / 0.86));
		pointer-events: none;
	}

	.center {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
	}

	.stage {
		position: relative;
		flex: 1;
		display: flex;
		min-height: 0;
	}

	main {
		flex: 1;
		min-width: 0;
		overflow-y: auto;
		container: main / inline-size;
	}

	.panel {
		display: flex;
		flex-direction: column;
		width: 340px;
		flex: none;
		min-height: 0;
		margin: 0 10px 10px 0;
		border-radius: 16px;
		background: rgb(0 0 0 / 0.26);
		border: 1px solid var(--line);
		animation: panel-in 0.25s var(--ease);
	}

	@keyframes panel-in {
		from {
			opacity: 0;
			transform: translateX(16px);
		}
	}

	.lyrics-head {
		padding: 18px 22px 4px;
	}

	.bar {
		grid-column: 1 / -1;
		z-index: 30;
	}

	.app.mobile {
		display: flex;
		flex-direction: column;
		height: 100dvh;
	}

	.app.mobile main {
		flex: 1;
		min-height: 0;
		overscroll-behavior: contain;
	}

	/* The blur fades out towards the image's edges; on a phone the page reaches the screen's top
	   edge (under the status bar), so the image is made larger to keep the glow there too. */
	.app.mobile .backdrop {
		inset: -40%;
		width: 180%;
		height: 180%;
	}

	@media (max-width: 1040px) {
		.panel {
			position: absolute;
			inset: 0 0 0 auto;
			z-index: 10;
			background: rgb(18 18 23 / 0.96);
		}
	}
</style>
