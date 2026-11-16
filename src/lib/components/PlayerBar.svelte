<script lang="ts">
	import { app } from '$lib/app.svelte';
	import { menu } from '$lib/menu.svelte';
	import { formatQuality, player } from '$lib/player.svelte';
	import { settings } from '$lib/settings.svelte';
	import Disc from './Disc.svelte';
	import Icon from './Icon.svelte';
	import SeekBar from './SeekBar.svelte';

	const song = $derived(player.current);
	const signal = $derived(player.signal);
	const volume = $derived(settings.data.volume);
	let lastVolume = 1;

	// One word that tells the truth about what is reaching the DAC.
	const verdict = $derived.by(() => {
		if (!signal) return null;
		if (signal.bitPerfect) return { text: 'Bit-perfect', cls: 'perfect' };
		if (signal.source.lossless) return { text: 'Lossless', cls: 'lossless' };
		return { text: signal.source.codec.toUpperCase(), cls: '' };
	});

	function toggleMute() {
		if (volume > 0) {
			lastVolume = volume;
			player.setVolume(0);
		} else {
			player.setVolume(lastVolume || 1);
		}
	}

	function wheel(e: WheelEvent) {
		e.preventDefault();
		player.setVolume(volume + (e.deltaY < 0 ? 0.04 : -0.04));
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
			set(45, '45 минут'),
			set(60, '1 час'),
			set(90, '1,5 часа'),
			set('track', 'До конца трека'),
			{ label: '', separator: true },
			{ ...set(null, 'Выключить таймер'), icon: 'x' }
		]);
	}

	const roundArt = $derived(
		settings.data.look.coverShape === 'round' || (settings.data.look.coverShape === 'vinyl' && settings.data.look.vinylEverywhere)
	);
	const sleeping = $derived(player.sleepUntil !== null || player.sleepAfterTrack);
</script>

<footer class="bar">
	<div class="now">
		{#if song}
			<button class="art" class:round={roundArt} onclick={() => (app.nowPlaying = !app.nowPlaying)} aria-label="Развернуть плеер">
				<Disc id={song.coverArt} size={160} eager small />
				<span class="expand"><Icon name={app.nowPlaying ? 'down' : 'up'} size={20} /></span>
			</button>
			<div class="meta">
				<button class="title ellipsis" onclick={() => song.albumId && app.go({ name: 'album', id: song.albumId })}>{song.title}</button>
				<button class="artist ellipsis" onclick={() => song.artistId && app.go({ name: 'artist', id: song.artistId })}>{song.artist ?? ''}</button>
			</div>
			<button class="icon-btn" class:on={!!song.starred} onclick={() => player.toggleStar(song)} aria-label={song.starred ? 'Убрать из избранного' : 'В избранное'}>
				<Icon name={song.starred ? 'heartFill' : 'heart'} size={17} />
			</button>
		{:else}
			<div class="idle muted">Ничего не играет</div>
		{/if}
	</div>

	<div class="center">
		<div class="controls">
			<button class="icon-btn" class:on={player.shuffle} onclick={() => player.toggleShuffle()} aria-label="Перемешать" aria-pressed={player.shuffle}>
				<Icon name="shuffle" size={17} />
			</button>
			<button class="icon-btn skip" onclick={() => player.previous()} disabled={!song} aria-label="Предыдущий"><Icon name="prev" size={20} /></button>
			<button class="main" onclick={() => player.toggle()} disabled={!song} aria-label={player.playing ? 'Пауза' : 'Играть'}>
				{#if player.state === 'loading' || player.state === 'buffering'}
					<span class="spinner dark"></span>
				{:else}
					<Icon name={player.playing ? 'pause' : 'play'} size={20} />
				{/if}
			</button>
			<button class="icon-btn skip" onclick={() => player.next()} disabled={!song} aria-label="Следующий"><Icon name="next" size={20} /></button>
			<button class="icon-btn" class:on={player.repeat !== 'off'} onclick={() => player.cycleRepeat()} aria-label="Повтор">
				<Icon name={player.repeat === 'one' ? 'repeat1' : 'repeat'} size={17} />
			</button>
		</div>
		<SeekBar />
	</div>

	<div class="side">
		{#if verdict && song}
			<button class="signal {verdict.cls}" onclick={() => (app.soundOpen = true)} title="Звуковой тракт: {formatQuality(song)}">
				<span class="dot"></span>{verdict.text}
			</button>
		{/if}
		<button class="icon-btn" class:on={app.panel === 'lyrics'} onclick={() => (app.panel = app.panel === 'lyrics' ? null : 'lyrics')} aria-label="Текст песни" title="Текст песни">
			<Icon name="lyrics" />
		</button>
		<button class="icon-btn" class:on={app.panel === 'queue'} onclick={() => (app.panel = app.panel === 'queue' ? null : 'queue')} aria-label="Очередь" title="Очередь">
			<Icon name="queue" />
		</button>
		<button class="icon-btn" class:on={settings.data.dsp.enabled} onclick={() => (app.soundOpen = true)} aria-label="Звук и эквалайзер" title="Звук и эквалайзер">
			<Icon name="sliders" />
		</button>
		<button class="icon-btn" class:on={sleeping} onclick={sleepMenu} aria-label="Таймер сна" title="Таймер сна"><Icon name="moon" /></button>
		<div class="volume" onwheel={wheel}>
			<button class="icon-btn" onclick={toggleMute} aria-label={volume > 0 ? 'Выключить звук' : 'Включить звук'}>
				<Icon name={volume === 0 ? 'mute' : volume < 0.5 ? 'volumeLow' : 'volume'} />
			</button>
			<input
				type="range"
				min="0"
				max="1"
				step="0.01"
				value={volume}
				style:--fill="{volume * 100}%"
				oninput={(e) => player.setVolume(+e.currentTarget.value)}
				aria-label="Громкость"
			/>
		</div>
	</div>
</footer>

<style>
	.bar {
		display: grid;
		grid-template-columns: minmax(180px, 1fr) minmax(320px, 1.4fr) minmax(180px, 1fr);
		align-items: center;
		gap: 20px;
		height: 86px;
		padding: 0 18px;
		background: rgb(14 14 18 / 0.72);
		backdrop-filter: blur(30px) saturate(1.4);
		border-top: 1px solid var(--line);
		container-type: inline-size;
	}

	.now {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}

	.art {
		position: relative;
		flex: none;
		width: 56px;
		border-radius: 8px;
		overflow: hidden;
		box-shadow: 0 4px 14px rgb(0 0 0 / 0.4);
	}

	.art.round {
		border-radius: 50%;
	}

	.expand {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		background: rgb(0 0 0 / 0.5);
		opacity: 0;
		transition: opacity 0.15s;
	}

	.art:hover .expand {
		opacity: 1;
	}

	.meta {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		min-width: 0;
	}

	.title {
		max-width: 100%;
		font-weight: 600;
		text-align: left;
	}

	.artist {
		max-width: 100%;
		font-size: 13px;
		color: var(--muted);
		text-align: left;
	}

	.title:hover,
	.artist:hover {
		text-decoration: underline;
	}

	.center {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
		min-width: 0;
	}

	.controls {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.skip {
		color: var(--text);
	}

	.main {
		display: grid;
		place-items: center;
		width: 40px;
		height: 40px;
		margin: 0 6px;
		border-radius: 50%;
		background: #fff;
		color: #0b0b10;
		transition: transform 0.15s var(--ease);
	}

	.main:hover:not(:disabled) {
		transform: scale(1.07);
	}

	.main:active:not(:disabled) {
		transform: scale(0.95);
	}

	.spinner.dark {
		width: 18px;
		height: 18px;
		border-color: rgb(0 0 0 / 0.15);
		border-top-color: #0b0b10;
	}

	.side {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 2px;
		min-width: 0;
	}

	.signal {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 24px;
		margin-right: 8px;
		padding: 0 9px;
		border-radius: 6px;
		background: var(--surface-hover);
		color: var(--muted);
		font-size: 10.5px;
		font-weight: 700;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		white-space: nowrap;
	}

	.signal .dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: currentColor;
	}

	.signal.lossless {
		color: var(--accent);
		background: var(--accent-soft);
	}

	.signal.perfect {
		color: var(--good);
		background: rgb(74 222 128 / 0.14);
	}

	.signal.perfect .dot {
		box-shadow: 0 0 8px var(--good);
	}

	.volume {
		display: flex;
		align-items: center;
		gap: 2px;
		margin-left: 4px;
	}

	.volume input {
		width: 92px;
	}

	@container (max-width: 1080px) {
		.signal,
		.volume input {
			display: none;
		}
	}
</style>
