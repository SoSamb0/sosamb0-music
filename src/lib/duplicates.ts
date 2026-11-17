// Finding repeated tracks in a playlist.

import type { Song } from './types';

/** "Song Title (Remastered 2011) [Live]" and "song title - 2011 remaster" both become "song title". */
export function normalizeTitle(title: string): string {
	return title
		.toLowerCase()
		.replace(/\s*[([].*?[)\]]/g, '')
		.replace(/\s+-\s+.*(remaster|version|edit|mix|live|mono|stereo|demo).*$/i, '')
		.replace(/\bfeat\.?.*$/i, '')
		.replace(/[^\p{L}\p{N}]+/gu, ' ')
		.trim();
}

function normalizeArtist(artist: string | undefined): string {
	return (artist ?? '')
		.toLowerCase()
		.split(/\s*(?:,|&|feat\.?|ft\.?|\bx\b)\s*/)[0]
		.replace(/[^\p{L}\p{N}]+/gu, ' ')
		.trim();
}

export interface Duplicates {
	/** Later copies of exactly the same track (same file on the server). */
	exact: number[];
	/** Later copies of the same song from another album or release. */
	similar: number[];
}

/** Positions of every repeat after its first appearance. */
export function findDuplicates(songs: Song[]): Duplicates {
	const ids = new Set<string>();
	const names = new Map<string, string>();
	const exact: number[] = [];
	const similar: number[] = [];
	songs.forEach((song, i) => {
		if (ids.has(song.id)) {
			exact.push(i);
			return;
		}
		ids.add(song.id);
		const key = `${normalizeArtist(song.artist)}|${normalizeTitle(song.title)}`;
		const first = names.get(key);
		if (first !== undefined && first !== song.id) similar.push(i);
		else names.set(key, song.id);
	});
	return { exact, similar };
}
