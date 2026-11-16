import { invoke } from './tauri';
import type { ServerConfig } from './subsonic';

export type BandKind = 'peak' | 'lowshelf' | 'highshelf' | 'lowpass' | 'highpass' | 'notch';

export interface EqBand {
	kind: BandKind;
	freq: number;
	gainDb: number;
	q: number;
	enabled: boolean;
}

export interface DspConfig {
	enabled: boolean;
	preampDb: number;
	eqEnabled: boolean;
	bands: EqBand[];
	bassDb: number;
	crossfeed: number;
	width: number;
	limiter: boolean;
}

export const GRAPHIC_FREQS = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

export const EQ_PRESETS: Record<string, number[]> = {
	Ровно: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
	'Больше баса': [6, 5, 3.5, 1.5, 0, 0, 0, 0, 0, 0],
	'Глубокий бас': [7, 5, 2, 0, -1, -1, 0, 0, 0, 0],
	Вокал: [-2, -1.5, -0.5, 1, 2.5, 3, 2.5, 1.5, 0, -0.5],
	Яркость: [0, 0, 0, 0, 0, 0.5, 1.5, 3, 4.5, 5],
	Рок: [4, 3, 1.5, -0.5, -1.5, -0.5, 1.5, 3, 4, 4],
	Электроника: [5, 4, 1.5, 0, -1.5, 1, 0.5, 1.5, 3.5, 4.5],
	Классика: [3, 2, 1, 0, 0, 0, -0.5, 1, 2, 2.5],
	Джаз: [2.5, 1.5, 0.5, 1.5, -1, -1, 0, 1, 2, 2.5],
	Громкость: [5, 3.5, 1.5, 0, -1, -1, 0, 1.5, 3.5, 4.5]
};

export function graphicBands(gains: number[] = EQ_PRESETS['Ровно']): EqBand[] {
	return GRAPHIC_FREQS.map((freq, i) => ({ kind: 'peak', freq, gainDb: gains[i] ?? 0, q: 1.41, enabled: true }));
}

interface Persisted {
	server: ServerConfig | null;
	demo: boolean;
	volume: number;
	output: { deviceId: string | null; exclusive: boolean };
	options: {
		crossfadeSecs: number;
		replayGain: 'off' | 'track' | 'album';
		replayGainPreampDb: number;
		preventClipping: boolean;
		cacheLimitMb: number;
	};
	dsp: DspConfig;
	eqPreset: string;
	lrclib: boolean;
	closeToTray: boolean;
	hotkeys: boolean;
	scrobble: boolean;
	discordAppId: string;
}

const KEY = 'lumen.settings.v1';

function defaults(): Persisted {
	return {
		server: null,
		demo: false,
		volume: 1,
		output: { deviceId: null, exclusive: false },
		options: { crossfadeSecs: 0, replayGain: 'off', replayGainPreampDb: 0, preventClipping: true, cacheLimitMb: 2048 },
		dsp: {
			enabled: false,
			preampDb: 0,
			eqEnabled: true,
			bands: graphicBands(),
			bassDb: 0,
			crossfeed: 0,
			width: 1,
			limiter: true
		},
		eqPreset: 'Ровно',
		lrclib: true,
		closeToTray: false,
		hotkeys: false,
		scrobble: true,
		discordAppId: ''
	};
}

function load(): Persisted {
	const base = defaults();
	try {
		const saved = JSON.parse(localStorage.getItem(KEY) ?? '{}');
		return {
			...base,
			...saved,
			output: { ...base.output, ...saved.output },
			options: { ...base.options, ...saved.options },
			dsp: { ...base.dsp, ...saved.dsp }
		};
	} catch {
		return base;
	}
}

class Settings {
	data = $state<Persisted>(load());
	private saveTimer: ReturnType<typeof setTimeout> | undefined;

	/** Persist soon. Call after mutating `data`. */
	save() {
		clearTimeout(this.saveTimer);
		this.saveTimer = setTimeout(() => {
			try {
				localStorage.setItem(KEY, JSON.stringify(this.data));
			} catch {
				// Storage unavailable: settings last for this session only.
			}
		}, 150);
	}

	/** Volume slider position (0..1) to linear gain. Cubic feels even to the ear; 1 stays exactly 1. */
	gain(volume = this.data.volume): number {
		return volume >= 1 ? 1 : volume ** 3;
	}

	pushDsp() {
		this.save();
		return invoke('player_set_dsp', { config: $state.snapshot(this.data.dsp) });
	}

	pushOutput() {
		this.save();
		return invoke('player_set_output', { config: $state.snapshot(this.data.output) });
	}

	pushOptions() {
		this.save();
		return invoke('player_set_options', { options: $state.snapshot(this.data.options) });
	}

	pushVolume() {
		this.save();
		return invoke('player_set_volume', { volume: this.gain() });
	}

	/** Send every audio setting to the engine (at startup). */
	async pushAll() {
		await this.pushOutput();
		await this.pushOptions();
		await this.pushDsp();
		await this.pushVolume();
	}

	applyPreset(name: string) {
		const gains = EQ_PRESETS[name];
		if (!gains) return;
		this.data.eqPreset = name;
		this.data.dsp.bands = graphicBands(gains);
		// Leave headroom for the largest boost so the limiter has nothing to do.
		this.data.dsp.preampDb = -Math.max(0, ...gains);
		this.data.dsp.eqEnabled = true;
		if (name !== 'Ровно') this.data.dsp.enabled = true;
		this.pushDsp();
	}

	/**
	 * Import a headphone correction profile in the AutoEQ / Equalizer APO "ParametricEQ.txt"
	 * format. Returns the number of filters read.
	 */
	importAutoEq(text: string): number {
		const bands: EqBand[] = [];
		let preamp = 0;
		for (const line of text.split(/\r?\n/)) {
			const p = line.match(/^\s*Preamp:\s*(-?[\d.]+)\s*dB/i);
			if (p) {
				preamp = Number(p[1]);
				continue;
			}
			const f = line.match(/^\s*Filter\s*\d*:\s*(ON|OFF)\s+(PK|PEQ|LSC?|HSC?|LSQ|HSQ|LP|LPQ|HP|HPQ|NO)\s+Fc\s+([\d.]+)\s*Hz(?:\s+Gain\s+(-?[\d.]+)\s*dB)?(?:\s+Q\s+([\d.]+))?/i);
			if (!f) continue;
			const type = f[2].toUpperCase();
			const kind: BandKind = type.startsWith('LS') ? 'lowshelf' : type.startsWith('HS') ? 'highshelf' : type.startsWith('LP') ? 'lowpass' : type.startsWith('HP') ? 'highpass' : type === 'NO' ? 'notch' : 'peak';
			bands.push({ kind, freq: Number(f[3]), gainDb: Number(f[4] ?? 0), q: Number(f[5] ?? 0.707), enabled: f[1].toUpperCase() === 'ON' });
		}
		if (bands.length === 0) return 0;
		this.data.dsp.bands = bands;
		this.data.dsp.preampDb = preamp;
		this.data.dsp.eqEnabled = true;
		this.data.dsp.enabled = true;
		this.data.eqPreset = 'Профиль наушников';
		this.pushDsp();
		return bands.length;
	}
}

export const settings = new Settings();
