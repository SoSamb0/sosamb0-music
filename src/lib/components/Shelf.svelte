<script lang="ts">
	import { app } from '$lib/app.svelte';
	import { player } from '$lib/player.svelte';
	import type { Album } from '$lib/types';
	import AlbumCard from './AlbumCard.svelte';
	import Icon from './Icon.svelte';

	// A horizontal row of albums with scroll arrows and "play the whole row".
	let { title, albums, onrefresh }: { title: string; albums: Album[]; onrefresh?: () => void } = $props();

	let row = $state<HTMLElement>();
	let atStart = $state(true);
	let atEnd = $state(false);
	let busy = $state(false);

	function update() {
		if (!row) return;
		// Scroll snapping can leave the row a few pixels off zero.
		atStart = row.scrollLeft <= 8;
		atEnd = row.scrollLeft + row.clientWidth >= row.scrollWidth - 8;
	}

	$effect(() => {
		albums;
		queueMicrotask(update);
	});

	function scroll(dir: 1 | -1) {
		row?.scrollBy({ left: dir * row.clientWidth * 0.85, behavior: 'smooth' });
	}

	// Every album of the row, one after another.
	async function playAll(shuffled: boolean) {
		busy = true;
		const full = await app.attempt(() => Promise.all(albums.map((a) => app.api!.album(a.id))));
		busy = false;
		const songs = full?.flatMap((a) => a.song ?? []) ?? [];
		if (songs.length) player.play(songs, 0, shuffled);
	}
</script>

<svelte:window onresize={update} />

<div class="section-head">
	<h2>{title}</h2>
	<div class="tools">
		<button class="btn small" onclick={() => playAll(false)} disabled={busy}>
			{#if busy}<span class="spinner tiny"></span>{:else}<Icon name="play" size={14} />{/if}Слушать всё
		</button>
		<button class="icon-btn" onclick={() => playAll(true)} disabled={busy} aria-label="Перемешать всё" title="Перемешать всё"><Icon name="shuffle" size={16} /></button>
		{#if onrefresh}
			<button class="icon-btn" onclick={onrefresh} aria-label="Обновить подборку" title="Обновить подборку"><Icon name="refresh" size={16} /></button>
		{/if}
		<button class="icon-btn arrow" onclick={() => scroll(-1)} disabled={atStart} aria-label="Прокрутить назад"><Icon name="left" /></button>
		<button class="icon-btn arrow" onclick={() => scroll(1)} disabled={atEnd} aria-label="Прокрутить вперёд"><Icon name="right" /></button>
	</div>
</div>
<div class="shelf" bind:this={row} onscroll={update}>
	{#each albums as album (album.id)}<div class="cell"><AlbumCard {album} /></div>{/each}
</div>

<style>
	.tools {
		display: flex;
		align-items: center;
		gap: 2px;
	}

	.tools .btn {
		margin-right: 6px;
	}

	.spinner.tiny {
		width: 12px;
		height: 12px;
		border-width: 2px;
	}

	.shelf {
		display: grid;
		grid-auto-flow: column;
		grid-auto-columns: minmax(150px, 172px);
		gap: 18px;
		overflow-x: auto;
		padding: 6px 4px 14px;
		margin: -6px -4px 0;
		scroll-snap-type: x proximity;
		/* Hidden until hovered. (The standard property overrides ::-webkit-scrollbar in Chromium.) */
		scrollbar-width: none;
		scrollbar-color: rgb(255 255 255 / 0.22) transparent;
	}

	.shelf:hover {
		scrollbar-width: thin;
	}

	.cell {
		scroll-snap-align: start;
		min-width: 0;
	}

	/* Phones: rows are swiped, so no arrows; "play all" becomes a round button. */
	@container main (max-width: 560px) {
		.arrow {
			display: none;
		}

		.tools .btn {
			width: 36px;
			padding: 0;
			justify-content: center;
			font-size: 0;
			gap: 0;
			margin-right: 2px;
		}

		.shelf {
			grid-auto-columns: 138px;
			gap: 12px;
			margin: -6px -16px 0;
			padding: 6px 16px 10px;
			scroll-padding: 0 16px;
		}
	}
</style>
