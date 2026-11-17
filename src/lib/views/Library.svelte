<script lang="ts">
	import { app, type Route } from '$lib/app.svelte';
	import Icon from '$lib/components/Icon.svelte';

	const ITEMS: { route: Route; label: string; icon: string }[] = [
		{ route: { name: 'albums' }, label: 'Альбомы', icon: 'album' },
		{ route: { name: 'songs' }, label: 'Треки', icon: 'note' },
		{ route: { name: 'artists' }, label: 'Исполнители', icon: 'artist' },
		{ route: { name: 'genres' }, label: 'Жанры', icon: 'genre' },
		{ route: { name: 'favorites' }, label: 'Избранное', icon: 'heart' },
		{ route: { name: 'stats' }, label: 'Статистика', icon: 'stats' },
		{ route: { name: 'settings' }, label: 'Настройки', icon: 'settings' }
	];
</script>

<div class="page">
	<h1>Библиотека</h1>
	<div class="list">
		{#each ITEMS as item (item.label)}
			<button class="row" onclick={() => app.go(item.route)}>
				<span class="ico"><Icon name={item.icon} size={20} /></span>
				<span class="grow">{item.label}</span>
				<Icon name="right" size={16} />
			</button>
		{/each}
	</div>
</div>

<style>
	.list {
		display: flex;
		flex-direction: column;
		margin-top: 12px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 14px;
		height: 56px;
		padding: 0 6px;
		border-bottom: 1px solid var(--line);
		font-size: 16px;
		text-align: left;
		color: var(--faint);
	}

	.row .grow {
		flex: 1;
		color: var(--text);
	}

	.ico {
		display: grid;
		place-items: center;
		width: 38px;
		height: 38px;
		border-radius: 10px;
		background: var(--surface);
		color: var(--accent);
	}
</style>
