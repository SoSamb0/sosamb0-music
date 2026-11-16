// Frequency response of the equaliser, for drawing the curve. Mirrors src-tauri/src/audio/dsp.rs.

import type { DspConfig, EqBand } from './settings.svelte';

const RATE = 48000;

function coeffs(band: EqBand) {
	const freq = Math.min(Math.max(band.freq, 10), RATE * 0.49);
	const q = Math.min(Math.max(band.q, 0.05), 30);
	const a = 10 ** (band.gainDb / 40);
	const w0 = (2 * Math.PI * freq) / RATE;
	const cos = Math.cos(w0);
	const alpha = Math.sin(w0) / (2 * q);
	const k = 2 * Math.sqrt(a) * alpha;
	let b0: number, b1: number, b2: number, a0: number, a1: number, a2: number;
	switch (band.kind) {
		case 'lowshelf':
			b0 = a * (a + 1 - (a - 1) * cos + k);
			b1 = 2 * a * (a - 1 - (a + 1) * cos);
			b2 = a * (a + 1 - (a - 1) * cos - k);
			a0 = a + 1 + (a - 1) * cos + k;
			a1 = -2 * (a - 1 + (a + 1) * cos);
			a2 = a + 1 + (a - 1) * cos - k;
			break;
		case 'highshelf':
			b0 = a * (a + 1 + (a - 1) * cos + k);
			b1 = -2 * a * (a - 1 + (a + 1) * cos);
			b2 = a * (a + 1 + (a - 1) * cos - k);
			a0 = a + 1 - (a - 1) * cos + k;
			a1 = 2 * (a - 1 - (a + 1) * cos);
			a2 = a + 1 - (a - 1) * cos - k;
			break;
		case 'lowpass':
			b0 = (1 - cos) / 2;
			b1 = 1 - cos;
			b2 = (1 - cos) / 2;
			a0 = 1 + alpha;
			a1 = -2 * cos;
			a2 = 1 - alpha;
			break;
		case 'highpass':
			b0 = (1 + cos) / 2;
			b1 = -(1 + cos);
			b2 = (1 + cos) / 2;
			a0 = 1 + alpha;
			a1 = -2 * cos;
			a2 = 1 - alpha;
			break;
		case 'notch':
			b0 = 1;
			b1 = -2 * cos;
			b2 = 1;
			a0 = 1 + alpha;
			a1 = -2 * cos;
			a2 = 1 - alpha;
			break;
		default:
			b0 = 1 + alpha * a;
			b1 = -2 * cos;
			b2 = 1 - alpha * a;
			a0 = 1 + alpha / a;
			a1 = -2 * cos;
			a2 = 1 - alpha / a;
	}
	return [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0];
}

function bandDb([b0, b1, b2, a1, a2]: number[], freq: number): number {
	const w = (2 * Math.PI * freq) / RATE;
	const c1 = Math.cos(w);
	const c2 = Math.cos(2 * w);
	const num = b0 * b0 + b1 * b1 + b2 * b2 + 2 * (b0 * b1 + b1 * b2) * c1 + 2 * b0 * b2 * c2;
	const den = 1 + a1 * a1 + a2 * a2 + 2 * (a1 + a1 * a2) * c1 + 2 * a2 * c2;
	return 10 * Math.log10(Math.max(num / den, 1e-12));
}

function audible(band: EqBand): boolean {
	if (!band.enabled) return false;
	return band.kind === 'peak' || band.kind === 'lowshelf' || band.kind === 'highshelf' ? Math.abs(band.gainDb) > 1e-6 : true;
}

/** Gain in dB of the whole chain at each of `freqs`. */
export function response(dsp: DspConfig, freqs: number[]): number[] {
	const active: number[][] = [];
	if (dsp.eqEnabled) for (const band of dsp.bands) if (audible(band)) active.push(coeffs(band));
	if (Math.abs(dsp.bassDb) > 1e-6) active.push(coeffs({ kind: 'lowshelf', freq: 90, gainDb: dsp.bassDb, q: 0.7, enabled: true }));
	return freqs.map((f) => dsp.preampDb + active.reduce((sum, c) => sum + bandDb(c, f), 0));
}

/** `count` frequencies spaced evenly on a log scale from 20 Hz to 20 kHz. */
export function logFreqs(count: number): number[] {
	return Array.from({ length: count }, (_, i) => 20 * 1000 ** (i / (count - 1)));
}
