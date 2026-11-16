<script lang="ts">
	import { app } from '$lib/app.svelte';
	import { player } from '$lib/player.svelte';
	import { setMiniMode } from '$lib/window';
	import { settings } from '$lib/settings.svelte';
	import Disc from './Disc.svelte';
	import Icon from './Icon.svelte';
	import SeekBar from './SeekBar.svelte';

	const song = $derived(player.current);

	async function restore() {
		await setMiniMode(false);
		app.mini = false;
	}
</script>

<div class="mini" data-tauri-drag-region>
	<div class="art" class:round={settings.data.look.coverShape !== 'square'}><Disc id={song?.coverArt} size={200} eager small /></div>
	<div class="body">
		<div class="top">
			<div class="text">
				<div class="title ellipsis">{song?.title ?? 'Ничего не играет'}</div>
				<div class="artist ellipsis">{song?.artist ?? ''}</div>
			</div>
			<button class="icon-btn" onclick={restore} aria-label="Развернуть окно" title="Развернуть окно"><Icon name="max" size={14} /></button>
		</div>
		<div class="controls">
			<button class="icon-btn" onclick={() => player.previous()} disabled={!song} aria-label="Предыдущий"><Icon name="prev" /></button>
			<button class="icon-btn play" onclick={() => player.toggle()} disabled={!song} aria-label={player.playing ? 'Пауза' : 'Играть'}>
				<Icon name={player.playing ? 'pause' : 'play'} size={20} />
			</button>
			<button class="icon-btn" onclick={() => player.next()} disabled={!song} aria-label="Следующий"><Icon name="next" /></button>
			<div class="seek"><SeekBar showTimes={false} /></div>
		</div>
	</div>
</div>

<style>
	.mini {
		display: flex;
		align-items: center;
		gap: 14px;
		height: 100vh;
		padding: 14px;
		background: rgb(14 14 18 / 0.6);
	}

	.art {
		flex: none;
		height: 100%;
		aspect-ratio: 1;
		border-radius: 10px;
		overflow: hidden;
		pointer-events: none;
	}

	.art.round {
		border-radius: 50%;
	}

	.body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		height: 100%;
	}

	.top {
		display: flex;
		align-items: flex-start;
		gap: 6px;
	}

	.text {
		flex: 1;
		min-width: 0;
		pointer-events: none;
	}

	.title {
		font-weight: 650;
	}

	.artist {
		font-size: 12.5px;
		color: var(--muted);
	}

	.controls {
		display: flex;
		align-items: center;
		gap: 2px;
	}

	.play {
		color: var(--text);
	}

	.seek {
		flex: 1;
		margin-left: 8px;
	}
</style>
