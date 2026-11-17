<script lang="ts">
	import { SvelteSet } from 'svelte/reactivity';
	import { app, formatTime } from '$lib/app.svelte';
	import { drag, endDrag, reorder, startDrag, trackLabel } from '$lib/dnd.svelte';
	import { menu, playlistItems, songMenu, songsMenu, type MenuItem } from '$lib/menu.svelte';
	import { isLossless, player } from '$lib/player.svelte';
	import type { Song } from '$lib/types';
	import Cover from './Cover.svelte';
	import Icon from './Icon.svelte';

	let {
		songs,
		numbering = 'index',
		showCover = true,
		showAlbum = true,
		showArtist = true,
		extraMenu,
		onremove,
		onreorder
	}: {
		songs: Song[];
		numbering?: 'index' | 'track';
		showCover?: boolean;
		showAlbum?: boolean;
		showArtist?: boolean;
		extraMenu?: (song: Song, index: number) => MenuItem[];
		/** When set, selected rows can be removed (positions in `songs`). */
		onremove?: (indexes: number[]) => void;
		/** When set, rows can be dragged into a new order; receives the old indexes in the new order. */
		onreorder?: (order: number[]) => void;
	} = $props();

	// Drag and drop: rows can be dragged to a playlist, the queue, or (with onreorder) within the list.
	const source = Symbol('song-list');
	let dropAt = $state<number | null>(null);

	function dragStart(e: DragEvent, i: number) {
		if (!onreorder) return;
		if (!selected.has(i)) {
			selected.clear();
			selected.add(i);
			anchor = i;
			multi = false;
		}
		const indexes = [...selected].sort((a, b) => a - b);
		const tracks = indexes.map((k) => songs[k]);
		startDrag(e, { label: tracks.length === 1 ? tracks[0].title : trackLabel(tracks.length), load: () => tracks, source, indexes });
	}

	function dragOver(e: DragEvent, i: number) {
		if (!onreorder || drag.payload?.source !== source) return;
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		dropAt = e.clientY < r.top + r.height / 2 ? i : i + 1;
	}

	function dropOnList(e: DragEvent) {
		const moving = drag.payload?.indexes;
		if (dropAt === null || !moving || drag.payload?.source !== source) return;
		e.preventDefault();
		const order = reorder(songs.length, moving, dropAt);
		dropAt = null;
		endDrag();
		selected.clear();
		if (order.some((v, k) => v !== k)) onreorder?.(order);
	}

	function dragEnd() {
		dropAt = null;
		endDrag();
	}

	const currentId = $derived(player.current?.id);

	// Selection works like a file manager: click, Ctrl+click, Shift+click.
	const selected = new SvelteSet<number>();
	let anchor: number | null = null;
	/** Set once the user selects on purpose (checkbox, Ctrl, Shift); a plain click only highlights. */
	let multi = $state(false);
	const showBar = $derived(selected.size > 1 || (multi && selected.size > 0));
	const picked = $derived([...selected].sort((a, b) => a - b).map((i) => songs[i]).filter(Boolean));

	// A new list (another album, a refreshed playlist) starts with nothing selected.
	const listKey = $derived(songs[0]?.id ?? '');
	$effect(() => {
		listKey;
		selected.clear();
		anchor = null;
		multi = false;
	});
	// Rows removed underneath the selection.
	$effect(() => {
		const n = songs.length;
		for (const i of [...selected]) if (i >= n) selected.delete(i);
	});

	function quality(song: Song): string | null {
		if (!isLossless(song)) return null;
		return (song.bitDepth ?? 16) > 16 || (song.samplingRate ?? 44100) > 48000 ? 'Hi-Res' : 'Lossless';
	}

	function toggle(i: number) {
		if (selected.has(i)) selected.delete(i);
		else selected.add(i);
		anchor = i;
		multi = true;
	}

	function rowClick(e: MouseEvent, i: number) {
		// Buttons and links inside the row do their own thing.
		if ((e.target as HTMLElement).closest('button')) return;
		if (e.shiftKey && anchor !== null) {
			const [a, b] = anchor < i ? [anchor, i] : [i, anchor];
			if (!e.ctrlKey) selected.clear();
			for (let k = a; k <= b; k++) selected.add(k);
			multi = true;
		} else if (e.ctrlKey || e.metaKey) {
			toggle(i);
		} else {
			selected.clear();
			selected.add(i);
			anchor = i;
			multi = false;
		}
	}

	function openMenu(e: MouseEvent, song: Song, i: number) {
		if (selected.size > 1 && selected.has(i)) {
			menu.open(e, songsMenu(picked, removeItem(), () => selected.clear()));
		} else {
			if (!selected.has(i)) {
				selected.clear();
				selected.add(i);
				anchor = i;
			}
			menu.open(e, songMenu(song, extraMenu?.(song, i) ?? []));
		}
	}

	function removeItem(): MenuItem[] {
		if (!onremove) return [];
		return [{ label: '', separator: true }, { label: `Убрать из списка (${selected.size})`, icon: 'trash', danger: true, action: removeSelected }];
	}

	function removeSelected() {
		const indexes = [...selected];
		selected.clear();
		onremove?.(indexes);
	}

	/** Select exactly these rows (e.g. the duplicates a page found). */
	export function selectIndexes(indexes: number[]) {
		selected.clear();
		for (const i of indexes) selected.add(i);
		multi = true;
		anchor = indexes[0] ?? null;
		// Bring the first one into view.
		queueMicrotask(() => document.querySelectorAll('.list .row.selected')[0]?.scrollIntoView({ block: 'center', behavior: 'smooth' }));
	}

	/** Select every track of the list (also exported for "select all" buttons on pages). */
	export function selectAll() {
		songs.forEach((_, i) => selected.add(i));
		multi = true;
	}

	function run(action: () => void) {
		action();
		selected.clear();
	}

	function onKey(e: KeyboardEvent) {
		if (selected.size === 0 || (e.target instanceof Element && e.target.closest('input, textarea, select'))) return;
		if (e.key === 'Escape') {
			selected.clear();
			e.stopPropagation();
		} else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a') {
			selectAll();
			e.preventDefault();
		} else if (e.key === 'Delete' && onremove) {
			removeSelected();
			e.preventDefault();
		}
	}
</script>

<svelte:window onkeydown={onKey} />

<div class="list" class:with-album={showAlbum} class:selecting={showBar} role="list">
	{#each songs as song, i (song.id + ':' + i)}
		{@const active = song.id === currentId}
		<!-- Clicking the row selects it with the mouse; from the keyboard the checkbox button does it. -->
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
		<div
			class="row"
			class:active
			class:selected={selected.has(i)}
			class:drop-before={dropAt === i}
			class:drop-after={dropAt === songs.length && i === songs.length - 1}
			role="listitem"
			draggable={onreorder ? 'true' : undefined}
			ondragstart={(e) => dragStart(e, i)}
			ondragover={(e) => dragOver(e, i)}
			ondrop={dropOnList}
			ondragend={dragEnd}
			onclick={(e) => rowClick(e, i)}
			ondblclick={(e) => !(e.target as HTMLElement).closest('button') && player.play(songs, i)}
			oncontextmenu={(e) => openMenu(e, song, i)}
		>
			<button class="check" onclick={() => toggle(i)} aria-label={selected.has(i) ? 'Снять выбор' : 'Выбрать'} aria-pressed={selected.has(i)}>
				<Icon name={selected.has(i) ? 'checked' : 'checkbox'} size={17} />
			</button>

			<button class="num tnum" onclick={() => (active ? player.toggle() : player.play(songs, i))} aria-label="Воспроизвести {song.title}">
				{#if active && player.playing}
					<span class="bars" aria-hidden="true"><i></i><i></i><i></i></span>
				{:else}
					<span class="n">{numbering === 'track' ? (song.track ?? i + 1) : i + 1}</span>
				{/if}
				<span class="p"><Icon name={active && player.playing ? 'pause' : 'play'} size={15} /></span>
			</button>

			<div class="main">
				{#if showCover}
					<div class="thumb"><Cover id={song.coverArt} size={80} /></div>
				{/if}
				<div class="text">
					<div class="title ellipsis">{song.title}</div>
					{#if showArtist && song.artist}
						<button class="artist ellipsis" onclick={() => song.artistId && app.go({ name: 'artist', id: song.artistId })}>{song.artist}</button>
					{/if}
				</div>
			</div>

			{#if showAlbum}
				<button class="album ellipsis" onclick={() => song.albumId && app.go({ name: 'album', id: song.albumId })}>{song.album ?? ''}</button>
			{/if}

			<div class="q">
				{#if quality(song)}<span class="badge" class:hi={quality(song) === 'Hi-Res'}>{quality(song)}</span>{/if}
			</div>

			<button
				class="icon-btn heart"
				class:on={!!song.starred}
				onclick={() => player.toggleStar(song)}
				aria-label={song.starred ? 'Убрать из избранного' : 'В избранное'}
			>
				<Icon name={song.starred ? 'heartFill' : 'heart'} size={16} />
			</button>
			<div class="dur tnum">{formatTime(song.duration)}</div>
			<button class="icon-btn more" onclick={(e) => openMenu(e, song, i)} aria-label="Ещё">
				<Icon name="more" size={17} />
			</button>
		</div>
	{/each}

	{#if showBar}
		<div class="bar" role="toolbar" aria-label="Действия с выбранными треками">
			<span class="count">Выбрано: {selected.size}</span>
			{#if selected.size < songs.length}
				<button class="btn small" onclick={selectAll} title="Ctrl+A">Выбрать все ({songs.length})</button>
			{/if}
			<button class="btn small primary" onclick={() => run(() => player.play(picked))}><Icon name="play" size={14} />Играть</button>
			<button class="btn small" onclick={() => run(() => player.playNext(picked))}>Следующими</button>
			<button class="btn small" onclick={() => run(() => player.enqueue(picked))}>В очередь</button>
			<button class="btn small" onclick={(e) => menu.open(e, playlistItems(() => picked, () => selected.clear()))}><Icon name="playlist" size={14} />В плейлист</button>
			<button class="icon-btn" onclick={() => run(() => player.starMany(picked, true))} aria-label="В избранное" title="В избранное"><Icon name="heart" size={16} /></button>
			{#if onremove}
				<button class="icon-btn danger" onclick={removeSelected} aria-label="Убрать из списка" title="Убрать из списка"><Icon name="trash" size={16} /></button>
			{/if}
			<span class="spacer"></span>
			<button class="icon-btn" onclick={() => selected.clear()} aria-label="Снять выбор" title="Снять выбор (Esc)"><Icon name="x" size={16} /></button>
		</div>
	{/if}
</div>

<style>
	.list {
		display: flex;
		flex-direction: column;
	}

	.row {
		display: grid;
		grid-template-columns: 24px 38px minmax(0, 1fr) 76px 34px 52px 34px;
		align-items: center;
		gap: 8px;
		height: 54px;
		padding: 0 10px 0 4px;
		border-radius: var(--radius-sm);
		transition: background 0.12s;
		content-visibility: auto;
		contain-intrinsic-size: auto 54px;
	}

	.with-album .row {
		grid-template-columns: 24px 38px minmax(0, 1.3fr) minmax(0, 1fr) 76px 34px 52px 34px;
	}

	.row:hover {
		background: var(--surface);
	}

	.row {
		position: relative;
	}

	/* Where dragged rows will land. */
	.row.drop-before::before,
	.row.drop-after::after {
		content: '';
		position: absolute;
		left: 8px;
		right: 8px;
		height: 2px;
		border-radius: 2px;
		background: var(--accent);
		box-shadow: 0 0 8px var(--accent);
	}

	.row.drop-before::before {
		top: -1px;
	}

	.row.drop-after::after {
		bottom: -1px;
	}

	.row.active {
		background: var(--accent-soft);
	}

	.row.selected {
		background: rgb(var(--accent-rgb) / 0.22);
	}

	.check {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		color: var(--faint);
		opacity: 0;
		transition: opacity 0.12s;
	}

	.row:hover .check,
	.selecting .check,
	.check:focus-visible {
		opacity: 1;
	}

	.row.selected .check {
		color: var(--accent);
	}

	.num {
		position: relative;
		display: grid;
		place-items: center;
		width: 38px;
		height: 38px;
		color: var(--faint);
		font-size: 13px;
	}

	.num .p {
		position: absolute;
		display: none;
		color: var(--text);
	}

	.row:hover .num .p {
		display: block;
	}

	.row:hover .num .n,
	.row:hover .num .bars {
		visibility: hidden;
	}

	.row.active .num {
		color: var(--accent);
	}

	.bars {
		display: flex;
		align-items: flex-end;
		gap: 2px;
		height: 13px;
	}

	.bars i {
		width: 3px;
		border-radius: 1px;
		background: var(--accent);
		animation: bounce 0.9s ease-in-out infinite;
	}

	.bars i:nth-child(2) {
		animation-delay: -0.3s;
	}

	.bars i:nth-child(3) {
		animation-delay: -0.6s;
	}

	@keyframes bounce {
		0%,
		100% {
			height: 4px;
		}
		50% {
			height: 13px;
		}
	}

	.main {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}

	.thumb {
		flex: none;
		width: 40px;
		border-radius: 6px;
		overflow: hidden;
	}

	.text {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		min-width: 0;
	}

	.title {
		max-width: 100%;
		font-weight: 550;
	}

	.row.active .title {
		color: var(--accent);
	}

	.artist,
	.album {
		max-width: 100%;
		color: var(--muted);
		font-size: 13px;
		text-align: left;
	}

	.artist:hover,
	.album:hover {
		color: var(--text);
		text-decoration: underline;
	}

	.q {
		display: flex;
		justify-content: flex-end;
	}

	.heart:not(.on),
	.more {
		opacity: 0;
	}

	.row:hover .heart,
	.row:hover .more,
	.heart:focus-visible,
	.more:focus-visible {
		opacity: 1;
	}

	.dur {
		color: var(--muted);
		font-size: 13px;
		text-align: right;
	}

	.bar {
		position: sticky;
		bottom: 12px;
		z-index: 5;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		margin-top: 10px;
		padding: 8px 8px 8px 16px;
		border-radius: 14px;
		background: rgb(24 26 34 / 0.94);
		backdrop-filter: blur(20px);
		border: 1px solid var(--line);
		box-shadow: 0 12px 34px rgb(0 0 0 / 0.5);
		animation: bar-in 0.2s var(--ease);
	}

	@keyframes bar-in {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}

	.count {
		margin-right: 8px;
		font-weight: 600;
		font-size: 13px;
	}

	.spacer {
		flex: 1;
	}

	.icon-btn.danger:hover {
		color: var(--danger);
	}

	@container main (max-width: 720px) {
		.with-album .row,
		.row {
			grid-template-columns: 24px 38px minmax(0, 1fr) 34px 52px 34px;
		}

		.album,
		.q {
			display: none;
		}
	}

	/* Phone: number, title, length and "⋯". The heart and selection live in the long-press menu. */
	@container main (max-width: 480px) {
		.with-album .row,
		.row {
			grid-template-columns: 28px minmax(0, 1fr) 40px 34px;
			gap: 6px;
			height: 58px;
			padding: 0 0 0 2px;
		}

		.row > .check,
		.row > .heart {
			display: none;
		}

		.selecting .row > .check {
			display: grid;
		}

		.selecting .row {
			grid-template-columns: 28px 28px minmax(0, 1fr) 40px 34px;
		}
	}
</style>
