<script lang="ts">
	import { app, plural } from '$lib/app.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import { SMART, type SmartKind } from '$lib/library.svelte';
	import { menu } from '$lib/menu.svelte';
	import { settings } from '$lib/settings.svelte';

	function pinMenu(e: MouseEvent, id: string, title: string) {
		const pinned = settings.isPinned(id);
		menu.open(e, [{ label: pinned ? 'Открепить с главной' : 'Закрепить на главной', icon: pinned ? 'x' : 'home', action: () => settings.togglePin(id) }], { title });
	}

	async function create() {
		const name = (await app.ask('Название плейлиста'))?.trim();
		if (!name) return;
		const ok = await app.attempt(async () => {
			await app.api!.createPlaylist(name, []);
			return true;
		});
		if (ok) app.refreshPlaylists();
	}
</script>

<div class="page">
	<div class="head">
		<h1>Плейлисты</h1>
		<button class="btn" onclick={create}><Icon name="plus" size={18} />Новый</button>
	</div>

	<h2 class="sub">Умные</h2>
	<div class="list">
		{#each Object.entries(SMART) as [id, s] (id)}
			<button class="row" onclick={() => app.go({ name: 'smart', id: id as SmartKind })} oncontextmenu={(e) => pinMenu(e, `smart:${id}`, s.title)}>
				<span class="ico"><Icon name={s.icon} size={20} /></span>
				<span class="grow">
					<span class="ellipsis">{s.title}</span>
					<span class="hint ellipsis">{s.hint}</span>
				</span>
			</button>
		{/each}
	</div>

	<h2 class="sub">Мои</h2>
	<div class="list">
		{#each app.playlists as p (p.id)}
			<button class="row" onclick={() => app.go({ name: 'playlist', id: p.id })} oncontextmenu={(e) => pinMenu(e, p.id, p.name)}>
				<span class="ico"><Icon name="playlist" size={20} /></span>
				<span class="grow">
					<span class="ellipsis">{p.name}</span>
					{#if p.songCount !== undefined}<span class="hint">{plural(p.songCount, 'трек', 'трека', 'треков')}</span>{/if}
				</span>
			</button>
		{:else}
			<p class="muted">Пока нет плейлистов</p>
		{/each}
	</div>
</div>

<style>
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 36px;
		padding: 0 14px;
		border-radius: 999px;
		background: var(--surface-strong);
	}

	.sub {
		margin: 22px 0 6px;
		font-size: 13px;
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.list {
		display: flex;
		flex-direction: column;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 14px;
		min-height: 60px;
		padding: 6px;
		border-bottom: 1px solid var(--line);
		text-align: left;
	}

	.grow {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		font-size: 16px;
	}

	.hint {
		font-size: 12.5px;
		color: var(--muted);
	}

	.ico {
		flex: none;
		display: grid;
		place-items: center;
		width: 42px;
		height: 42px;
		border-radius: 10px;
		background: var(--card);
		color: var(--accent);
	}
</style>
