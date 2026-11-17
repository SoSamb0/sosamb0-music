<script lang="ts">
	import { app, type Route } from '$lib/app.svelte';
	import { menu } from '$lib/menu.svelte';
	import Icon from './Icon.svelte';
	import LogoMark from './LogoMark.svelte';

	const NAV: { route: Route; label: string; icon: string; match: string[] }[] = [
		{ route: { name: 'home' }, label: 'Главная', icon: 'home', match: ['home'] },
		{ route: { name: 'albums' }, label: 'Альбомы', icon: 'album', match: ['albums', 'album'] },
		{ route: { name: 'artists' }, label: 'Исполнители', icon: 'artist', match: ['artists', 'artist'] },
		{ route: { name: 'genres' }, label: 'Жанры', icon: 'genre', match: ['genres', 'genre'] },
		{ route: { name: 'favorites' }, label: 'Избранное', icon: 'heart', match: ['favorites'] },
		{ route: { name: 'stats' }, label: 'Статистика', icon: 'stats', match: ['stats'] }
	];

	const current = $derived(app.route);

	async function create() {
		const name = await app.ask('Название плейлиста');
		if (!name?.trim()) return;
		const ok = await app.attempt(async () => {
			await app.api!.createPlaylist(name.trim(), []);
			return true;
		});
		if (ok) app.refreshPlaylists();
	}

	function playlistMenu(e: MouseEvent, id: string, name: string) {
		menu.open(e, [
			{ label: 'Открыть', icon: 'playlist', action: () => app.go({ name: 'playlist', id }) },
			{
				label: 'Удалить плейлист',
				icon: 'trash',
				danger: true,
				action: async () => {
					const ok = await app.attempt(async () => {
						await app.api!.deletePlaylist(id);
						return true;
					});
					if (!ok) return;
					app.toast(`Плейлист «${name}» удалён`);
					if (current.name === 'playlist' && current.id === id) app.go({ name: 'home' });
					app.refreshPlaylists();
				}
			}
		]);
	}
</script>

<nav class="sidebar">
	<div class="brand" data-tauri-drag-region>
		<LogoMark size={26} />
		<span class="name">SoSamb0 Music</span>
	</div>

	<div class="links">
		{#each NAV as item (item.label)}
			<button class="link" class:active={item.match.includes(current.name)} onclick={() => app.go(item.route)}>
				<Icon name={item.icon} size={18} />
				<span>{item.label}</span>
			</button>
		{/each}
	</div>

	<div class="head">
		<span>Плейлисты</span>
		<button class="icon-btn add" onclick={create} aria-label="Новый плейлист" title="Новый плейлист"><Icon name="plus" size={16} /></button>
	</div>
	<div class="playlists">
		{#each app.playlists as p (p.id)}
			<button
				class="link small"
				class:active={current.name === 'playlist' && current.id === p.id}
				onclick={() => app.go({ name: 'playlist', id: p.id })}
				oncontextmenu={(e) => playlistMenu(e, p.id, p.name)}
			>
				<span class="ellipsis">{p.name}</span>
			</button>
		{:else}
			<p class="none faint">Пока нет плейлистов</p>
		{/each}
	</div>

	<button class="link bottom" class:active={current.name === 'settings'} onclick={() => app.go({ name: 'settings' })}>
		<Icon name="settings" size={18} />
		<span>Настройки</span>
	</button>
</nav>

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		width: 228px;
		min-height: 0;
		padding: 0 10px 10px;
		background: rgb(0 0 0 / 0.22);
		border-right: 1px solid var(--line);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 52px;
		padding: 0 10px;
		flex: none;
	}

	.name {
		font: 700 17px/1 'Segoe UI Variable Display', 'Segoe UI', system-ui, sans-serif;
		letter-spacing: -0.01em;
		pointer-events: none;
	}

	.links {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding-top: 4px;
	}

	.link {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 38px;
		padding: 0 12px;
		border-radius: var(--radius-sm);
		color: var(--muted);
		font-weight: 550;
		text-align: left;
		transition:
			background 0.12s,
			color 0.12s;
	}

	.link:hover {
		background: var(--surface);
		color: var(--text);
	}

	.link.active {
		background: var(--surface-hover);
		color: var(--text);
	}

	.link.active :global(svg) {
		color: var(--accent);
	}

	.link.small {
		height: 32px;
		font-weight: 450;
		font-size: 13.5px;
	}

	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin: 22px 0 4px;
		padding: 0 4px 0 12px;
		font-size: 11.5px;
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--faint);
	}

	.add {
		width: 28px;
		height: 28px;
	}

	.playlists {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 1px;
		margin: 0 -4px;
		padding: 0 4px;
	}

	.none {
		padding: 6px 12px;
		font-size: 13px;
	}

	.bottom {
		flex: none;
		margin-top: 8px;
	}
</style>
