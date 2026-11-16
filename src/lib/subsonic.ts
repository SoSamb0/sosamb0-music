import { md5 } from './md5';
import { parseLrc } from './lyrics';
import type { Album, AlbumListType, Artist, Genre, Lyrics, MusicApi, Playlist, Song } from './types';

export interface ServerConfig {
	url: string;
	username: string;
	/** md5(password + salt). The password itself is never stored. */
	token: string;
	salt: string;
}

const CLIENT = 'SoSamb0 Music';
const API_VERSION = '1.16.1';

export function makeServerConfig(url: string, username: string, password: string): ServerConfig {
	const salt = Array.from(crypto.getRandomValues(new Uint8Array(8)), (b) => b.toString(16).padStart(2, '0')).join('');
	let clean = url.trim().replace(/\/+$/, '');
	if (!/^https?:\/\//i.test(clean)) clean = 'http://' + clean;
	return { url: clean, username: username.trim(), token: md5(password + salt), salt };
}

export class SubsonicError extends Error {}

function list<T>(value: T[] | T | undefined): T[] {
	if (value === undefined || value === null) return [];
	return Array.isArray(value) ? value : [value];
}

export class Subsonic implements MusicApi {
	readonly id: string;

	constructor(private cfg: ServerConfig) {
		this.id = md5(cfg.url + '|' + cfg.username).slice(0, 10);
	}

	private query(params: Record<string, string | number | boolean | undefined | (string | number)[]>): string {
		const q = new URLSearchParams({
			u: this.cfg.username,
			t: this.cfg.token,
			s: this.cfg.salt,
			v: API_VERSION,
			c: CLIENT,
			f: 'json'
		});
		for (const [key, value] of Object.entries(params)) {
			if (value === undefined) continue;
			if (Array.isArray(value)) value.forEach((v) => q.append(key, String(v)));
			else q.set(key, String(value));
		}
		return q.toString();
	}

	private url(endpoint: string, params: Record<string, string | number | boolean | undefined | (string | number)[]> = {}) {
		return `${this.cfg.url}/rest/${endpoint}?${this.query(params)}`;
	}

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	private async call(endpoint: string, params: Record<string, string | number | boolean | undefined | (string | number)[]> = {}): Promise<any> {
		let response: Response;
		try {
			response = await fetch(this.url(endpoint, params));
		} catch {
			throw new SubsonicError('Сервер недоступен. Проверьте адрес и подключение.');
		}
		if (!response.ok) throw new SubsonicError(`Сервер ответил ошибкой ${response.status}`);
		const body = (await response.json())['subsonic-response'];
		if (!body) throw new SubsonicError('Это не похоже на сервер Navidrome / Subsonic.');
		if (body.status !== 'ok') {
			const code = body.error?.code;
			if (code === 40 || code === 41) throw new SubsonicError('Неверный логин или пароль.');
			throw new SubsonicError(body.error?.message ?? 'Неизвестная ошибка сервера');
		}
		return body;
	}

	async ping() {
		await this.call('ping');
	}

	async albumList(type: AlbumListType, size: number, offset = 0, genre?: string): Promise<Album[]> {
		const body = await this.call('getAlbumList2', { type, size, offset, genre });
		return list(body.albumList2?.album);
	}

	async album(id: string): Promise<Album> {
		const album = (await this.call('getAlbum', { id })).album;
		album.song = list(album.song);
		return album;
	}

	async artists(): Promise<Artist[]> {
		const body = await this.call('getArtists');
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		return list<any>(body.artists?.index).flatMap((index) => list<Artist>(index.artist));
	}

	async artist(id: string): Promise<Artist> {
		const artist = (await this.call('getArtist', { id })).artist;
		artist.album = list(artist.album);
		return artist;
	}

	async topSongs(artist: string, count = 10): Promise<Song[]> {
		return list((await this.call('getTopSongs', { artist, count })).topSongs?.song);
	}

	async similarSongs(id: string, count = 50): Promise<Song[]> {
		return list((await this.call('getSimilarSongs2', { id, count })).similarSongs2?.song);
	}

	async randomSongs(size: number, genre?: string): Promise<Song[]> {
		return list((await this.call('getRandomSongs', { size, genre })).randomSongs?.song);
	}

	async playlists(): Promise<Playlist[]> {
		return list((await this.call('getPlaylists')).playlists?.playlist);
	}

	async playlist(id: string): Promise<Playlist> {
		const playlist = (await this.call('getPlaylist', { id })).playlist;
		playlist.entry = list(playlist.entry);
		return playlist;
	}

	async createPlaylist(name: string, songIds: string[]) {
		await this.call('createPlaylist', { name, songId: songIds });
	}

	async addToPlaylist(id: string, songIds: string[]) {
		await this.call('updatePlaylist', { playlistId: id, songIdToAdd: songIds });
	}

	async removeFromPlaylist(id: string, index: number) {
		await this.call('updatePlaylist', { playlistId: id, songIndexToRemove: index });
	}

	async deletePlaylist(id: string) {
		await this.call('deletePlaylist', { id });
	}

	async starred() {
		const s = (await this.call('getStarred2')).starred2 ?? {};
		return { songs: list<Song>(s.song), albums: list<Album>(s.album), artists: list<Artist>(s.artist) };
	}

	async star(kind: 'song' | 'album' | 'artist', id: string, on: boolean) {
		const key = kind === 'song' ? 'id' : kind === 'album' ? 'albumId' : 'artistId';
		await this.call(on ? 'star' : 'unstar', { [key]: id });
	}

	async search(query: string) {
		const r = (await this.call('search3', { query, songCount: 50, albumCount: 24, artistCount: 12 })).searchResult3 ?? {};
		return { songs: list<Song>(r.song), albums: list<Album>(r.album), artists: list<Artist>(r.artist) };
	}

	async genres(): Promise<Genre[]> {
		return list((await this.call('getGenres')).genres?.genre);
	}

	async scrobble(id: string, submission: boolean) {
		await this.call('scrobble', { id, submission, time: Date.now() });
	}

	async lyrics(song: Song): Promise<Lyrics | null> {
		// OpenSubsonic: timed lyrics embedded in the file or stored next to it.
		try {
			const body = await this.call('getLyricsBySongId', { id: song.id });
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const all = list<any>(body.lyricsList?.structuredLyrics);
			const best = all.find((l) => l.synced) ?? all[0];
			if (best) {
				const offset = (best.offset ?? 0) / 1000;
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				const lines = list<any>(best.line)
					.map((l) => ({
						time: best.synced && l.start !== undefined ? Math.max(0, l.start / 1000 + offset) : undefined,
						text: String(l.value ?? '').trim()
					}))
					.filter((l) => best.synced || l.text);
				if (lines.length) return { synced: !!best.synced, lines, source: 'server' };
			}
		} catch {
			// Older servers do not have this endpoint; fall through.
		}
		try {
			const body = await this.call('getLyrics', { artist: song.artist, title: song.title });
			const text: string | undefined = body.lyrics?.value;
			if (text?.trim()) {
				const parsed = parseLrc(text);
				return { ...parsed, source: 'server' };
			}
		} catch {
			// No lyrics on the server.
		}
		return null;
	}

	coverUrl(id: string | undefined, size = 300): string {
		if (!id) return '';
		return this.url('getCoverArt', { id, size });
	}

	streamUrl(song: Song): string {
		// format=raw: the server sends the original file, untouched. Never transcoded.
		return this.url('stream', { id: song.id, format: 'raw' });
	}
}
