import { app } from './app.svelte';
import { fetchLrclib } from './lyrics';
import { settings } from './settings.svelte';
import { SvelteSet } from 'svelte/reactivity';
import { setVizFrame } from './viz';
import { invoke, listen, onEngineEvent, type EngineEvent, type PlayState, type SignalPath, type TrackReq } from './tauri';
import type { Lyrics, Song } from './types';

export interface QueueItem {
	token: number;
	song: Song;
}

type CacheEvent =
	| { type: 'cached'; key: string }
	| { type: 'failed'; key: string; message: string }
	| { type: 'status'; done: number; total: number; active: string[]; limited: boolean };

export interface HistoryEntry {
	id: string;
	title: string;
	artist: string;
	album: string;
	seconds: number;
	at: number;
}

const QUEUE_KEY = 'lumen.queue.v1';
const HISTORY_KEY = 'lumen.history.v1';
const LOSSLESS = ['flac', 'alac', 'wav', 'aiff', 'aif', 'ape', 'wv', 'tta', 'dsf'];

export function isLossless(song: Song): boolean {
	return LOSSLESS.includes((song.suffix ?? '').toLowerCase());
}

/** "FLAC · 24 бит / 96 кГц" */
export function formatQuality(song: Song): string {
	const parts = [(song.suffix ?? '').toUpperCase()];
	if (isLossless(song)) {
		const rate = song.samplingRate ? `${+(song.samplingRate / 1000).toFixed(1)} кГц` : '';
		if (song.bitDepth && rate) parts.push(`${song.bitDepth} бит / ${rate}`);
		else if (rate) parts.push(rate);
	} else if (song.bitRate) {
		parts.push(`${song.bitRate} кбит/с`);
	}
	return parts.filter(Boolean).join(' · ');
}

class Player {
	queue = $state<QueueItem[]>([]);
	index = $state(-1);
	state = $state<PlayState>('stopped');
	position = $state(0);
	duration = $state(0);
	buffered = $state(0);
	signal = $state<SignalPath | null>(null);
	shuffle = $state(false);
	repeat = $state<'off' | 'all' | 'one'>('off');

	lyrics = $state<Lyrics | null>(null);
	lyricsLoading = $state(false);
	/** Manual timing correction for the current song, in seconds. */
	lyricsOffset = $state(0);

	sleepUntil = $state<number | null>(null);
	sleepAfterTrack = $state(false);

	current = $derived<Song | null>(this.queue[this.index]?.song ?? null);
	playing = $derived(this.state === 'playing' || this.state === 'loading' || this.state === 'buffering');

	private positionAt = 0;
	private nextToken = 1;
	private unshuffled: QueueItem[] | null = null;
	private scrobbled = false;
	private listened = 0;
	private failures = 0;
	private failed = new Set<number>();
	private lyricsFor: string | null = null;
	private saveTimer: ReturnType<typeof setTimeout> | undefined;
	private started = false;

	/** Cache keys of queue tracks that are fully downloaded. */
	cached = new SvelteSet<string>();
	/** Cache keys being downloaded in the background right now. */
	downloading = new SvelteSet<string>();
	/** The size limit stopped the queue from being cached completely. */
	cacheLimited = $state(false);
	private prefetchTimer: ReturnType<typeof setTimeout> | undefined;

	/** Call whenever a server connection is opened: the first time, and after switching servers. */
	init() {
		if (!this.started) {
			this.started = true;
			onEngineEvent((e) => this.onEvent(e));
			listen<CacheEvent>('cache', (e) => this.onCacheEvent(e));
		}
		// Demo and real servers use different engines, so settings are sent again each time.
		this.restore();
		settings.pushAll();
	}

	/** Position right now, extrapolated between engine reports. For smooth lyrics and progress. */
	livePosition(): number {
		if (this.state !== 'playing') return this.position;
		const elapsed = (performance.now() - this.positionAt) / 1000;
		const live = this.position + elapsed;
		return this.duration > 0 ? Math.min(live, this.duration) : live;
	}

	// ----------------------------------------------------------- engine events

	private onEvent(e: EngineEvent) {
		switch (e.type) {
			case 'status': {
				const now = performance.now();
				if (this.state === 'playing' && e.state === 'playing') {
					this.listened += Math.min(1, (now - this.positionAt) / 1000);
				}
				const wasState = this.state;
				this.state = e.state;
				this.position = e.position;
				this.positionAt = now;
				if (e.duration > 0) this.duration = e.duration;
				this.buffered = e.buffered;
				this.maybeScrobble();
				// Real playback start (after loading or buffering) fixes the timer shown in Discord.
				if (wasState !== e.state && (e.state === 'playing' || e.state === 'paused')) this.notifyDiscord();
				if (this.sleepUntil && Date.now() >= this.sleepUntil) {
					this.sleepUntil = null;
					this.pause();
					app.toast('Таймер сна: воспроизведение остановлено');
				}
				break;
			}
			case 'trackStarted': {
				const i = this.queue.findIndex((q) => q.token === e.token);
				if (i < 0) break;
				this.index = i;
				this.failures = 0;
				// A gapless change starts at zero; the next status confirms it.
				this.position = 0;
				this.positionAt = performance.now();
				this.onTrackChange();
				break;
			}
			case 'ended':
				this.state = 'stopped';
				this.position = 0;
				if (this.sleepAfterTrack) {
					this.sleepAfterTrack = false;
					app.toast('Таймер сна: воспроизведение остановлено');
				}
				this.notifyMedia();
				break;
			case 'error': {
				const item = this.queue.find((q) => q.token === e.token);
				app.toast(item ? `«${item.song.title}»: ${e.message}` : e.message, 'error');
				if (e.token === null) break;
				this.failed.add(e.token);
				if (this.queue[this.index]?.token === e.token) {
					// The track being started failed: move on, but not forever.
					if (++this.failures < 4 && this.nextIndex() !== null) this.next();
					else this.stop();
				} else {
					this.syncNext();
				}
				break;
			}
			case 'signal':
				this.signal = e.path;
				if (e.path && settings.deviceChanged(e.path.device.deviceName)) app.toast(`Звук для «${e.path.device.deviceName}»`);
				break;
			case 'viz':
				setVizFrame(e);
				break;
		}
	}

	private onTrackChange() {
		const song = this.current;
		this.scrobbled = false;
		this.listened = 0;
		this.duration = song?.duration ?? 0;
		this.syncNext();
		this.persist();
		this.notifyMedia();
		if (!song) return;
		this.loadLyrics(song);
		if (settings.data.scrobble) app.api?.scrobble(song.id, false).catch(() => {});
	}

	private maybeScrobble() {
		const song = this.current;
		if (!song || this.scrobbled || !this.duration) return;
		// Last.fm rule: half the track or four minutes, whichever comes first.
		if (this.listened < Math.min(this.duration / 2, 240)) return;
		this.scrobbled = true;
		if (settings.data.scrobble) app.api?.scrobble(song.id, true).catch(() => {});
		this.logHistory(song);
	}

	private async loadLyrics(song: Song) {
		if (this.lyricsFor === song.id) return;
		this.lyricsFor = song.id;
		this.lyrics = null;
		this.lyricsLoading = true;
		this.lyricsOffset = Number(localStorage.getItem(`lumen.lyricsOffset.${song.id}`) ?? 0) || 0;
		let found = (await app.api?.lyrics(song).catch(() => null)) ?? null;
		// The community database is consulted only when the server has no timed lyrics.
		if (!found?.synced && settings.data.lrclib && !settings.data.demo) {
			const online = await fetchLrclib(song);
			if (online && (online.synced || !found)) found = online;
		}
		if (this.lyricsFor !== song.id) return;
		this.lyrics = found;
		this.lyricsLoading = false;
	}

	setLyricsOffset(seconds: number) {
		this.lyricsOffset = Math.round(seconds * 10) / 10;
		const song = this.current;
		if (!song) return;
		const key = `lumen.lyricsOffset.${song.id}`;
		if (this.lyricsOffset) localStorage.setItem(key, String(this.lyricsOffset));
		else localStorage.removeItem(key);
	}

	// ---------------------------------------------------------------- transport

	cacheKey(song: Song): string {
		return `${app.api?.id ?? 'x'}_${song.id}`;
	}

	private onCacheEvent(e: CacheEvent) {
		if (e.type === 'cached') {
			this.cached.add(e.key);
			this.downloading.delete(e.key);
		} else if (e.type === 'failed') {
			this.downloading.delete(e.key);
		} else {
			this.downloading.clear();
			e.active.forEach((k) => this.downloading.add(k));
			this.cacheLimited = e.limited;
		}
	}

	private schedulePrefetch() {
		clearTimeout(this.prefetchTimer);
		this.prefetchTimer = setTimeout(() => this.syncPrefetch(), 700);
	}

	/** Tell the native side which queue tracks to download in the background, in play order. */
	async syncPrefetch() {
		clearTimeout(this.prefetchTimer);
		const limitMb = settings.data.options.cacheLimitMb;
		const items = [...this.queue];
		if (!items.length || !app.api || settings.data.demo) {
			this.cached.clear();
			return invoke('cache_prefetch', { tracks: [], limitMb });
		}
		const flags = await invoke<boolean[]>('cache_contains', {
			items: items.map((q) => ({ cacheKey: this.cacheKey(q.song), suffix: q.song.suffix ?? '' }))
		}).catch(() => undefined);
		this.cached.clear();
		flags?.forEach((on, i) => on && this.cached.add(this.cacheKey(items[i].song)));
		if (!settings.data.cache.prefetch) return invoke('cache_prefetch', { tracks: [], limitMb });

		// The playing track first (so seeking within it never waits), then what follows.
		const start = Math.max(0, this.index);
		let order = [...items.slice(start), ...(this.repeat === 'all' ? items.slice(0, start) : [])];
		const ahead = settings.data.cache.ahead;
		if (ahead > 0) order = order.slice(0, ahead + 1);
		const seen = new Set<string>();
		const tracks = order
			.filter((q) => {
				const key = this.cacheKey(q.song);
				if (this.cached.has(key) || seen.has(key)) return false;
				seen.add(key);
				return true;
			})
			.map((q) => this.request(q));
		return invoke('cache_prefetch', { tracks, limitMb });
	}

	private request(item: QueueItem): TrackReq {
		const song = item.song;
		return {
			token: item.token,
			url: app.api?.streamUrl(song) ?? '',
			cacheKey: this.cacheKey(song),
			suffix: song.suffix ?? '',
			duration: song.duration ?? 0,
			albumId: song.albumId ?? '',
			replayGain: { ...(song.replayGain ?? {}) },
			sim: { bits: song.bitDepth, rate: song.samplingRate, lossless: isLossless(song) }
		};
	}

	private nextIndex(): number | null {
		if (this.queue.length === 0 || this.index < 0) return null;
		if (this.repeat === 'one') return this.index;
		for (let step = 1; step <= this.queue.length; step++) {
			let i = this.index + step;
			if (i >= this.queue.length) {
				if (this.repeat !== 'all') return null;
				i %= this.queue.length;
			}
			if (!this.failed.has(this.queue[i].token)) return i;
		}
		return null;
	}

	/** Tell the engine what follows the current track, so it can preload it for gapless. */
	private syncNext() {
		const i = this.sleepAfterTrack ? null : this.nextIndex();
		invoke('player_set_next', { track: i === null ? null : this.request(this.queue[i]) });
	}

	private playAt(i: number) {
		const item = this.queue[i];
		if (!item) return;
		this.index = i;
		this.state = 'loading';
		this.position = 0;
		this.positionAt = performance.now();
		this.duration = item.song.duration ?? 0;
		invoke('player_play', { track: this.request(item) });
		this.syncNext();
		this.loadLyrics(item.song);
		this.persist();
	}

	private wrap(songs: Song[]): QueueItem[] {
		return songs.map((song) => ({ token: this.nextToken++, song }));
	}

	/** Replace the queue and start playing at `start`. */
	play(songs: Song[], start = 0, shuffled = false) {
		if (songs.length === 0) return;
		this.failed.clear();
		this.failures = 0;
		let items = this.wrap(songs);
		this.unshuffled = null;
		if (shuffled) this.shuffle = true;
		if (this.shuffle) {
			this.unshuffled = items;
			const first = shuffled ? [] : [items[start]];
			const rest = items.filter((_, i) => shuffled || i !== start);
			items = [...first, ...shuffleArray(rest)];
			start = 0;
		}
		this.queue = items;
		this.playAt(start);
	}

	playNext(songs: Song[]) {
		if (this.queue.length === 0) return this.play(songs);
		this.queue.splice(this.index + 1, 0, ...this.wrap(songs));
		this.unshuffled = null;
		this.syncNext();
		this.persist();
		app.toast(songs.length === 1 ? 'Будет играть следующим' : `Добавлено следующими: ${songs.length}`);
	}

	enqueue(songs: Song[]) {
		if (this.queue.length === 0) return this.play(songs);
		this.queue.push(...this.wrap(songs));
		this.unshuffled = null;
		this.syncNext();
		this.persist();
		app.toast(songs.length === 1 ? 'Добавлено в очередь' : `Добавлено в очередь: ${songs.length}`);
	}

	jump(i: number) {
		this.failures = 0;
		this.playAt(i);
	}

	toggle() {
		if (this.state === 'stopped') {
			if (this.queue.length) this.playAt(Math.max(0, this.index));
		} else if (this.state === 'paused') {
			this.state = 'playing';
			this.positionAt = performance.now();
			invoke('player_resume');
		} else {
			this.pause();
		}
		this.notifyMedia();
	}

	pause() {
		if (this.state === 'stopped' || this.state === 'paused') return;
		this.position = this.livePosition();
		this.state = 'paused';
		invoke('player_pause');
		this.notifyMedia();
	}

	stop() {
		this.state = 'stopped';
		this.position = 0;
		invoke('player_stop');
		this.notifyMedia();
	}

	next() {
		// A manual skip ignores "repeat one".
		const saved = this.repeat;
		if (saved === 'one') this.repeat = 'all';
		const i = this.nextIndex();
		this.repeat = saved;
		if (i === null) this.stop();
		else this.playAt(i);
	}

	previous() {
		if (this.livePosition() > 3 || this.index <= 0) this.seek(0);
		else this.playAt(this.index - 1);
	}

	seek(seconds: number) {
		if (!this.current || this.state === 'stopped') return;
		const target = Math.max(0, Math.min(seconds, Math.max(0, this.duration - 0.5)));
		this.position = target;
		this.positionAt = performance.now();
		invoke('player_seek', { position: target });
		// Also moves the lock-screen progress bar (and updates Discord).
		this.notifyMedia();
	}

	setVolume(volume: number) {
		settings.data.volume = Math.max(0, Math.min(1, volume));
		settings.pushVolume();
	}

	toggleShuffle() {
		this.shuffle = !this.shuffle;
		const current = this.queue[this.index];
		if (this.shuffle) {
			this.unshuffled = [...this.queue];
			const rest = this.queue.filter((_, i) => i !== this.index);
			this.queue = current ? [current, ...shuffleArray(rest)] : shuffleArray(rest);
			this.index = current ? 0 : -1;
		} else if (this.unshuffled) {
			this.queue = this.unshuffled;
			this.unshuffled = null;
			this.index = current ? this.queue.findIndex((q) => q.token === current.token) : -1;
		}
		this.syncNext();
		this.persist();
	}

	cycleRepeat() {
		this.repeat = this.repeat === 'off' ? 'all' : this.repeat === 'all' ? 'one' : 'off';
		this.syncNext();
		this.persist();
	}

	remove(i: number) {
		if (i === this.index) return;
		this.queue.splice(i, 1);
		if (i < this.index) this.index--;
		this.unshuffled = null;
		this.syncNext();
		this.persist();
	}

	move(from: number, to: number) {
		if (from === to || from < 0 || to < 0 || from >= this.queue.length || to >= this.queue.length) return;
		const current = this.queue[this.index];
		const [item] = this.queue.splice(from, 1);
		this.queue.splice(to, 0, item);
		this.index = current ? this.queue.findIndex((q) => q.token === current.token) : -1;
		this.unshuffled = null;
		this.syncNext();
		this.persist();
	}

	/** Drop everything except the track that is playing. */
	clearUpcoming() {
		const current = this.queue[this.index];
		this.queue = current ? [current] : [];
		this.index = current ? 0 : -1;
		this.unshuffled = null;
		this.syncNext();
		this.persist();
	}

	// ------------------------------------------------------------------- extras

	/** Endless mix of songs similar to `song`, falling back to its genre. */
	async radio(song: Song) {
		const api = app.api;
		if (!api) return;
		let similar = (await api.similarSongs(song.id, 60).catch(() => [])) as Song[];
		if (similar.length < 5) similar = await api.randomSongs(60, song.genre).catch(() => []);
		const rest = similar.filter((s) => s.id !== song.id);
		const wasShuffle = this.shuffle;
		this.shuffle = false;
		this.play([song, ...rest], 0);
		this.shuffle = wasShuffle;
		app.toast(`Радио по треку «${song.title}»`);
	}

	/** Star or unstar several tracks. */
	async starMany(songs: Song[], on: boolean) {
		const todo = songs.filter((s) => !!s.starred !== on);
		const value = on ? new Date().toISOString() : undefined;
		for (const song of todo) {
			song.starred = value;
			for (const q of this.queue) if (q.song.id === song.id) q.song.starred = value;
		}
		const ok = await app.attempt(async () => {
			await Promise.all(todo.map((s) => app.api!.star('song', s.id, on)));
			return true;
		}, 'Не удалось изменить избранное');
		if (ok) app.toast(on ? `В избранное: ${songs.length}` : `Убрано из избранного: ${songs.length}`);
	}

	async toggleStar(song: Song) {
		const on = !song.starred;
		const value = on ? new Date().toISOString() : undefined;
		song.starred = value;
		for (const q of this.queue) if (q.song.id === song.id) q.song.starred = value;
		await app.attempt(() => app.api!.star('song', song.id, on), 'Не удалось изменить избранное');
	}

	setSleep(minutes: number | 'track' | null) {
		this.sleepUntil = null;
		this.sleepAfterTrack = false;
		if (minutes === 'track') this.sleepAfterTrack = true;
		else if (minutes) this.sleepUntil = Date.now() + minutes * 60_000;
		this.syncNext();
	}

	private albumMbids = new Map<string, string>();
	private discordTimer: ReturnType<typeof setTimeout> | undefined;

	/** Send what is playing to Discord. Debounced; the native side also rate-limits. */
	notifyDiscord() {
		clearTimeout(this.discordTimer);
		this.discordTimer = setTimeout(async () => {
			const song = this.current;
			if (!settings.data.discord.enabled || settings.data.demo) return;
			if (!song || this.state === 'stopped') return invoke('discord_update', { now: null });
			let mbid = song.albumId ? this.albumMbids.get(song.albumId) : '';
			if (mbid === undefined && song.albumId) {
				const album = await app.api?.album(song.albumId).catch(() => null);
				mbid = album?.musicBrainzId ?? '';
				this.albumMbids.set(song.albumId, mbid);
			}
			if (song !== this.current) return;
			invoke('discord_update', {
				now: {
					title: song.title,
					artist: song.artist ?? '',
					album: song.album ?? '',
					albumMbid: mbid ?? '',
					duration: this.duration || song.duration || 0,
					position: this.livePosition(),
					playing: this.state !== 'paused'
				}
			});
		}, 400);
	}

	private notifyMedia() {
		this.notifyDiscord();
		const song = this.current;
		invoke('media_update', {
			info: {
				title: song?.title ?? '',
				artist: song?.artist ?? '',
				album: song?.album ?? '',
				coverUrl: song && !settings.data.demo ? (app.api?.coverUrl(song.coverArt, 600) ?? '') : '',
				duration: this.duration,
				position: this.livePosition(),
				state: this.state === 'stopped' ? 'stopped' : this.state === 'paused' ? 'paused' : 'playing'
			}
		}).catch(() => {});
	}

	// -------------------------------------------------------------- persistence

	private persist() {
		this.schedulePrefetch();
		clearTimeout(this.saveTimer);
		this.saveTimer = setTimeout(() => {
			try {
				const songs = this.queue.slice(0, 1000).map((q) => $state.snapshot(q.song));
				localStorage.setItem(
					QUEUE_KEY,
					JSON.stringify({ server: app.api?.id, songs, index: Math.min(this.index, songs.length - 1), repeat: this.repeat, shuffle: this.shuffle })
				);
			} catch {
				// Queue is too large for storage; it simply will not be restored.
			}
		}, 400);
	}

	/** Bring back the last queue for the server we are connected to (without starting playback). */
	restore() {
		try {
			const saved = JSON.parse(localStorage.getItem(QUEUE_KEY) ?? 'null');
			if (!saved || saved.server !== app.api?.id || !Array.isArray(saved.songs)) return;
			this.queue = this.wrap(saved.songs);
			this.index = Math.max(0, Math.min(saved.index ?? 0, this.queue.length - 1));
			this.repeat = saved.repeat ?? 'off';
			this.shuffle = !!saved.shuffle;
			this.duration = this.current?.duration ?? 0;
			if (this.current) this.loadLyrics(this.current);
			this.schedulePrefetch();
		} catch {
			// Corrupt saved queue: start empty.
		}
	}

	reset() {
		this.stop();
		this.queue = [];
		this.index = -1;
		this.lyrics = null;
		this.lyricsFor = null;
	}

	private logHistory(song: Song) {
		try {
			const list: HistoryEntry[] = JSON.parse(localStorage.getItem(HISTORY_KEY) ?? '[]');
			list.push({ id: song.id, title: song.title, artist: song.artist ?? '', album: song.album ?? '', seconds: song.duration ?? 0, at: Date.now() });
			localStorage.setItem(HISTORY_KEY, JSON.stringify(list.slice(-5000)));
		} catch {
			// History is a convenience; losing an entry is harmless.
		}
	}

	history(): HistoryEntry[] {
		try {
			return JSON.parse(localStorage.getItem(HISTORY_KEY) ?? '[]');
		} catch {
			return [];
		}
	}
}

function shuffleArray<T>(items: T[]): T[] {
	const out = [...items];
	for (let i = out.length - 1; i > 0; i--) {
		const j = Math.floor(Math.random() * (i + 1));
		[out[i], out[j]] = [out[j], out[i]];
	}
	return out;
}

export const player = new Player();
