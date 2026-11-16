<script lang="ts">
	import { untrack } from 'svelte';
	import { app } from '$lib/app.svelte';
	import AlbumCard from '$lib/components/AlbumCard.svelte';
	import type { Album, AlbumListType } from '$lib/types';

	let { genre }: { genre?: string } = $props();

	const SORTS: { type: AlbumListType; label: string }[] = [
		{ type: 'newest', label: 'Новые' },
		{ type: 'alphabeticalByName', label: 'По алфавиту' },
		{ type: 'frequent', label: 'Часто слушаемые' },
		{ type: 'recent', label: 'Недавно слушали' },
		{ type: 'random', label: 'Случайные' }
	];
	const PAGE = 60;

	let sort = $state<AlbumListType>('newest');
	let albums = $state<Album[]>([]);
	let loading = $state(false);
	let done = $state(false);
	let sentinel = $state<HTMLElement>();
	let run = 0;

	const type = $derived<AlbumListType>(genre ? 'byGenre' : sort);

	async function more() {
		if (loading || done) return;
		loading = true;
		const mine = run;
		const page = await app.attempt(() => app.api!.albumList(type, PAGE, albums.length, genre));
		if (mine !== run) return;
		loading = false;
		if (!page || page.length < PAGE) done = true;
		if (page) {
			// Random pages can repeat albums; keep each one once.
			const seen = new Set(albums.map((a) => a.id));
			albums.push(...page.filter((a) => !seen.has(a.id)));
		}
	}

	// Start over when the sort order or genre changes.
	$effect(() => {
		type;
		genre;
		// Only the sort order and genre should restart the list, not the paging state.
		untrack(() => {
			run++;
			albums = [];
			done = false;
			loading = false;
			more();
		});
	});

	// Load the next page as the end of the grid approaches.
	$effect(() => {
		if (!sentinel) return;
		const observer = new IntersectionObserver((entries) => entries[0].isIntersecting && more(), { rootMargin: '600px' });
		observer.observe(sentinel);
		return () => observer.disconnect();
	});
</script>

<div class="page">
	<h1>{genre ?? 'Альбомы'}</h1>
	{#if !genre}
		<div class="sorts">
			{#each SORTS as s (s.type)}
				<button class="chip" class:on={sort === s.type} onclick={() => (sort = s.type)}>{s.label}</button>
			{/each}
		</div>
	{/if}

	<div class="grid">
		{#each albums as album (album.id)}<AlbumCard {album} />{/each}
	</div>
	{#if !loading && done && albums.length === 0}
		<div class="empty"><p>Здесь пока нет альбомов</p></div>
	{/if}
	<div bind:this={sentinel} class="sentinel">{#if loading}<span class="spinner"></span>{/if}</div>
</div>

<style>
	.sorts {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin: 16px 0 4px;
	}

	.grid {
		margin-top: 20px;
	}

	.sentinel {
		display: grid;
		place-items: center;
		height: 70px;
	}
</style>
