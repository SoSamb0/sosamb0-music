<script lang="ts" module>
	export type VizStyle = 'bars' | 'wave' | 'ring' | 'vu' | 'glow';
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import { player } from '$lib/player.svelte';
	import { useVisualizer, viz } from '$lib/viz';

	let { style, faint = false }: { style: VizStyle; faint?: boolean } = $props();

	let canvas = $state<HTMLCanvasElement>();

	// Displayed values ease towards the engine's, so motion stays smooth between frames.
	const BANDS = 48;
	const shown = new Float32Array(BANDS);
	const caps = new Float32Array(BANDS);
	const wave = new Float32Array(128);
	const level = [0, 0];
	const needle = [0, 0];

	onMount(() => {
		const release = useVisualizer();
		const ctx = canvas!.getContext('2d')!;
		let raf = 0;
		let accent = '139 124 246';
		let lastAccent = 0;
		let last = performance.now();

		const frame = (now: number) => {
			raf = requestAnimationFrame(frame);
			const dt = Math.min(0.1, (now - last) / 1000);
			last = now;
			if (now - lastAccent > 500) {
				accent = getComputedStyle(document.documentElement).getPropertyValue('--accent-rgb').trim() || accent;
				lastAccent = now;
			}

			const dpr = window.devicePixelRatio || 1;
			const w = canvas!.clientWidth;
			const h = canvas!.clientHeight;
			if (canvas!.width !== Math.round(w * dpr) || canvas!.height !== Math.round(h * dpr)) {
				canvas!.width = Math.round(w * dpr);
				canvas!.height = Math.round(h * dpr);
			}
			ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
			ctx.clearRect(0, 0, w, h);

			// Follow the engine; let everything fall back to rest when playback stops.
			const live = player.playing && viz.frame && now - viz.at < 300 ? viz.frame : null;
			const fall = Math.pow(0.02, dt);
			for (let i = 0; i < BANDS; i++) {
				const target = live ? live.bands[i] / 255 : 0;
				shown[i] = target > shown[i] ? shown[i] + (target - shown[i]) * Math.min(1, dt * 30) : shown[i] * fall + target * (1 - fall);
				caps[i] = Math.max(shown[i], caps[i] - dt * 0.5);
			}
			for (let i = 0; i < wave.length; i++) {
				const target = live ? live.wave[i] / 127 : 0;
				wave[i] += (target - wave[i]) * Math.min(1, dt * 20);
			}
			for (let c = 0; c < 2; c++) {
				level[c] = live ? live.level[c] : 0;
				// Analog needle: quick rise, slower fall.
				const k = level[c] > needle[c] ? dt * 12 : dt * 3;
				needle[c] += (level[c] - needle[c]) * Math.min(1, k);
			}

			const col = (a: number) => `rgb(${accent} / ${a})`;
			if (style === 'bars') drawBars(ctx, w, h, col);
			else if (style === 'wave') drawWave(ctx, w, h, col);
			else if (style === 'ring') drawRing(ctx, w, h, col);
			else if (style === 'vu') drawVu(ctx, w, h, col);
			else drawGlow(ctx, w, h, col);
		};
		raf = requestAnimationFrame(frame);
		return () => {
			cancelAnimationFrame(raf);
			release();
		};
	});

	type Col = (a: number) => string;

	function drawBars(ctx: CanvasRenderingContext2D, w: number, h: number, col: Col) {
		const gap = Math.max(2, w / BANDS / 4);
		const bw = (w - gap * (BANDS - 1)) / BANDS;
		const base = h * 0.78;
		const grad = ctx.createLinearGradient(0, base, 0, 0);
		grad.addColorStop(0, col(0.55));
		grad.addColorStop(1, col(1));
		for (let i = 0; i < BANDS; i++) {
			const x = i * (bw + gap);
			const bh = Math.max(2, shown[i] * base * 0.95);
			ctx.fillStyle = grad;
			roundRect(ctx, x, base - bh, bw, bh, Math.min(bw / 2, 4));
			// Peak caps.
			ctx.fillStyle = 'rgba(255,255,255,.75)';
			ctx.fillRect(x, base - caps[i] * base * 0.95 - 3, bw, 2);
			// Reflection.
			ctx.fillStyle = col(0.12);
			ctx.fillRect(x, base + 3, bw, bh * 0.25);
		}
	}

	function drawWave(ctx: CanvasRenderingContext2D, w: number, h: number, col: Col) {
		const mid = h / 2;
		const amp = h * 0.42;
		const path = new Path2D();
		for (let i = 0; i < wave.length; i++) {
			const x = (i / (wave.length - 1)) * w;
			const y = mid - wave[i] * amp;
			if (i === 0) path.moveTo(x, y);
			else {
				const px = ((i - 0.5) / (wave.length - 1)) * w;
				path.quadraticCurveTo(px, mid - wave[i - 1] * amp, x, y);
			}
		}
		ctx.lineWidth = 6;
		ctx.strokeStyle = col(0.18);
		ctx.stroke(path);
		ctx.lineWidth = 2.5;
		ctx.strokeStyle = col(1);
		ctx.stroke(path);
	}

	function drawRing(ctx: CanvasRenderingContext2D, w: number, h: number, col: Col) {
		const cx = w / 2;
		const cy = h / 2;
		const r = Math.min(w, h) * 0.24;
		const len = Math.min(w, h) * 0.22;
		const bass = (shown[0] + shown[1] + shown[2] + shown[3]) / 4;
		ctx.beginPath();
		ctx.arc(cx, cy, r * (0.92 + bass * 0.12), 0, Math.PI * 2);
		ctx.strokeStyle = col(0.35);
		ctx.lineWidth = 2;
		ctx.stroke();
		ctx.lineCap = 'round';
		ctx.lineWidth = Math.max(2, (Math.PI * 2 * r) / (BANDS * 2) - 3);
		// Mirror the spectrum so the ring is symmetric left/right.
		for (let k = 0; k < BANDS * 2; k++) {
			const i = k < BANDS ? k : BANDS * 2 - 1 - k;
			const a = (k / (BANDS * 2)) * Math.PI * 2 - Math.PI / 2;
			const l = 4 + shown[i] * len;
			ctx.strokeStyle = col(0.45 + shown[i] * 0.55);
			ctx.beginPath();
			ctx.moveTo(cx + Math.cos(a) * r, cy + Math.sin(a) * r);
			ctx.lineTo(cx + Math.cos(a) * (r + l), cy + Math.sin(a) * (r + l));
			ctx.stroke();
		}
	}

	function drawVu(ctx: CanvasRenderingContext2D, w: number, h: number, col: Col) {
		const meters = 2;
		const mw = Math.min((w - 24) / meters, h * 1.6);
		const mh = mw * 0.62;
		const top = (h - mh) / 2;
		// Line weights grow with the meter so a big one does not look spindly.
		const k = Math.max(1, mw / 240);
		for (let c = 0; c < meters; c++) {
			const x = (w - mw * meters - 24) / 2 + c * (mw + 24);
			ctx.fillStyle = 'rgba(255,255,255,.05)';
			roundRect(ctx, x, top, mw, mh, 14);
			const cx = x + mw / 2;
			const cy = top + mh * 0.92;
			const r = mh * 0.72;
			const from = Math.PI * 1.22;
			const to = Math.PI * 1.78;
			// Scale: normal zone, then the loud zone in white.
			ctx.lineWidth = 3 * k;
			ctx.strokeStyle = col(0.7);
			ctx.beginPath();
			ctx.arc(cx, cy, r, from, from + (to - from) * 0.78);
			ctx.stroke();
			ctx.strokeStyle = 'rgba(255,255,255,.9)';
			ctx.beginPath();
			ctx.arc(cx, cy, r, from + (to - from) * 0.78, to);
			ctx.stroke();
			for (let t = 0; t <= 10; t++) {
				const a = from + ((to - from) * t) / 10;
				const inner = r - (t % 5 === 0 ? 14 : 8) * k;
				ctx.strokeStyle = t >= 8 ? 'rgba(255,255,255,.8)' : col(0.6);
				ctx.lineWidth = (t % 5 === 0 ? 2.5 : 1.5) * k;
				ctx.beginPath();
				ctx.moveTo(cx + Math.cos(a) * inner, cy + Math.sin(a) * inner);
				ctx.lineTo(cx + Math.cos(a) * r, cy + Math.sin(a) * r);
				ctx.stroke();
			}
			const a = from + (to - from) * needle[c];
			ctx.strokeStyle = 'rgba(255,255,255,.95)';
			ctx.lineWidth = 2.5 * k;
			ctx.lineCap = 'round';
			ctx.beginPath();
			ctx.moveTo(cx, cy);
			ctx.lineTo(cx + Math.cos(a) * (r + 6), cy + Math.sin(a) * (r + 6));
			ctx.stroke();
			ctx.fillStyle = col(1);
			ctx.beginPath();
			ctx.arc(cx, cy, 7 * k, 0, Math.PI * 2);
			ctx.fill();
			ctx.fillStyle = 'rgba(255,255,255,.55)';
			ctx.font = `600 ${Math.round(12 * k)}px Segoe UI, sans-serif`;
			ctx.textAlign = 'center';
			ctx.fillText(c === 0 ? 'L' : 'R', cx, top + mh * 0.62);
		}
	}

	function drawGlow(ctx: CanvasRenderingContext2D, w: number, h: number, col: Col) {
		const avg = (a: number, b: number) => {
			let s = 0;
			for (let i = a; i < b; i++) s += shown[i];
			return s / (b - a);
		};
		const blobs = [
			{ e: avg(0, 8), x: 0.3, y: 0.62, r: 0.5 },
			{ e: avg(8, 24), x: 0.68, y: 0.38, r: 0.38 },
			{ e: avg(24, 48), x: 0.55, y: 0.72, r: 0.28 }
		];
		const t = performance.now() / 1000;
		for (const [k, b] of blobs.entries()) {
			const x = (b.x + Math.sin(t * 0.3 + k) * 0.04) * w;
			const y = (b.y + Math.cos(t * 0.25 + k * 2) * 0.04) * h;
			const r = Math.max(w, h) * b.r * (0.6 + b.e * 0.7);
			const g = ctx.createRadialGradient(x, y, 0, x, y, r);
			g.addColorStop(0, col(0.12 + b.e * 0.45));
			g.addColorStop(1, col(0));
			ctx.fillStyle = g;
			ctx.fillRect(0, 0, w, h);
		}
	}

	function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
		ctx.beginPath();
		ctx.roundRect(x, y, w, h, r);
		ctx.fill();
	}
</script>

<canvas bind:this={canvas} class:faint aria-hidden="true"></canvas>

<style>
	canvas {
		display: block;
		width: 100%;
		height: 100%;
	}

	canvas.faint {
		opacity: 0.55;
	}
</style>
