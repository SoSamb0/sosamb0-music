<script lang="ts">
	import { formatTime } from '$lib/app.svelte';
	import { player } from '$lib/player.svelte';

	let { showTimes = true }: { showTimes?: boolean } = $props();

	let track = $state<HTMLElement>();
	let now = $state(0);
	let drag = $state<number | null>(null);
	let hover = $state<number | null>(null);

	// A slow timer plus a CSS transition gives smooth motion for almost no CPU.
	$effect(() => {
		now = player.livePosition();
		const id = setInterval(() => (now = player.livePosition()), 250);
		return () => clearInterval(id);
	});

	$effect(() => {
		// Jump straight to a new position after a seek or track change.
		player.position;
		now = player.livePosition();
	});

	const duration = $derived(player.duration || 0);
	const shown = $derived(drag ?? now);
	const fraction = $derived(duration > 0 ? Math.min(1, shown / duration) : 0);

	function at(e: PointerEvent): number {
		const rect = track!.getBoundingClientRect();
		return Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width)) * duration;
	}

	function down(e: PointerEvent) {
		if (!duration) return;
		track!.setPointerCapture(e.pointerId);
		drag = at(e);
	}

	function move(e: PointerEvent) {
		hover = at(e);
		if (drag !== null) drag = at(e);
	}

	function up() {
		if (drag === null) return;
		player.seek(drag);
		now = drag;
		drag = null;
	}

	function key(e: KeyboardEvent) {
		if (e.key === 'ArrowRight') player.seek(now + 5);
		else if (e.key === 'ArrowLeft') player.seek(now - 5);
		else return;
		e.preventDefault();
		e.stopPropagation();
	}
</script>

<div class="seek">
	{#if showTimes}<span class="time tnum">{formatTime(shown)}</span>{/if}
	<div
		class="track"
		class:dragging={drag !== null}
		bind:this={track}
		onpointerdown={down}
		onpointermove={move}
		onpointerup={up}
		onpointercancel={() => (drag = null)}
		onpointerleave={() => (hover = null)}
		onkeydown={key}
		role="slider"
		tabindex="0"
		aria-label="Позиция"
		aria-valuemin={0}
		aria-valuemax={Math.round(duration)}
		aria-valuenow={Math.round(shown)}
		aria-valuetext={formatTime(shown)}
	>
		<div class="rail">
			<div class="buffer" style:width="{player.buffered * 100}%"></div>
			<div class="fill" style:width="{fraction * 100}%"></div>
		</div>
		<div class="thumb" style:left="{fraction * 100}%"></div>
		{#if hover !== null && duration}
			<div class="tip tnum" style:left="{(hover / duration) * 100}%">{formatTime(hover)}</div>
		{/if}
	</div>
	{#if showTimes}<span class="time tnum right">{formatTime(duration)}</span>{/if}
</div>

<style>
	.seek {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
	}

	.time {
		width: 42px;
		font-size: 12px;
		color: var(--muted);
		text-align: right;
	}

	.time.right {
		text-align: left;
	}

	.track {
		position: relative;
		flex: 1;
		height: 18px;
		display: flex;
		align-items: center;
		cursor: pointer;
		touch-action: none;
	}

	.rail {
		position: relative;
		width: 100%;
		height: 4px;
		border-radius: 999px;
		background: var(--surface-strong);
		overflow: hidden;
		transition: height 0.15s var(--ease);
	}

	.track:hover .rail,
	.dragging .rail {
		height: 6px;
	}

	.buffer,
	.fill {
		position: absolute;
		inset: 0 auto 0 0;
		border-radius: 999px;
	}

	.buffer {
		background: rgb(255 255 255 / 0.14);
		transition: width 0.4s linear;
	}

	.fill {
		background: var(--accent);
		transition: width 0.25s linear;
	}

	.dragging .fill,
	.dragging .thumb {
		transition: none;
	}

	.thumb {
		position: absolute;
		top: 50%;
		width: 12px;
		height: 12px;
		margin: -6px 0 0 -6px;
		border-radius: 50%;
		background: #fff;
		box-shadow: 0 1px 5px rgb(0 0 0 / 0.5);
		opacity: 0;
		transition:
			opacity 0.15s,
			left 0.25s linear;
	}

	.track:hover .thumb,
	.dragging .thumb,
	.track:focus-visible .thumb {
		opacity: 1;
	}

	.tip {
		position: absolute;
		bottom: 20px;
		transform: translateX(-50%);
		padding: 3px 7px;
		border-radius: 6px;
		background: rgb(20 20 26 / 0.95);
		border: 1px solid var(--line);
		font-size: 11.5px;
		pointer-events: none;
	}
</style>
