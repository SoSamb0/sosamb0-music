<script lang="ts">
	import { app } from '$lib/app.svelte';
	import { menu, songHeader, songMenu } from '$lib/menu.svelte';
	import { formatQuality, player } from '$lib/player.svelte';
	import { settings } from '$lib/settings.svelte';
	import Disc from '../Disc.svelte';
	import Icon from '../Icon.svelte';
	import LyricsView from '../LyricsView.svelte';
	import QueuePanel from '../QueuePanel.svelte';
	import SeekBar from '../SeekBar.svelte';

	const song = $derived(player.current);
	const signal = $derived(player.signal);
	const art = $derived(app.api?.coverUrl(song?.coverArt, 600) ?? '');
	const sleeping = $derived(player.sleepUntil !== null || player.sleepAfterTrack);

	/** What fills the middle: the cover, the lyrics or the queue (app.panel). */
	const view = $derived(app.panel ?? 'cover');

	const verdict = $derived.by(() => {
		if (!signal) return null;
		if (signal.bitPerfect) return { text: 'Bit-perfect', cls: 'perfect' };
		if (signal.source.lossless) return { text: 'Lossless', cls: 'lossless' };
		return { text: signal.source.codec.toUpperCase(), cls: '' };
	});

	function show(panel: 'lyrics' | 'queue') {
		app.panel = app.panel === panel ? null : panel;
	}

	// Pull the cover down to close the player.
	let startY: number | null = null;
	let pull = $state(0);
	function down(e: PointerEvent) {
		if (view !== 'cover') return;
		startY = e.clientY;
	}
	function move(e: PointerEvent) {
		if (startY !== null) pull = Math.max(0, e.clientY - startY);
	}
	function up() {
		if (pull > 110) close();
		startY = null;
		pull = 0;
	}

	function close() {
		app.nowPlaying = false;
		app.panel = null;
	}

	function sleepMenu(e: MouseEvent) {
		const set = (m: number | 'track' | null, label: string) => ({
			label,
			icon: 'clock',
			action: () => {
				player.setSleep(m);
				app.toast(m === null ? 'Таймер сна выключен' : `Таймер сна: ${label.toLowerCase()}`);
			}
		});
		menu.open(e, [
			set(15, '15 минут'),
			set(30, '30 минут'),
			set(60, '1 час'),
			set(90, '1,5 часа'),
			set('track', 'До конца трека'),
			{ label: '', separator: true },
			{ ...set(null, 'Выключить таймер'), icon: 'x' }
		]);
	}
</script>

<div
	class="mp"
	style:translate="0 {pull}px"
	class:pulling={pull > 0}
	onpointerdown={down}
	onpointermove={move}
	onpointerup={up}
	onpointercancel={up}
	role="dialog"
	aria-label="Сейчас играет"
	tabindex="-1"
>
	{#if art && settings.data.look.npBackground !== 'solid'}
		<div class="bg" aria-hidden="true">{#key art}<img src={art} alt="" />{/key}</div>
	{/if}

	<header>
		<button class="icon-btn" onclick={close} aria-label="Свернуть"><Icon name="down" size={24} /></button>
		<span class="head muted">
			{#if view === 'lyrics'}Текст{:else if view === 'queue'}Очередь{:else}Сейчас играет{/if}
		</span>
		<button class="icon-btn" onclick={(e) => song && menu.open(e, songMenu(song), songHeader(song))} disabled={!song} aria-label="Ещё"><Icon name="more" size={22} /></button>
	</header>

	<div class="middle">
		{#if view === 'lyrics'}
			<div class="scroll"><LyricsView large /></div>
		{:else if view === 'queue'}
			<div class="scroll"><QueuePanel /></div>
		{:else if song}
			<div class="art"><Disc id={song.coverArt} size={800} eager /></div>
		{/if}
	</div>

	{#if song}
		<div class="info">
			<div class="text">
				<h1 class="ellipsis">{song.title}</h1>
				<button class="artist ellipsis" onclick={() => song.artistId && app.go({ name: 'artist', id: song.artistId })}>{song.artist ?? ''}</button>
			</div>
			<button class="icon-btn" class:on={!!song.starred} onclick={() => player.toggleStar(song)} aria-label={song.starred ? 'Убрать из избранного' : 'В избранное'}>
				<Icon name={song.starred ? 'heartFill' : 'heart'} size={24} />
			</button>
		</div>
		<div class="quality">
			<button class="signal {verdict?.cls ?? ''}" onclick={() => (app.soundOpen = true)}>
				{#if verdict}<span class="dot"></span>{verdict.text} · {/if}{formatQuality(song)}
			</button>
		</div>
	{/if}

	<div class="seek"><SeekBar /></div>

	<div class="controls">
		<button class="icon-btn" class:on={player.shuffle} onclick={() => player.toggleShuffle()} aria-label="Перемешать" aria-pressed={player.shuffle}>
			<Icon name="shuffle" size={22} />
		</button>
		<button class="icon-btn skip" onclick={() => player.previous()} disabled={!song} aria-label="Предыдущий"><Icon name="prev" size={30} /></button>
		<button class="main" onclick={() => player.toggle()} disabled={!song} aria-label={player.playing ? 'Пауза' : 'Играть'}>
			{#if player.state === 'loading' || player.state === 'buffering'}
				<span class="spinner dark"></span>
			{:else}
				<Icon name={player.playing ? 'pause' : 'play'} size={32} />
			{/if}
		</button>
		<button class="icon-btn skip" onclick={() => player.next()} disabled={!song} aria-label="Следующий"><Icon name="next" size={30} /></button>
		<button class="icon-btn" class:on={player.repeat !== 'off'} onclick={() => player.cycleRepeat()} aria-label="Повтор">
			<Icon name={player.repeat === 'one' ? 'repeat1' : 'repeat'} size={22} />
		</button>
	</div>

	<div class="tools">
		<button class="icon-btn" class:on={view === 'lyrics'} onclick={() => show('lyrics')} aria-label="Текст песни"><Icon name="lyrics" size={22} /></button>
		<button class="icon-btn" class:on={settings.data.dsp.enabled} onclick={() => (app.soundOpen = true)} aria-label="Звук и эквалайзер">
			<Icon name="sliders" size={22} />
		</button>
		<button class="icon-btn" class:on={sleeping} onclick={sleepMenu} aria-label="Таймер сна"><Icon name="moon" size={22} /></button>
		<button class="icon-btn" class:on={view === 'queue'} onclick={() => show('queue')} aria-label="Очередь"><Icon name="queue" size={22} /></button>
	</div>
</div>

<style>
	.mp {
		position: fixed;
		inset: 0;
		z-index: 40;
		display: flex;
		flex-direction: column;
		padding: calc(env(safe-area-inset-top) + 8px) 22px calc(env(safe-area-inset-bottom) + 18px);
		background: #0a0a0d;
		overflow: hidden;
		isolation: isolate;
		animation: rise 0.3s var(--ease);
		transition: translate 0.25s var(--ease);
	}

	.mp.pulling {
		transition: none;
	}

	@keyframes rise {
		from {
			transform: translateY(40px);
			opacity: 0;
		}
	}

	.bg {
		position: absolute;
		inset: -20%;
		z-index: -1;
	}

	.bg img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		filter: blur(70px) saturate(1.5);
		opacity: 0.5;
	}

	.bg::after {
		content: '';
		position: absolute;
		inset: 0;
		background: linear-gradient(180deg, rgb(10 10 13 / 0.35), rgb(10 10 13 / 0.85));
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 48px;
		margin: 0 -10px;
	}

	.head {
		font-size: 13px;
		font-weight: 600;
		letter-spacing: 0.02em;
	}

	.middle {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 0;
		margin: 8px 0 18px;
	}

	.art {
		width: min(100%, 52vh);
		border-radius: 14px;
		overflow: hidden;
		box-shadow: 0 24px 60px rgb(0 0 0 / 0.55);
	}

	.scroll {
		align-self: stretch;
		width: 100%;
		overflow-y: auto;
		margin: 0 -22px;
		padding: 0 6px;
	}

	.info {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.text {
		flex: 1;
		min-width: 0;
	}

	h1 {
		font-size: 22px;
		line-height: 1.25;
	}

	.artist {
		max-width: 100%;
		font-size: 16px;
		color: var(--muted);
		text-align: left;
	}

	.quality {
		margin-top: 8px;
	}

	.signal {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 26px;
		padding: 0 10px;
		border-radius: 999px;
		font-size: 12px;
		color: var(--muted);
		background: rgb(255 255 255 / 0.07);
	}

	.dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: currentColor;
	}

	.signal.lossless {
		color: var(--accent);
	}

	.signal.perfect {
		color: var(--good, #5fd38d);
	}

	.seek {
		margin-top: 14px;
	}

	.controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: 8px;
	}

	.skip {
		width: 56px;
		height: 56px;
	}

	.main {
		display: grid;
		place-items: center;
		width: 72px;
		height: 72px;
		border-radius: 50%;
		background: var(--text, #fff);
		color: #0b0b10;
	}

	.tools {
		display: flex;
		justify-content: space-between;
		margin-top: 18px;
	}
</style>
