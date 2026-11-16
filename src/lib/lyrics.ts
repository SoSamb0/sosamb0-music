import type { LyricLine, Lyrics, Song } from './types';

/** Parse LRC text. Text without timestamps comes back as unsynced lines. */
export function parseLrc(text: string): { synced: boolean; lines: LyricLine[] } {
	const timed: LyricLine[] = [];
	const plain: LyricLine[] = [];
	let offset = 0;

	for (const raw of text.split(/\r?\n/)) {
		const meta = raw.match(/^\s*\[offset:\s*([+-]?\d+)\s*\]/i);
		if (meta) {
			// LRC offset is in milliseconds; positive values make lyrics appear sooner.
			offset = -Number(meta[1]) / 1000;
			continue;
		}
		const stamps = [...raw.matchAll(/\[(\d{1,3}):(\d{1,2})(?:[.:](\d{1,3}))?\]/g)];
		const body = raw
			.replace(/\[[^\]]*\]/g, '')
			.replace(/<\d{1,3}:\d{1,2}(?:[.:]\d{1,3})?>/g, '')
			.trim();
		if (stamps.length === 0) {
			if (body && !/^\s*\[[a-z]+:/i.test(raw)) plain.push({ text: body });
			continue;
		}
		for (const s of stamps) {
			const fraction = s[3] ? Number(s[3]) / 10 ** s[3].length : 0;
			timed.push({ time: Number(s[1]) * 60 + Number(s[2]) + fraction, text: body });
		}
	}

	if (timed.length >= 2) {
		timed.sort((a, b) => a.time! - b.time!);
		return { synced: true, lines: timed.map((l) => ({ time: Math.max(0, l.time! + offset), text: l.text })) };
	}
	return { synced: false, lines: plain.length ? plain : timed.map((l) => ({ text: l.text })) };
}

/** Index of the line being sung at `position`, or -1 before the first line. */
export function activeLine(lines: LyricLine[], position: number): number {
	let lo = 0;
	let hi = lines.length - 1;
	let found = -1;
	while (lo <= hi) {
		const mid = (lo + hi) >> 1;
		if ((lines[mid].time ?? 0) <= position) {
			found = mid;
			lo = mid + 1;
		} else {
			hi = mid - 1;
		}
	}
	return found;
}

/** Look a song up on LRCLIB, a free community lyrics database. */
export async function fetchLrclib(song: Song): Promise<Lyrics | null> {
	if (!song.title || !song.artist) return null;
	const q = new URLSearchParams({ track_name: song.title, artist_name: song.artist });
	if (song.album) q.set('album_name', song.album);
	if (song.duration) q.set('duration', String(Math.round(song.duration)));
	try {
		const response = await fetch(`https://lrclib.net/api/get?${q}`);
		if (!response.ok) return null;
		const body = await response.json();
		const text: string | undefined = body.syncedLyrics || body.plainLyrics;
		if (!text) return null;
		return { ...parseLrc(text), source: 'lrclib' };
	} catch {
		return null;
	}
}
