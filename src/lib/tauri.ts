// Thin bridge to the native side. Outside Tauri (a plain browser, or demo mode) the audio
// commands are answered by a simulated transport so the whole interface stays usable.

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export interface SourceInfo {
	codec: string;
	rate: number;
	channels: number;
	bits: number | null;
	lossless: boolean;
	duration: number | null;
}

export interface DeviceFormat {
	deviceName: string;
	rate: number;
	channels: number;
	bits: number;
	float: boolean;
	exclusive: boolean;
	note: string | null;
}

export interface SignalPath {
	source: SourceInfo;
	device: DeviceFormat;
	resampled: boolean;
	replayGainDb: number;
	dspActive: boolean;
	volumeUnity: boolean;
	dithered: boolean;
	bitPerfect: boolean;
}

export type PlayState = 'stopped' | 'loading' | 'buffering' | 'playing' | 'paused';

export type EngineEvent =
	| { type: 'status'; state: PlayState; token: number | null; position: number; duration: number; buffered: number }
	| { type: 'trackStarted'; token: number }
	| { type: 'ended' }
	| { type: 'error'; token: number | null; message: string }
	| { type: 'signal'; path: SignalPath | null };

export interface TrackReq {
	token: number;
	url: string;
	cacheKey: string;
	suffix: string;
	duration: number;
	albumId: string;
	replayGain: { trackGain?: number; albumGain?: number; trackPeak?: number; albumPeak?: number };
	/** Used only by the simulated transport. */
	sim?: { bits?: number; rate?: number; lossless: boolean };
}

type Listener = (event: EngineEvent) => void;

/** Simulated engine used when there is no native side or no real stream URL. */
class SimEngine {
	private listeners = new Set<Listener>();
	private cur: TrackReq | null = null;
	private next: TrackReq | null = null;
	private position = 0;
	private playing = false;
	private last = 0;
	private exclusive = false;
	private volume = 1;
	private dsp = false;
	private timer: ReturnType<typeof setInterval> | null = null;

	subscribe(fn: Listener) {
		this.listeners.add(fn);
		return () => this.listeners.delete(fn);
	}

	private emit(event: EngineEvent) {
		this.listeners.forEach((fn) => fn(event));
	}

	private status() {
		this.emit({
			type: 'status',
			state: this.cur ? (this.playing ? 'playing' : 'paused') : 'stopped',
			token: this.cur?.token ?? null,
			position: this.position,
			duration: this.cur?.duration ?? 0,
			buffered: 1
		});
	}

	private signal() {
		if (!this.cur) return this.emit({ type: 'signal', path: null });
		const sim = this.cur.sim ?? { lossless: true, bits: 16, rate: 44100 };
		const rate = sim.rate ?? 44100;
		const deviceRate = this.exclusive ? rate : 48000;
		const resampled = deviceRate !== rate;
		const unity = this.volume === 1;
		this.emit({
			type: 'signal',
			path: {
				source: { codec: sim.lossless ? 'flac' : 'mp3', rate, channels: 2, bits: sim.bits ?? null, lossless: sim.lossless, duration: this.cur.duration },
				device: { deviceName: 'Демо-устройство', rate: deviceRate, channels: 2, bits: this.exclusive ? 24 : 32, float: !this.exclusive, exclusive: this.exclusive, note: null },
				resampled,
				replayGainDb: 0,
				dspActive: this.dsp,
				volumeUnity: unity,
				dithered: this.exclusive && (this.dsp || !unity || !sim.lossless),
				bitPerfect: this.exclusive && sim.lossless && !this.dsp && unity && !resampled
			}
		});
	}

	private start(track: TrackReq) {
		this.cur = track;
		this.position = 0;
		this.emit({ type: 'trackStarted', token: track.token });
		this.signal();
	}

	private tick() {
		const now = performance.now();
		if (this.playing && this.cur) {
			this.position += (now - this.last) / 1000;
			if (this.position >= this.cur.duration) {
				if (this.next) {
					const next = this.next;
					this.next = null;
					this.start(next);
				} else {
					this.cur = null;
					this.playing = false;
					this.emit({ type: 'ended' });
					this.signal();
				}
			}
		}
		this.last = now;
		this.status();
	}

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	invoke(cmd: string, args: any = {}): unknown {
		this.timer ??= setInterval(() => this.tick(), 250);
		switch (cmd) {
			case 'player_play':
				this.playing = true;
				this.last = performance.now();
				this.start(args.track);
				break;
			case 'player_set_next':
				this.next = args.track;
				break;
			case 'player_pause':
				this.tick();
				this.playing = false;
				break;
			case 'player_resume':
				this.last = performance.now();
				this.playing = !!this.cur;
				break;
			case 'player_stop':
				this.cur = null;
				this.playing = false;
				this.signal();
				break;
			case 'player_seek':
				this.position = args.position;
				this.last = performance.now();
				break;
			case 'player_set_volume':
				this.volume = args.volume;
				this.signal();
				break;
			case 'player_set_dsp': {
				const c = args.config;
				this.dsp = c.enabled && (c.preampDb !== 0 || c.bassDb !== 0 || c.crossfeed > 0 || c.width !== 1 || (c.eqEnabled && c.bands.some((b: { enabled: boolean; gainDb: number }) => b.enabled && b.gainDb !== 0)));
				this.signal();
				break;
			}
			case 'player_set_output':
				this.exclusive = args.config.exclusive;
				this.signal();
				break;
			case 'list_output_devices':
				return [
					{ id: 'demo-1', name: 'Динамики (демо)', isDefault: true },
					{ id: 'demo-2', name: 'USB DAC (демо)', isDefault: false }
				];
			case 'cache_info':
				return { bytes: 0, files: 0 };
		}
		this.status();
		return undefined;
	}
}

const sim = new SimEngine();
let simulated = !inTauri;

/** Route audio commands to the simulated transport (demo mode). */
export function useSimulatedEngine(on: boolean) {
	simulated = on || !inTauri;
}

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	const audio = cmd.startsWith('player_') || cmd === 'list_output_devices' || cmd.startsWith('cache_');
	if (audio && simulated) return sim.invoke(cmd, args) as T;
	if (!inTauri) return undefined as T;
	return tauriInvoke<T>(cmd, args);
}

export function onEngineEvent(fn: Listener): () => void {
	const offSim = sim.subscribe((e) => simulated && fn(e));
	let offNative: (() => void) | undefined;
	let cancelled = false;
	if (inTauri) {
		tauriListen<EngineEvent>('player', (e) => !simulated && fn(e.payload)).then((off) => {
			if (cancelled) off();
			else offNative = off;
		});
	}
	return () => {
		cancelled = true;
		offSim();
		offNative?.();
	};
}

export async function listen<T>(event: string, fn: (payload: T) => void): Promise<() => void> {
	if (!inTauri) return () => {};
	return tauriListen<T>(event, (e) => fn(e.payload));
}
