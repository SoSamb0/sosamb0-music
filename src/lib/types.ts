export interface ReplayGain {
	trackGain?: number;
	albumGain?: number;
	trackPeak?: number;
	albumPeak?: number;
}

export interface Song {
	id: string;
	title: string;
	artist?: string;
	artistId?: string;
	album?: string;
	albumId?: string;
	coverArt?: string;
	duration?: number;
	track?: number;
	discNumber?: number;
	year?: number;
	genre?: string;
	suffix?: string;
	bitRate?: number;
	bitDepth?: number;
	samplingRate?: number;
	channelCount?: number;
	starred?: string;
	replayGain?: ReplayGain;
}

export interface Album {
	id: string;
	name: string;
	/** MusicBrainz release id (OpenSubsonic). */
	musicBrainzId?: string;
	artist?: string;
	artistId?: string;
	coverArt?: string;
	songCount?: number;
	duration?: number;
	year?: number;
	genre?: string;
	starred?: string;
	song?: Song[];
}

export interface Artist {
	id: string;
	name: string;
	coverArt?: string;
	artistImageUrl?: string;
	albumCount?: number;
	starred?: string;
	album?: Album[];
}

export interface Playlist {
	id: string;
	name: string;
	songCount?: number;
	duration?: number;
	coverArt?: string;
	owner?: string;
	comment?: string;
	entry?: Song[];
}

export interface Genre {
	value: string;
	songCount?: number;
	albumCount?: number;
}

export interface LyricLine {
	/** Seconds from the start of the track; undefined for unsynced text. */
	time?: number;
	text: string;
}

export interface Lyrics {
	synced: boolean;
	lines: LyricLine[];
	source: 'server' | 'lrclib';
}

export type AlbumListType = 'newest' | 'recent' | 'frequent' | 'random' | 'alphabeticalByName' | 'starred' | 'byGenre';

/** Everything the UI needs from a music server. Implemented by Subsonic and by the demo library. */
export interface MusicApi {
	readonly id: string;
	ping(): Promise<void>;
	albumList(type: AlbumListType, size: number, offset?: number, genre?: string): Promise<Album[]>;
	album(id: string): Promise<Album>;
	artists(): Promise<Artist[]>;
	artist(id: string): Promise<Artist>;
	topSongs(artist: string, count?: number): Promise<Song[]>;
	similarSongs(id: string, count?: number): Promise<Song[]>;
	randomSongs(size: number, genre?: string): Promise<Song[]>;
	playlists(): Promise<Playlist[]>;
	playlist(id: string): Promise<Playlist>;
	createPlaylist(name: string, songIds: string[]): Promise<void>;
	addToPlaylist(id: string, songIds: string[]): Promise<void>;
	removeFromPlaylist(id: string, index: number): Promise<void>;
	deletePlaylist(id: string): Promise<void>;
	starred(): Promise<{ songs: Song[]; albums: Album[]; artists: Artist[] }>;
	star(kind: 'song' | 'album' | 'artist', id: string, on: boolean): Promise<void>;
	search(query: string): Promise<{ songs: Song[]; albums: Album[]; artists: Artist[] }>;
	genres(): Promise<Genre[]>;
	scrobble(id: string, submission: boolean): Promise<void>;
	lyrics(song: Song): Promise<Lyrics | null>;
	coverUrl(id: string | undefined, size?: number): string;
	streamUrl(song: Song): string;
}
