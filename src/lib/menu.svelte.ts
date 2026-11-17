import { app } from './app.svelte';
import { player } from './player.svelte';
import type { Album, Song } from './types';

export interface MenuItem {
	label: string;
	icon?: string;
	action?: () => void;
	children?: MenuItem[];
	danger?: boolean;
	separator?: boolean;
}

/** What a menu is about, shown on top of the phone's menu sheet. */
export interface MenuHeader {
	title: string;
	subtitle?: string;
	/** Cover art id. */
	cover?: string;
	round?: boolean;
}

class Menu {
	items = $state<MenuItem[]>([]);
	header = $state<MenuHeader | null>(null);
	x = $state(0);
	y = $state(0);
	visible = $state(false);

	open(event: MouseEvent, items: MenuItem[], header: MenuHeader | null = null) {
		event.preventDefault();
		event.stopPropagation();
		this.items = items;
		this.header = header;
		this.x = event.clientX;
		this.y = event.clientY;
		this.visible = true;
	}

	close() {
		this.visible = false;
	}
}

export const menu = new Menu();

/** "Add to playlist" entries. `after` runs once the tracks were added. */
export function playlistItems(songs: () => Promise<Song[]> | Song[], after?: () => void): MenuItem[] {
	const add = async (id: string, name: string) => {
		const list = await songs();
		const ok = await app.attempt(async () => {
			await app.api!.addToPlaylist(id, list.map((s) => s.id));
			return true;
		});
		if (ok) {
			app.toast(list.length === 1 ? `Добавлено в «${name}»` : `Добавлено в «${name}»: ${list.length}`);
			app.refreshPlaylists();
			after?.();
		}
	};
	return [
		{
			label: 'Новый плейлист…',
			icon: 'plus',
			action: async () => {
				const name = await app.ask('Название плейлиста');
				if (!name?.trim()) return;
				const list = await songs();
				const ok = await app.attempt(async () => {
					await app.api!.createPlaylist(name.trim(), list.map((s) => s.id));
					return true;
				});
				if (ok) {
					app.toast(`Плейлист «${name.trim()}» создан`);
					app.refreshPlaylists();
					after?.();
				}
			}
		},
		...app.playlists.map((p) => ({ label: p.name, action: () => add(p.id, p.name) }))
	];
}

export const songHeader = (song: Song): MenuHeader => ({
	title: song.title,
	subtitle: [song.artist, song.album].filter(Boolean).join(' · '),
	cover: song.coverArt
});

export const albumHeader = (album: Album): MenuHeader => ({
	title: album.name,
	subtitle: [album.artist, album.year].filter(Boolean).join(' · '),
	cover: album.coverArt
});

export function songMenu(song: Song, extra: MenuItem[] = []): MenuItem[] {
	return [
		{ label: 'Играть следующим', icon: 'next', action: () => player.playNext([song]) },
		{ label: 'Добавить в очередь', icon: 'queue', action: () => player.enqueue([song]) },
		{ label: 'Радио по треку', icon: 'radio', action: () => player.radio(song) },
		{ label: '', separator: true },
		{ label: song.starred ? 'Убрать из избранного' : 'В избранное', icon: 'heart', action: () => player.toggleStar(song) },
		{ label: 'Добавить в плейлист', icon: 'playlist', children: playlistItems(() => [song]) },
		{ label: '', separator: true },
		...(song.albumId ? [{ label: 'Перейти к альбому', icon: 'album', action: () => app.go({ name: 'album', id: song.albumId! }) }] : []),
		...(song.artistId ? [{ label: 'Перейти к исполнителю', icon: 'artist', action: () => app.go({ name: 'artist', id: song.artistId! }) }] : []),
		...extra
	];
}

/** Actions for several selected tracks at once. */
export function songsMenu(songs: Song[], extra: MenuItem[] = [], after?: () => void): MenuItem[] {
	const n = songs.length;
	const then = (fn: () => unknown) => () => {
		fn();
		after?.();
	};
	return [
		{ label: `Играть выбранные (${n})`, icon: 'play', action: then(() => player.play(songs)) },
		{ label: 'Играть следующими', icon: 'next', action: then(() => player.playNext(songs)) },
		{ label: 'Добавить в очередь', icon: 'queue', action: then(() => player.enqueue(songs)) },
		{ label: '', separator: true },
		{ label: 'В избранное', icon: 'heart', action: then(() => player.starMany(songs, true)) },
		{ label: 'Убрать из избранного', icon: 'heart', action: then(() => player.starMany(songs, false)) },
		{ label: 'Добавить в плейлист', icon: 'playlist', children: playlistItems(() => songs, after) },
		...extra
	];
}

export function albumMenu(album: Album): MenuItem[] {
	const songs = async () => album.song ?? (await app.api!.album(album.id)).song ?? [];
	return [
		{ label: 'Воспроизвести', icon: 'play', action: async () => player.play(await songs()) },
		{ label: 'Перемешать', icon: 'shuffle', action: async () => player.play(await songs(), 0, true) },
		{ label: 'Играть следующим', icon: 'next', action: async () => player.playNext(await songs()) },
		{ label: 'Добавить в очередь', icon: 'queue', action: async () => player.enqueue(await songs()) },
		{ label: '', separator: true },
		{ label: 'Добавить в плейлист', icon: 'playlist', children: playlistItems(songs) },
		...(album.artistId ? [{ label: 'Перейти к исполнителю', icon: 'artist', action: () => app.go({ name: 'artist', id: album.artistId! }) }] : [])
	];
}
