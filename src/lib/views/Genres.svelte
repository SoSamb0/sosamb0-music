<script lang="ts">
	import { app, plural } from '$lib/app.svelte';
	import { loader } from '$lib/loader.svelte';

	const genres = loader(() => app.api!.genres().then((list) => list.sort((a, b) => (b.songCount ?? 0) - (a.songCount ?? 0))));

	// A stable hue per genre name, so each tile keeps its colour between visits.
	function hue(name: string): number {
		let h = 0;
		for (const ch of name) h = (h * 31 + ch.codePointAt(0)!) % 360;
		return h;
	}
</script>

<div class="page">
	<h1>Жанры</h1>
	{#if genres.loading}
		<div class="empty"><span class="spinner"></span></div>
	{:else if !genres.value?.length}
		<div class="empty"><p>На сервере нет жанров</p></div>
	{:else}
		<div class="tiles">
			{#each genres.value as genre (genre.value)}
				<button class="tile" style:--h={hue(genre.value)} onclick={() => app.go({ name: 'genre', id: genre.value })}>
					<span class="name">{genre.value}</span>
					<span class="count">{plural(genre.songCount ?? 0, 'трек', 'трека', 'треков')}</span>
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
		gap: 12px;
		margin-top: 22px;
	}

	.tile {
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		align-items: flex-start;
		height: 104px;
		padding: 14px 16px;
		border-radius: var(--radius);
		background: linear-gradient(135deg, hsl(var(--h) 55% 34%), hsl(calc(var(--h) + 40) 50% 18%));
		text-align: left;
		transition:
			transform 0.2s var(--ease),
			filter 0.2s;
	}

	.tile:hover {
		transform: translateY(-2px);
		filter: brightness(1.12);
	}

	.name {
		font: 650 17px/1.2 'Segoe UI Variable Display', 'Segoe UI', system-ui, sans-serif;
		overflow-wrap: anywhere;
	}

	.count {
		font-size: 12.5px;
		color: rgb(255 255 255 / 0.72);
	}
</style>
