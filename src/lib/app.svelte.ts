import { DemoApi } from './demo';
import { settings } from './settings.svelte';
import { makeServerConfig, Subsonic } from './subsonic';
import { useSimulatedEngine } from './tauri';
import type { MusicApi, Playlist } from './types';

export type Route =
	| { name: 'home' }
	| { name: 'albums' }
	| { name: 'songs' }
	| { name: 'smart'; id: 'top' | 'forgotten' | 'new' }
	| { name: 'album'; id: string }
	| { name: 'artists' }
	| { name: 'artist'; id: string }
	| { name: 'playlists' }
	| { name: 'playlist'; id: string }
	| { name: 'favorites' }
	| { name: 'genres' }
	| { name: 'genre'; id: string }
	| { name: 'search'; q: string }
	| { name: 'stats' }
	| { name: 'settings' };

export interface Toast {
	id: number;
	text: string;
	kind: 'info' | 'error';
}

export type Panel = 'queue' | 'lyrics' | null;

class App {
	api = $state<MusicApi | null>(null);
	connecting = $state(false);

	history = $state<Route[]>([{ name: 'home' }]);
	cursor = $state(0);
	route = $derived(this.history[this.cursor]);

	panel = $state<Panel>(null);
	nowPlaying = $state(false);
	soundOpen = $state(false);
	mini = $state(false);
	toasts = $state<Toast[]>([]);
	playlists = $state<Playlist[]>([]);
	/** Bumped whenever playlist contents may have changed, so open playlist views reload. */
	playlistRevision = $state(0);
	/** A pending text question shown as a dialog (window.prompt is unreliable in webviews). */
	question = $state<{ title: string; value: string; confirm?: string; resolve: (answer: string | null) => void } | null>(null);
	private toastId = 0;

	ask(title: string, value = ''): Promise<string | null> {
		this.question?.resolve(null);
		return new Promise((resolve) => (this.question = { title, value, resolve }));
	}

	/** Ask for confirmation. Resolves true when the user agrees. */
	confirm(title: string, confirm = 'Удалить'): Promise<boolean> {
		this.question?.resolve(null);
		return new Promise((resolve) => (this.question = { title, value: '', confirm, resolve: (a) => resolve(a !== null) }));
	}

	answer(value: string | null) {
		this.question?.resolve(value);
		this.question = null;
	}

	go(route: Route) {
		const current = this.history[this.cursor];
		if (JSON.stringify(current) === JSON.stringify(route)) return;
		// A search refines in place instead of filling the history with keystrokes.
		const replace = current.name === 'search' && route.name === 'search';
		this.history = [...this.history.slice(0, this.cursor + (replace ? 0 : 1)), route];
		this.cursor = this.history.length - 1;
		this.nowPlaying = false;
	}

	back() {
		if (this.nowPlaying) this.nowPlaying = false;
		else if (this.cursor > 0) this.cursor--;
	}

	forward() {
		if (this.cursor < this.history.length - 1) this.cursor++;
	}

	toast(text: string, kind: Toast['kind'] = 'info') {
		const id = ++this.toastId;
		this.toasts.push({ id, text, kind });
		setTimeout(() => (this.toasts = this.toasts.filter((t) => t.id !== id)), kind === 'error' ? 6000 : 3000);
	}

	/** Run an action and surface its failure as a toast instead of an unhandled rejection. */
	async attempt<T>(action: () => Promise<T>, failure?: string): Promise<T | undefined> {
		try {
			return await action();
		} catch (e) {
			this.toast(failure ?? (e instanceof Error ? e.message : String(e)), 'error');
			return undefined;
		}
	}

	async login(url: string, username: string, password: string) {
		const config = makeServerConfig(url, username, password);
		const api = new Subsonic(config);
		this.connecting = true;
		try {
			await api.ping();
		} finally {
			this.connecting = false;
		}
		settings.data.server = config;
		settings.data.demo = false;
		settings.save();
		this.open(api, false);
	}

	demo() {
		settings.data.demo = true;
		settings.save();
		this.open(new DemoApi(), true);
	}

	/** Reconnect with saved credentials, if any. */
	restore() {
		if (settings.data.demo) this.open(new DemoApi(), true);
		else if (settings.data.server) this.open(new Subsonic(settings.data.server), false);
	}

	private open(api: MusicApi, simulated: boolean) {
		useSimulatedEngine(simulated);
		this.api = api;
		this.history = [{ name: 'home' }];
		this.cursor = 0;
		this.refreshPlaylists();
	}

	logout() {
		settings.data.server = null;
		settings.data.demo = false;
		settings.save();
		this.api = null;
		this.playlists = [];
	}

	async refreshPlaylists() {
		this.playlistRevision++;
		const list = await this.attempt(() => this.api!.playlists());
		if (list) this.playlists = list;
	}
}

export const app = new App();

export function formatTime(seconds: number | undefined): string {
	if (seconds === undefined || !isFinite(seconds) || seconds < 0) return '0:00';
	const s = Math.floor(seconds);
	const h = Math.floor(s / 3600);
	const m = Math.floor((s % 3600) / 60);
	const rest = String(s % 60).padStart(2, '0');
	return h ? `${h}:${String(m).padStart(2, '0')}:${rest}` : `${m}:${rest}`;
}

export function formatDuration(seconds: number | undefined): string {
	if (!seconds) return '';
	const m = Math.round(seconds / 60);
	return m >= 60 ? `${Math.floor(m / 60)} ч ${m % 60} мин` : `${m} мин`;
}

export function plural(n: number, one: string, few: string, many: string): string {
	const mod10 = n % 10;
	const mod100 = n % 100;
	const word = mod10 === 1 && mod100 !== 11 ? one : mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14) ? few : many;
	return `${n} ${word}`;
}
