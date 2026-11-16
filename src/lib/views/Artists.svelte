<script lang="ts">
	import { app, plural } from '$lib/app.svelte';
	import Cover from '$lib/components/Cover.svelte';
	import { loader } from '$lib/loader.svelte';

	const artists = loader(() => app.api!.artists());
	let filter = $state('');

	const shown = $derived.by(() => {
		const q = filter.trim().toLowerCase();
		const list = artists.value ?? [];
		return q ? list.filter((a) => a.name.toLowerCase().includes(q)) : list;
	});
</script>

<div class="page">
	<div class="head">
		<h1>Исполнители</h1>
		<input class="field filter" bind:value={filter} placeholder="Фильтр" aria-label="Фильтр исполнителей" onkeydown={(e) => e.stopPropagation()} />
	</div>

	{#if artists.loading}
		<div class="empty"><span class="spinner"></span></div>
	{:else if shown.length === 0}
		<div class="empty"><p>Никого не найдено</p></div>
	{:else}
		<div class="grid">
			{#each shown as artist (artist.id)}
				<button class="artist" onclick={() => app.go({ name: 'artist', id: artist.id })}>
					<div class="art"><Cover id={artist.coverArt} size={300} round icon="artist" /></div>
					<div class="name ellipsis">{artist.name}</div>
					{#if artist.albumCount}<div class="sub">{plural(artist.albumCount, 'альбом', 'альбома', 'альбомов')}</div>{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 22px;
	}

	.filter {
		width: 220px;
		height: 36px;
		border-radius: 999px;
	}

	.artist {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		min-width: 0;
		padding: 12px 8px;
		border-radius: var(--radius);
		text-align: center;
		transition: background 0.15s;
		content-visibility: auto;
		contain-intrinsic-size: auto 230px;
	}

	.artist:hover {
		background: var(--surface);
	}

	.art {
		width: 82%;
		margin-bottom: 10px;
		border-radius: 50%;
		box-shadow: 0 8px 24px rgb(0 0 0 / 0.35);
	}

	.name {
		max-width: 100%;
		font-weight: 600;
	}

	.sub {
		font-size: 13px;
		color: var(--muted);
	}
</style>
