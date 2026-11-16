// Derive an accent colour from album art so the interface takes on the mood of the music.

export const DEFAULT_ACCENT: [number, number, number] = [139, 124, 246];

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
	r /= 255;
	g /= 255;
	b /= 255;
	const max = Math.max(r, g, b);
	const min = Math.min(r, g, b);
	const l = (max + min) / 2;
	const d = max - min;
	if (d === 0) return [0, 0, l];
	const s = d / (1 - Math.abs(2 * l - 1));
	let h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
	h = (h * 60 + 360) % 360;
	return [h, s, l];
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
	const c = (1 - Math.abs(2 * l - 1)) * s;
	const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
	const m = l - c / 2;
	const [r, g, b] = h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
	return [Math.round((r + m) * 255), Math.round((g + m) * 255), Math.round((b + m) * 255)];
}

/** Pick the dominant vivid hue from RGBA pixel data and return a readable accent colour. */
export function accentFromPixels(data: Uint8ClampedArray): [number, number, number] {
	const BUCKETS = 24;
	const weight = new Float64Array(BUCKETS);
	const hueX = new Float64Array(BUCKETS);
	const hueY = new Float64Array(BUCKETS);
	const sat = new Float64Array(BUCKETS);

	for (let i = 0; i < data.length; i += 4) {
		if (data[i + 3] < 128) continue;
		const [h, s, l] = rgbToHsl(data[i], data[i + 1], data[i + 2]);
		// Greys, near-blacks and near-whites carry no hue worth using.
		if (s < 0.18 || l < 0.12 || l > 0.92) continue;
		const w = s * (1 - Math.abs(l - 0.5));
		const b = Math.floor((h / 360) * BUCKETS) % BUCKETS;
		const rad = (h * Math.PI) / 180;
		weight[b] += w;
		hueX[b] += Math.cos(rad) * w;
		hueY[b] += Math.sin(rad) * w;
		sat[b] += s * w;
	}

	let best = -1;
	for (let b = 0; b < BUCKETS; b++) if (weight[b] > 0 && (best < 0 || weight[b] > weight[best])) best = b;
	if (best < 0) return DEFAULT_ACCENT;

	const hue = ((Math.atan2(hueY[best], hueX[best]) * 180) / Math.PI + 360) % 360;
	const saturation = Math.min(0.85, Math.max(0.5, sat[best] / weight[best]));
	// Fixed lightness keeps text and controls legible on the dark background for any cover.
	return hslToRgb(hue, saturation, 0.68);
}

const cache = new Map<string, [number, number, number]>();

export async function accentFromImage(url: string): Promise<[number, number, number]> {
	if (!url) return DEFAULT_ACCENT;
	const known = cache.get(url);
	if (known) return known;
	try {
		const image = new Image();
		image.crossOrigin = 'anonymous';
		image.decoding = 'async';
		image.src = url;
		await image.decode();
		const size = 32;
		const canvas = document.createElement('canvas');
		canvas.width = canvas.height = size;
		const ctx = canvas.getContext('2d', { willReadFrequently: true });
		if (!ctx) return DEFAULT_ACCENT;
		ctx.drawImage(image, 0, 0, size, size);
		const accent = accentFromPixels(ctx.getImageData(0, 0, size, size).data);
		cache.set(url, accent);
		return accent;
	} catch {
		// Cross-origin image without CORS headers, or a broken cover.
		return DEFAULT_ACCENT;
	}
}
