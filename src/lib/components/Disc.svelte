<script lang="ts">
	import { player } from '$lib/player.svelte';
	import { settings } from '$lib/settings.svelte';
	import Cover from './Cover.svelte';

	// Album art drawn in the shape chosen in the settings: square, round, or a spinning record.
	let {
		id,
		size = 300,
		eager = false,
		small = false
	}: { id?: string; size?: number; eager?: boolean; small?: boolean } = $props();

	const look = $derived(settings.data.look);
	const shape = $derived(small && !look.vinylEverywhere && look.coverShape === 'vinyl' ? 'square' : look.coverShape);
</script>

<div
	class="disc {shape}"
	class:small
	class:spinning={shape === 'vinyl' && player.playing}
	style:--turn="{look.vinylSeconds}s"
>
	<div class="face">
		<Cover {id} {size} {eager} round={shape !== 'square'} />
		{#if shape === 'vinyl'}
			<div class="grooves" aria-hidden="true"></div>
			<div class="hole" aria-hidden="true"></div>
		{/if}
	</div>
</div>

<style>
	.disc {
		width: 100%;
		border-radius: inherit;
	}

	.disc.round,
	.disc.vinyl {
		border-radius: 50%;
	}

	.face {
		position: relative;
		border-radius: inherit;
		overflow: hidden;
	}

	.vinyl .face {
		/* Runs forever; pausing freezes it where it is, like lifting the needle. */
		animation: spin var(--turn) linear infinite;
		animation-play-state: paused;
		box-shadow:
			0 0 0 2px rgb(0 0 0 / 0.6),
			inset 0 0 0 1px rgb(255 255 255 / 0.06);
	}

	.vinyl.spinning .face {
		animation-play-state: running;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.grooves {
		position: absolute;
		inset: 0;
		border-radius: 50%;
		pointer-events: none;
		background:
			radial-gradient(circle, transparent 0 17%, rgb(0 0 0 / 0.55) 17.4% 18.4%, transparent 18.8%),
			repeating-radial-gradient(circle, rgb(0 0 0 / 0) 0 3px, rgb(0 0 0 / 0.16) 3px 4px),
			conic-gradient(from 30deg, rgb(255 255 255 / 0.13), transparent 12%, transparent 45%, rgb(255 255 255 / 0.1) 52%, transparent 62%);
		mix-blend-mode: multiply;
	}

	.small .grooves {
		background:
			radial-gradient(circle, transparent 0 17%, rgb(0 0 0 / 0.55) 18% 21%, transparent 22%),
			conic-gradient(from 30deg, rgb(255 255 255 / 0.15), transparent 15%, transparent 45%, rgb(255 255 255 / 0.12) 52%, transparent 65%);
	}

	.hole {
		position: absolute;
		left: 50%;
		top: 50%;
		width: 7%;
		height: 7%;
		min-width: 5px;
		min-height: 5px;
		border-radius: 50%;
		transform: translate(-50%, -50%);
		background: #0a0a0d;
		box-shadow: 0 0 0 2px rgb(255 255 255 / 0.12);
	}
</style>
