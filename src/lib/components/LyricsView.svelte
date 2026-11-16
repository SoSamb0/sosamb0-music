<script lang="ts">
	import { activeLine } from '$lib/lyrics';
	import { player } from '$lib/player.svelte';
	import Icon from './Icon.svelte';

	let { large = false }: { large?: boolean } = $props();

	let box = $state<HTMLElement>();
	let active = $state(-1);
	let userScrolledAt = 0;

	const lyrics = $derived(player.lyrics);

	// Poll the playback clock; cheap, and only while this view is on screen.
	$effect(() => {
		if (!lyrics?.synced) {
			active = -1;
			return;
		}
		const lines = lyrics.lines;
		const update = () => (active = activeLine(lines, player.livePosition() + player.lyricsOffset + 0.15));
		update();
		const id = setInterval(update, 120);
		return () => clearInterval(id);
	});

	// Follow the active line unless the reader has just scrolled away on purpose.
	$effect(() => {
		const i = active;
		if (!box || i < 0 || performance.now() - userScrolledAt < 4000) return;
		const line = box.querySelector<HTMLElement>(`[data-i="${i}"]`);
		if (!line) return;
		box.scrollTo({ top: line.offsetTop - box.clientHeight / 2 + line.offsetHeight / 2, behavior: 'smooth' });
	});

	$effect(() => {
		// New song: back to the top.
		lyrics;
		userScrolledAt = 0;
		box?.scrollTo({ top: 0 });
	});

	function seekTo(time: number | undefined) {
		if (time === undefined) return;
		userScrolledAt = 0;
		player.seek(Math.max(0, time - player.lyricsOffset));
	}
</script>

<div class="lyrics" class:large>
	{#if player.lyricsLoading}
		<div class="empty"><span class="spinner"></span></div>
	{:else if !lyrics || lyrics.lines.length === 0}
		<div class="empty">
			<Icon name="lyrics" size={30} />
			<p>{player.current ? 'Текст для этой песни не найден' : 'Включите музыку, и здесь появится текст'}</p>
		</div>
	{:else}
		<div
			class="scroll"
			role="group"
			aria-label="Текст песни"
			bind:this={box}
			onwheel={() => (userScrolledAt = performance.now())}
			ontouchmove={() => (userScrolledAt = performance.now())}
		>
			<div class="pad"></div>
			{#each lyrics.lines as line, i (i)}
				{#if lyrics.synced}
					<button class="line" class:active={i === active} class:past={i < active} data-i={i} onclick={() => seekTo(line.time)}>
						{line.text || '♪'}
					</button>
				{:else}
					<p class="line plain">{line.text}</p>
				{/if}
			{/each}
			<div class="pad"></div>
		</div>
		<div class="tools">
			<span class="faint">{lyrics.source === 'lrclib' ? 'LRCLIB' : 'С сервера'}{lyrics.synced ? '' : ' · без синхронизации'}</span>
			{#if lyrics.synced}
				<span class="offset">
					<button class="icon-btn" onclick={() => player.setLyricsOffset(player.lyricsOffset - 0.5)} aria-label="Текст позже" title="Текст позже">−</button>
					<button class="value tnum" onclick={() => player.setLyricsOffset(0)} title="Сбросить сдвиг">
						{player.lyricsOffset > 0 ? '+' : ''}{player.lyricsOffset.toFixed(1)} с
					</button>
					<button class="icon-btn" onclick={() => player.setLyricsOffset(player.lyricsOffset + 0.5)} aria-label="Текст раньше" title="Текст раньше">+</button>
				</span>
			{/if}
		</div>
	{/if}
</div>

<style>
	.lyrics {
		position: relative;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}

	.scroll {
		flex: 1;
		overflow-y: auto;
		padding: 0 22px;
		scrollbar-width: none;
		mask-image: linear-gradient(transparent, #000 14%, #000 86%, transparent);
	}

	.scroll::-webkit-scrollbar {
		display: none;
	}

	.pad {
		height: 38%;
	}

	.line {
		display: block;
		width: 100%;
		margin: 0;
		padding: 7px 0;
		text-align: left;
		font: 650 20px/1.35 'Segoe UI Variable Display', 'Segoe UI', system-ui, sans-serif;
		letter-spacing: -0.01em;
		color: rgb(255 255 255 / 0.32);
		transform-origin: left center;
		transition:
			color 0.35s,
			transform 0.35s var(--ease),
			filter 0.35s;
		user-select: text;
	}

	button.line:hover {
		color: rgb(255 255 255 / 0.7);
	}

	.line.past {
		color: rgb(255 255 255 / 0.22);
	}

	.line.active {
		color: #fff;
		transform: scale(1.04);
	}

	.line.plain {
		color: rgb(255 255 255 / 0.82);
		font-weight: 500;
		font-size: 16px;
		padding: 3px 0;
	}

	.large .line {
		font-size: 30px;
		padding: 10px 0;
	}

	.large .line.plain {
		font-size: 20px;
		padding: 4px 0;
	}

	.tools {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 14px 10px 22px;
		font-size: 12px;
	}

	.offset {
		display: inline-flex;
		align-items: center;
	}

	.offset .icon-btn {
		width: 26px;
		height: 26px;
		font-size: 16px;
	}

	.value {
		min-width: 54px;
		color: var(--muted);
	}

	.value:hover {
		color: var(--text);
	}
</style>
