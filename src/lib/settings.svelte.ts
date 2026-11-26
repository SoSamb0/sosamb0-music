import { response } from './eqmath';
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

/** An equaliser curve the user saved under their own name. */
export interface UserPreset {
	name: string;
	bands: EqBand[];
	preampDb: number;
}

/** The sound settings remembered for one output device. */
export interface DeviceProfile {
	dsp: DspConfig;
	preset: string;
}

export interface Look {
	/** Background of the full-screen player. */
	npBackground: 'blur' | 'motion' | 'dim' | 'solid';
	coverShape: 'square' | 'round' | 'vinyl';
	/** Seconds per turn of the spinning record. */
	vinylSeconds: number;
	/** Spin the small cover in the player bar and mini player too. */
	vinylEverywhere: boolean;
	/** 'cover' takes the colour from the album art; otherwise a fixed hex colour. */
	accent: string;
	appBackdrop: boolean;
	/** Opacity of the blurred cover behind the whole app, 0..1. */
	backdropStrength: number;
	/** How strongly the cover shows through the full player's background (0.2..1). */
	npStrength: number;
	/** Blur of the cover behind the full player, in px (0 = sharp). */
	npBlur: number;
	npLyrics: boolean;
	lyricsSize: 's' | 'm' | 'l';
	lyricsAlign: 'left' | 'center';
	scale: number;
	animations: boolean;
	/** Visualiser in the full-screen player. */
	vizStyle: 'off' | 'bars' | 'wave' | 'ring' | 'vu' | 'glow';
	/** Where: instead of the lyrics, under them, or behind everything. */
	vizPlace: 'replace' | 'below' | 'background';
}

/**
 * The "SoSamb0 Music" application registered on discord.com/developers. An application id is a public
 * identifier (it appears in every status it shows), so shipping it in the app is safe.
 */
export const DEFAULT_DISCORD_APP_ID = '1571966119789465711';

export const ACCENTS = ['#5aa9ff', '#8b7cf6', '#2dd4bf', '#4ade80', '#fbbf24', '#fb923c', '#fb7185', '#e879f9', '#e5e7eb'];

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
	/** Keep the preamp just low enough that the curve never rises above 0 dB. */
	eqAutoPreamp: boolean;
	userPresets: UserPreset[];
	/** Remember the sound settings separately for each output device. */
	eqPerDevice: boolean;
	/** By device name as the engine reports it. */
	deviceProfiles: Record<string, DeviceProfile>;
	lrclib: boolean;
	closeToTray: boolean;
	hotkeys: boolean;
	scrobble: boolean;
	discordAppId: string;
	discord: {
		enabled: boolean;
		/** Own Discord application id; empty uses SoSamb0 Music's. */
		appId: string;
		/** Find the cover in public catalogues so Discord can show it. */
		covers: boolean;
		hidePaused: boolean;
	};
	cache: {
		/** Download the queue in the background. */
		prefetch: boolean;
		/** How many upcoming tracks to cache; 0 means the whole queue. */
		ahead: number;
	};
	look: Look;
	home: HomeLayout;
}

/** Sections of the home screen, in the order the user chose; hidden ones stay in the list. */
export type HomeSection = 'pinned' | 'mixes' | 'genres' | 'newest' | 'recent' | 'frequent' | 'random' | 'playlists' | 'smart';

export interface HomeLayout {
	order: HomeSection[];
	hidden: HomeSection[];
	/** Quick-access tiles: playlist ids, or "smart:<kind>" for smart playlists. */
	pinned: string[];
}

export const HOME_SECTIONS: Record<HomeSection, string> = {
	pinned: 'Закреплённое',
	mixes: 'Миксы',
	genres: 'Миксы по жанрам',
	newest: 'Недавно добавленные',
	recent: 'Вы недавно слушали',
	frequent: 'Слушаете чаще всего',
	random: 'Случайная подборка',
	playlists: 'Ваши плейлисты',
	smart: 'Умные плейлисты'
};

const HOME_ORDER: HomeSection[] = ['pinned', 'mixes', 'genres', 'newest', 'recent', 'playlists', 'frequent', 'smart', 'random'];

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
		eqAutoPreamp: true,
		userPresets: [],
		eqPerDevice: false,
		deviceProfiles: {},
		lrclib: true,
		closeToTray: false,
		hotkeys: false,
		scrobble: true,
		discordAppId: '',
		discord: { enabled: false, appId: '', covers: true, hidePaused: false },
		cache: { prefetch: true, ahead: 0 },
		look: {
			npBackground: 'blur',
			coverShape: 'square',
			vinylSeconds: 12,
			vinylEverywhere: true,
			accent: 'cover',
			appBackdrop: true,
			backdropStrength: 0.42,
			npStrength: 0.7,
			npBlur: 60,
			npLyrics: true,
			lyricsSize: 'm',
			lyricsAlign: 'left',
			scale: 1,
			animations: true,
			vizStyle: 'off',
			vizPlace: 'below'
		},
		home: { order: [...HOME_ORDER], hidden: [], pinned: [] }
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
			dsp: { ...base.dsp, ...saved.dsp },
			cache: { ...base.cache, ...saved.cache },
			discord: { ...base.discord, ...saved.discord },
			look: { ...base.look, ...saved.look },
			home: homeLayout(saved.home)
		};
	} catch {
		return base;
	}
}

/** A saved home layout, with sections added in newer versions appended. */
function homeLayout(saved: Partial<HomeLayout> | undefined): HomeLayout {
	const order = (saved?.order ?? []).filter((s) => s in HOME_SECTIONS);
	for (const s of HOME_ORDER) if (!order.includes(s)) order.push(s);
	return { order, hidden: saved?.hidden ?? [], pinned: saved?.pinned ?? [] };
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

	isPinned(id: string): boolean {
		return this.data.home.pinned.includes(id);
	}

	/** Put a playlist on the home screen's quick-access tiles, or take it off. */
	togglePin(id: string) {
		const pinned = this.data.home.pinned;
		this.data.home.pinned = pinned.includes(id) ? pinned.filter((p) => p !== id) : [...pinned, id];
		this.save();
	}

	/** Volume slider position (0..1) to linear gain. Cubic feels even to the ear; 1 stays exactly 1. */
	gain(volume = this.data.volume): number {
		return volume >= 1 ? 1 : volume ** 3;
	}

	pushDsp() {
		const dsp = this.data.dsp;
		if (this.data.eqAutoPreamp) dsp.preampDb = autoPreamp(dsp);
		if (this.data.eqPerDevice && this.device) {
			this.data.deviceProfiles[this.device] = { dsp: $state.snapshot(dsp), preset: this.data.eqPreset };
		}
		this.save();
		return invoke('player_set_dsp', { config: $state.snapshot(dsp) });
	}

	/** Output device the engine is playing on now (its name), or null before the first track. */
	device = $state<string | null>(null);

	/**
	 * The engine moved to another output device: bring up that device's own sound settings.
	 * Returns true when settings were switched (not on the first device after start).
	 */
	deviceChanged(name: string): boolean {
		if (!name || name === this.device) return false;
		const first = this.device === null;
		this.device = name;
		if (!this.data.eqPerDevice) return false;
		const saved = this.data.deviceProfiles[name];
		if (!saved) {
			// A device seen for the first time starts from the current settings.
			this.data.deviceProfiles[name] = { dsp: $state.snapshot(this.data.dsp), preset: this.data.eqPreset };
			this.save();
			return false;
		}
		this.data.dsp = { ...defaults().dsp, ...$state.snapshot(saved.dsp) };
		this.data.eqPreset = saved.preset;
		this.pushDsp();
		return !first;
	}

	setPerDevice(on: boolean) {
		this.data.eqPerDevice = on;
		if (on && this.device) this.data.deviceProfiles[this.device] = { dsp: $state.snapshot(this.data.dsp), preset: this.data.eqPreset };
		this.save();
	}

	forgetDevice(name: string) {
		delete this.data.deviceProfiles[name];
		this.save();
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
	/** Latest connection report from the Discord integration. */
	discordStatus = $state<{ connected: boolean; message: string | null; cover: string | null } | null>(null);

	pushDiscord() {
		this.save();
		const d = this.data.discord;
		return invoke('discord_configure', {
			config: {
				enabled: d.enabled && !this.data.demo,
				appId: d.appId.trim() || DEFAULT_DISCORD_APP_ID,
				covers: d.covers,
				hidePaused: d.hidePaused
			}
		});
	}

	async pushAll() {
		await this.pushDiscord();
		await this.pushOutput();
		await this.pushOptions();
		await this.pushDsp();
		await this.pushVolume();
	}

	applyPreset(name: string) {
		const own = this.data.userPresets.find((p) => p.name === name);
		const gains = EQ_PRESETS[name];
		if (own) {
			this.data.dsp.bands = $state.snapshot(own.bands);
			this.data.dsp.preampDb = own.preampDb;
		} else if (gains) {
			this.data.dsp.bands = graphicBands(gains);
			// Leave headroom for the largest boost so the limiter has nothing to do.
			this.data.dsp.preampDb = -Math.max(0, ...gains);
		} else return;
		this.data.eqPreset = name;
		this.data.dsp.eqEnabled = true;
		if (name !== 'Ровно') this.data.dsp.enabled = true;
		this.pushDsp();
	}

	isUserPreset(name: string): boolean {
		return this.data.userPresets.some((p) => p.name === name);
	}

	/** Why `name` cannot be used for a new preset, or null if it can. */
	presetNameProblem(name: string, except?: string): string | null {
		if (!name) return 'Введите название';
		if (name in EQ_PRESETS) return 'Так называется встроенный пресет';
		if (name !== except && this.isUserPreset(name)) return 'Пресет с таким названием уже есть';
		return null;
	}

	/** Save the current curve as a preset of the user's own (replacing one of the same name). */
	saveUserPreset(name: string) {
		const preset: UserPreset = { name, bands: $state.snapshot(this.data.dsp.bands), preampDb: this.data.dsp.preampDb };
		const list = this.data.userPresets.filter((p) => p.name !== name);
		this.data.userPresets = [...list, preset].sort((a, b) => a.name.localeCompare(b.name, 'ru'));
		this.data.eqPreset = name;
		this.save();
	}

	renameUserPreset(from: string, to: string) {
		const p = this.data.userPresets.find((x) => x.name === from);
		if (!p) return;
		p.name = to;
		this.data.userPresets.sort((a, b) => a.name.localeCompare(b.name, 'ru'));
		if (this.data.eqPreset === from) this.data.eqPreset = to;
		this.save();
	}

	deleteUserPreset(name: string) {
		this.data.userPresets = this.data.userPresets.filter((p) => p.name !== name);
		if (this.data.eqPreset === name) this.data.eqPreset = 'Свой';
		this.save();
	}

	/**
	 * Import a headphone correction profile in the AutoEQ / Equalizer APO "ParametricEQ.txt"
	 * format. Returns the number of filters read.
	 */
	importAutoEq(text: string, name = 'Профиль наушников'): number {
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
		this.data.eqPreset = name;
		this.pushDsp();
		return bands.length;
	}
}

/** Preamp that brings the highest point of the curve (equaliser and bass) down to 0 dB. */
export function autoPreamp(dsp: DspConfig): number {
	const peak = Math.max(0, ...response({ ...dsp, preampDb: 0 }, AUTO_FREQS));
	// A tenth of a dB below the peak, so rounding can never leave it above 0 dB.
	return -Math.ceil(peak * 10) / 10;
}

const AUTO_FREQS = Array.from({ length: 240 }, (_, i) => 20 * 1000 ** (i / 239));

const APO_TYPES: Record<BandKind, string> = { peak: 'PK', lowshelf: 'LSC', highshelf: 'HSC', lowpass: 'LP', highpass: 'HP', notch: 'NO' };

/** The current equaliser (and bass shelf) as an Equalizer APO / AutoEQ "ParametricEQ.txt". */
export function toApo(dsp: DspConfig): string {
	const lines = [`Preamp: ${dsp.preampDb.toFixed(1)} dB`];
	const bands = dsp.eqEnabled ? [...dsp.bands] : [];
	if (Math.abs(dsp.bassDb) > 1e-6) bands.push({ kind: 'lowshelf', freq: 90, gainDb: dsp.bassDb, q: 0.7, enabled: true });
	bands.forEach((b, i) => {
		const gain = b.kind === 'peak' || b.kind === 'lowshelf' || b.kind === 'highshelf' ? ` Gain ${b.gainDb.toFixed(1)} dB` : '';
		lines.push(`Filter ${i + 1}: ${b.enabled ? 'ON' : 'OFF'} ${APO_TYPES[b.kind]} Fc ${Math.round(b.freq)} Hz${gain} Q ${b.q.toFixed(2)}`);
	});
	return lines.map((l) => `${l}\r\n`).join('');
}

export const settings = new Settings();
