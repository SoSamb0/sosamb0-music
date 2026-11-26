<script lang="ts">
	// The music's spectrum, live, drawn under the equaliser curve on the same 20 Hz – 20 kHz scale.
	// The engine measures it at the output, so the effect of the equaliser shows up in it.
	import { onMount } from 'svelte';
	import { player } from '$lib/player.svelte';
	import { useVisualizer, viz } from '$lib/viz';

	// Must match src-tauri/src/audio/analyzer.rs: 48 bands, log-spaced 30 Hz … 16 kHz.
	const BANDS = 48;
	const LO = 30;
	const HI = 16000;
	const xs = Array.from({ length: BANDS }, (_, b) => Math.log10((LO * (HI / LO) ** ((b + 0.5) / BANDS)) / 20) / 3);

	let canvas = $state<HTMLCanvasElement>();

	onMount(() => {
		const release = useVisualizer();
		const ctx = canvas!.getContext('2d')!;
		const shown = new Float32Array(BANDS);
		let raf = 0;
		let last = performance.now();
		let accent = '';
		let accentAt = 0;
		let resting = false;

		const frame = (now: number) => {
			raf = requestAnimationFrame(frame);
			const dt = Math.min(0.1, (now - last) / 1000);
			last = now;

			const live = player.playing && viz.frame && now - viz.at < 300 ? viz.frame : null;
			const fall = Math.pow(0.05, dt);
			let sum = 0;
			for (let i = 0; i < BANDS; i++) {
				const target = live ? live.bands[i] / 255 : 0;
				shown[i] = target > shown[i] ? shown[i] + (target - shown[i]) * Math.min(1, dt * 18) : shown[i] * fall + target * (1 - fall);
				sum += shown[i];
			}
			// Nothing playing and everything has settled: skip drawing until music comes back.
			if (sum < 0.01 && resting) return;
			resting = sum < 0.01;

			const dpr = window.devicePixelRatio || 1;
			const w = canvas!.clientWidth;
			const h = canvas!.clientHeight;
			if (canvas!.width !== Math.round(w * dpr) || canvas!.height !== Math.round(h * dpr)) {
				canvas!.width = Math.round(w * dpr);
				canvas!.height = Math.round(h * dpr);
			}
			// The accent follows the cover; re-read it now and then rather than every frame.
			if (now - accentAt > 500) {
				accent = getComputedStyle(canvas!).getPropertyValue('--accent-rgb').trim() || '90 169 255';
				accentAt = now;
			}
			ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
			ctx.clearRect(0, 0, w, h);

			// A smooth outline through the band tops (midpoints joined with quadratic curves).
			const pts = xs.map((x, i) => [x * w, h - shown[i] * h * 0.92] as const);
			ctx.beginPath();
			ctx.moveTo(0, h);
			ctx.lineTo(0, pts[0][1]);
			for (let i = 0; i < pts.length - 1; i++) {
				const mx = (pts[i][0] + pts[i + 1][0]) / 2;
				const my = (pts[i][1] + pts[i + 1][1]) / 2;
				ctx.quadraticCurveTo(pts[i][0], pts[i][1], mx, my);
			}
			ctx.lineTo(w, pts[pts.length - 1][1]);
			ctx.lineTo(w, h);
			ctx.closePath();
			const grad = ctx.createLinearGradient(0, 0, 0, h);
			grad.addColorStop(0, `rgb(${accent} / 0.32)`);
			grad.addColorStop(1, `rgb(${accent} / 0.04)`);
			ctx.fillStyle = grad;
			ctx.fill();
		};
		raf = requestAnimationFrame(frame);
		return () => {
			cancelAnimationFrame(raf);
			release();
		};
	});
</script>

<canvas bind:this={canvas} aria-hidden="true"></canvas>

<style>
	canvas {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
</style>
