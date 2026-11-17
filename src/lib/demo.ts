// A small generated library so the interface can be explored without a server.
// Nothing here makes sound: demo mode drives the UI with a simulated transport.

import type { Album, AlbumListType, Artist, Genre, Lyrics, MusicApi, Playlist, Song } from './types';

const ARTISTS = ['Северное сияние', 'Kaleid', 'Тихий океан', 'Mono Atlas', 'Полночный экспресс', 'Vela'];
const ALBUM_WORDS = ['Отражения', 'Afterglow', 'Долгая зима', 'Parallax', 'Города из стекла', 'Low Tide', 'Signal', 'Маяк'];
const TRACK_WORDS = [
	'Первый свет', 'Undertow', 'Стеклянный дождь', 'Halcyon', 'Линия горизонта', 'Static Bloom', 'Ночной рейс',
	'Paper Moons', 'Тёплый шум', 'Meridian', 'Без адреса', 'Slow Orbit', 'Соль и ветер', 'Echo Park', 'Пульс'
];
const GENRES = ['Электроника', 'Инди', 'Эмбиент', 'Джаз', 'Рок', 'Неоклассика'];
const FORMATS = [
	{ suffix: 'flac', bitDepth: 16, samplingRate: 44100, bitRate: 942 },
	{ suffix: 'flac', bitDepth: 24, samplingRate: 96000, bitRate: 2890 },
	{ suffix: 'flac', bitDepth: 24, samplingRate: 48000, bitRate: 1612 },
	{ suffix: 'mp3', bitDepth: undefined, samplingRate: 44100, bitRate: 320 }
];

function rng(seed: number) {
	let s = seed >>> 0 || 1;
	return () => {
		s ^= s << 13;
		s ^= s >>> 17;
		s ^= s << 5;
		return (s >>> 0) / 2 ** 32;
	};
}

function cover(seed: number, label: string): string {
	const r = rng(seed * 7919 + 13);
	const hue = Math.floor(r() * 360);
	const hue2 = (hue + 40 + Math.floor(r() * 80)) % 360;
	const cx = 20 + Math.floor(r() * 60);
	const cy = 20 + Math.floor(r() * 60);
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${hue} 70% 46%)"/><stop offset="1" stop-color="hsl(${hue2} 65% 22%)"/></linearGradient><radialGradient id="h" cx="${cx}%" cy="${cy}%" r="60%"><stop offset="0" stop-color="hsl(${hue} 90% 72%)" stop-opacity=".75"/><stop offset="1" stop-color="hsl(${hue} 90% 72%)" stop-opacity="0"/></radialGradient></defs><rect width="100" height="100" fill="url(#g)"/><rect width="100" height="100" fill="url(#h)"/><circle cx="${100 - cx}" cy="${100 - cy}" r="${14 + Math.floor(r() * 16)}" fill="none" stroke="rgba(255,255,255,.28)" stroke-width="1.2"/><text x="8" y="92" font-family="Segoe UI,sans-serif" font-size="7" font-weight="600" fill="rgba(255,255,255,.82)">${label}</text></svg>`;
	return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

const albums: Album[] = [];
const songs = new Map<string, Song>();
const covers = new Map<string, string>();
const starredSongs = new Set<string>();

(function build() {
	const r = rng(20261114);
	for (let a = 0; a < 14; a++) {
		const artistIndex = a % ARTISTS.length;
		const name = ALBUM_WORDS[a % ALBUM_WORDS.length] + (a >= ALBUM_WORDS.length ? ' II' : '');
		const id = `al-${a}`;
		const format = FORMATS[Math.floor(r() * FORMATS.length)];
		const genre = GENRES[artistIndex % GENRES.length];
		const year = 2012 + Math.floor(r() * 14);
		covers.set(id, cover(a + 1, name));
		const count = 6 + Math.floor(r() * 5);
		const list: Song[] = [];
		for (let t = 0; t < count; t++) {
			const song: Song = {
				id: `${id}-s${t}`,
				title: TRACK_WORDS[Math.floor(r() * TRACK_WORDS.length)] + (r() > 0.85 ? ' (Live)' : ''),
				artist: ARTISTS[artistIndex],
				artistId: `ar-${artistIndex}`,
				album: name,
				albumId: id,
				coverArt: id,
				duration: 150 + Math.floor(r() * 190),
				track: t + 1,
				year,
				genre,
				...format,
				replayGain: { trackGain: -4 - r() * 6, albumGain: -6.5, trackPeak: 0.97, albumPeak: 0.99 }
			};
			if (r() > 0.8) {
				song.starred = '2026-01-01T00:00:00Z';
				starredSongs.add(song.id);
			}
			songs.set(song.id, song);
			list.push(song);
		}
		albums.push({
			id,
			name,
			artist: ARTISTS[artistIndex],
			artistId: `ar-${artistIndex}`,
			coverArt: id,
			songCount: count,
			duration: list.reduce((sum, s) => sum + (s.duration ?? 0), 0),
			year,
			genre,
			song: list
		});
	}
})();

const playlists: Playlist[] = [
	{ id: 'pl-0', name: 'Для работы', owner: 'demo', entry: [...songs.values()].filter((_, i) => i % 7 === 0) },
	{ id: 'pl-1', name: 'Вечер', owner: 'demo', entry: [...songs.values()].filter((_, i) => i % 11 === 3) }
];

const LYRIC_LINES = [
	'Город засыпает под стеклянный дождь', 'Мы считаем окна, словно этажи', 'Свет чужих экранов гаснет за спиной',
	'И дорога тянется к воде', 'Слышишь, как гудят провода', 'Ночь ложится мягко на плечи', 'Мы вернёмся, когда рассветёт',
	'А пока — только шум и тепло', 'Держи меня на линии', 'Пока горит маяк'
];

const shuffled = <T,>(items: T[], seed: number): T[] => {
	const r = rng(seed);
	return items
		.map((item) => ({ item, key: r() }))
		.sort((a, b) => a.key - b.key)
		.map((x) => x.item);
};

const delay = <T,>(value: T): Promise<T> => new Promise((resolve) => setTimeout(() => resolve(value), 60));
const artistOf = (i: number): Artist => {
	const own = albums.filter((al) => al.artistId === `ar-${i}`);
	return { id: `ar-${i}`, name: ARTISTS[i], coverArt: own[0]?.id, albumCount: own.length, album: own };
};

export class DemoApi implements MusicApi {
	readonly id = 'demo';

	ping = () => delay(undefined);

	albumList(type: AlbumListType, size: number, offset = 0, genre?: string) {
		let list = [...albums];
		if (type === 'random') list = shuffled(list, Math.floor(Date.now() / 60_000));
		else if (type === 'newest') list.reverse();
		else if (type === 'frequent') list = shuffled(list, 5);
		else if (type === 'recent') list = shuffled(list, 9);
		else if (type === 'alphabeticalByName') list.sort((a, b) => a.name.localeCompare(b.name));
		else if (type === 'byGenre') list = list.filter((a) => a.genre === genre);
		else if (type === 'starred') list = [];
		return delay(list.slice(offset, offset + size));
	}

	album = (id: string) => delay(albums.find((a) => a.id === id)!);
	artists = () => delay(ARTISTS.map((_, i) => artistOf(i)));
	artist = (id: string) => delay(artistOf(Number(id.slice(3))));
	topSongs = (artist: string, count = 10) =>
		delay(shuffled([...songs.values()].filter((s) => s.artist === artist), 3).slice(0, count));
	similarSongs = (id: string, count = 50) =>
		delay(shuffled([...songs.values()].filter((s) => s.id !== id && s.genre === songs.get(id)?.genre), 17).slice(0, count));
	randomSongs = (size: number, genre?: string) =>
		delay(shuffled([...songs.values()].filter((s) => !genre || s.genre === genre), Date.now() & 0xffff).slice(0, size));

	playlists = () =>
		delay(playlists.map((p) => ({ ...p, songCount: p.entry!.length, coverArt: p.entry![0]?.coverArt, entry: undefined })));
	playlist = (id: string) => delay({ ...playlists.find((p) => p.id === id)! });

	createPlaylist(name: string, songIds: string[]) {
		playlists.push({ id: `pl-${Date.now()}`, name, owner: 'demo', entry: songIds.map((id) => songs.get(id)!) });
		return delay(undefined);
	}

	addToPlaylist(id: string, songIds: string[]) {
		playlists.find((p) => p.id === id)?.entry!.push(...songIds.map((s) => songs.get(s)!));
		return delay(undefined);
	}

	removeFromPlaylist(id: string, indexes: number | number[]) {
		const entry = playlists.find((p) => p.id === id)?.entry;
		// Highest first, so earlier removals do not shift the later positions.
		for (const i of [indexes].flat().sort((a, b) => b - a)) entry?.splice(i, 1);
		return delay(undefined);
	}

	reorderPlaylist(id: string, songIds: string[]) {
		const p = playlists.find((x) => x.id === id);
		if (p) p.entry = songIds.map((s) => songs.get(s)!);
		return delay(undefined);
	}

	songs(query: string, count: number, offset: number) {
		const q = query.trim().toLowerCase();
		const all = [...songs.values()].filter((s) => !q || s.title.toLowerCase().includes(q) || (s.artist ?? '').toLowerCase().includes(q));
		return delay(all.slice(offset, offset + count));
	}

	renamePlaylist(id: string, name: string) {
		const p = playlists.find((x) => x.id === id);
		if (p) p.name = name;
		return delay(undefined);
	}

	deletePlaylist(id: string) {
		const i = playlists.findIndex((p) => p.id === id);
		if (i >= 0) playlists.splice(i, 1);
		return delay(undefined);
	}

	starred = () => delay({ songs: [...songs.values()].filter((s) => starredSongs.has(s.id)), albums: [], artists: [] });

	star(kind: 'song' | 'album' | 'artist', id: string, on: boolean) {
		if (kind === 'song') {
			const song = songs.get(id);
			if (song) song.starred = on ? new Date().toISOString() : undefined;
			if (on) starredSongs.add(id);
			else starredSongs.delete(id);
		}
		return delay(undefined);
	}

	search(query: string) {
		const q = query.toLowerCase();
		const has = (s?: string) => !!s && s.toLowerCase().includes(q);
		return delay({
			songs: [...songs.values()].filter((s) => has(s.title) || has(s.artist)).slice(0, 50),
			albums: albums.filter((a) => has(a.name) || has(a.artist)),
			artists: ARTISTS.map((_, i) => artistOf(i)).filter((a) => has(a.name))
		});
	}

	genres = () =>
		delay(
			GENRES.map<Genre>((value) => ({
				value,
				albumCount: albums.filter((a) => a.genre === value).length,
				songCount: [...songs.values()].filter((s) => s.genre === value).length
			})).filter((g) => g.songCount)
		);

	scrobble = () => delay(undefined);

	lyrics(song: Song): Promise<Lyrics | null> {
		const r = rng(song.id.length * 131 + (song.duration ?? 1));
		if (r() > 0.8) return delay(null);
		const lines = [];
		for (let t = 6; t < (song.duration ?? 180) - 8; t += 3.5 + r() * 3) {
			lines.push({ time: t, text: LYRIC_LINES[Math.floor(r() * LYRIC_LINES.length)] });
		}
		return delay({ synced: true, lines, source: 'server' as const });
	}

	coverUrl = (id: string | undefined) => (id ? (covers.get(id) ?? '') : '');
	streamUrl = () => '';
}
