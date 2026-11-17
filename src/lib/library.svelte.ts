// The whole track list of the server, for features that look across the library
// (smart playlists). Loaded on demand and kept for a few minutes.

import { app } from './app.svelte';
import type { Song } from './types';

const PAGE = 500;
const FRESH_MS = 10 * 60_000;

class Library {
	/** Raw (not deeply reactive): can be tens of thousands of tracks. */
	songs = $state.raw<Song[] | null>(null);
	loading = $state(false);
	/** Tracks fetched so far during a load, for a progress line. */
	count = $state(0);
	private loadedAt = 0;
	private server = '';

	async load(force = false): Promise<Song[] | null> {
		const api = app.api;
		if (!api) return null;
		const fresh = this.songs && this.server === api.id && Date.now() - this.loadedAt < FRESH_MS;
		if (fresh && !force) return this.songs;
		if (this.loading) return this.songs;
		this.loading = true;
		this.count = 0;
		try {
			const all: Song[] = [];
			const seen = new Set<string>();
			for (let offset = 0; ; offset += PAGE) {
				const page = await api.songs('', PAGE, offset);
				for (const s of page) {
					if (seen.has(s.id)) continue;
					seen.add(s.id);
					all.push(s);
				}
				this.count = all.length;
				if (page.length < PAGE) break;
			}
			this.songs = all;
			this.server = api.id;
			this.loadedAt = Date.now();
			return all;
		} catch (e) {
			app.toast(e instanceof Error ? e.message : String(e), 'error');
			return this.songs;
		} finally {
			this.loading = false;
		}
	}
}

export const library = new Library();

export type SmartKind = 'top' | 'forgotten' | 'new';

export const SMART: Record<SmartKind, { title: string; icon: string; hint: string }> = {
	top: { title: 'Самые прослушиваемые', icon: 'flame', hint: 'Треки, которые вы включаете чаще всего' },
	forgotten: { title: 'Давно не игравшие', icon: 'hourglass', hint: 'Любимое, что вы давно не включали' },
	new: { title: 'Недавно добавленные', icon: 'sparkle', hint: 'Свежие поступления в библиотеку' }
};

const time = (iso?: string) => (iso ? Date.parse(iso) || 0 : 0);

/**
 * Build a smart playlist. `localPlays` (track id → plays on this computer) stands in for the
 * server's counts when the server does not report them.
 */
export function smartList(kind: SmartKind, songs: Song[], limit: number, localPlays?: Map<string, number>): Song[] {
	const hasCounts = songs.some((s) => s.playCount !== undefined);
	const plays = (s: Song) => (hasCounts ? (s.playCount ?? 0) : (localPlays?.get(s.id) ?? 0));

	if (kind === 'top') {
		return songs
			.filter((s) => plays(s) > 0)
			.sort((a, b) => plays(b) - plays(a) || time(b.played) - time(a.played))
			.slice(0, limit);
	}

	if (kind === 'new') {
		return [...songs].sort((a, b) => time(b.created) - time(a.created)).slice(0, limit);
	}

	// Forgotten: things you played a lot, but not lately. Loosen the rules if that is too few.
	const day = 86_400_000;
	const now = Date.now();
	for (const [minPlays, minDays] of [
		[3, 60],
		[2, 30],
		[1, 14]
	]) {
		const picked = songs
			.filter((s) => plays(s) >= minPlays && s.played && now - time(s.played) > minDays * day)
			.sort((a, b) => plays(b) - plays(a) || time(a.played) - time(b.played));
		if (picked.length >= Math.min(20, limit) || minPlays === 1) return picked.slice(0, limit);
	}
	return [];
}
