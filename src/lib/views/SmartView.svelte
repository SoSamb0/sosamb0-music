<script lang="ts">
	import { app, formatDuration, plural } from '$lib/app.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SongList from '$lib/components/SongList.svelte';
	import { library, SMART, smartList, type SmartKind } from '$lib/library.svelte';
	import { player } from '$lib/player.svelte';

	let { kind }: { kind: SmartKind } = $props();

	const info = $derived(SMART[kind]);
	let limit = $state(100);

	library.load();

	// Plays recorded on this computer, used only if the server does not count plays.
	const localPlays = new Map<string, number>();
	for (const h of player.history()) localPlays.set(h.id, (localPlays.get(h.id) ?? 0) + 1);

	const songs = $derived(library.songs ? smartList(kind, library.songs, limit, localPlays) : []);
	const total = $derived(songs.reduce((t, s) => t + (s.duration ?? 0), 0));

	async function save() {
		const date = new Date().toLocaleDateString('ru-RU');
		const name = (await app.ask('Название плейлиста', `${info.title} · ${date}`))?.trim();
		if (!name) return;
		const ok = await app.attempt(async () => {
			await app.api!.createPlaylist(name, songs.map((s) => s.id));
			return true;
		});
		if (ok) {
			app.toast(`Плейлист «${name}» создан: ${songs.length}`);
			app.refreshPlaylists();
		}
	}

	function detail(kind: SmartKind): string {
		if (kind === 'top') return 'По числу прослушиваний на сервере';
		if (kind === 'forgotten') return 'Слушали много раз, но не включали больше месяца';
		return 'По дате добавления в библиотеку';
	}
</script>

<div class="page">
	<header class="hero">
		<div class="tile"><Icon name={info.icon} size={64} /></div>
		<div class="info">
			<div class="kind">Умный плейлист</div>
			<h1>{info.title}</h1>
			<div class="muted meta">
				{#if songs.length}{plural(songs.length, 'трек', 'трека', 'треков')} · {formatDuration(total)} · {/if}{detail(kind)}
			</div>
			<div class="actions">
				<button class="btn primary" onclick={() => player.play(songs)} disabled={!songs.length}><Icon name="play" size={16} />Слушать</button>
				<button class="btn" onclick={() => player.play(songs, 0, true)} disabled={!songs.length}><Icon name="shuffle" size={16} />Перемешать</button>
				<button class="btn" onclick={save} disabled={!songs.length} title="Сохранить этот список как обычный плейлист на сервере"><Icon name="plus" size={16} />Сохранить как плейлист</button>
				<button class="icon-btn" onclick={() => library.load(true)} disabled={library.loading} aria-label="Обновить" title="Обновить"><Icon name="refresh" size={18} /></button>
			</div>
		</div>
	</header>

	<div class="limits">
		<span class="muted">Показывать:</span>
		{#each [50, 100, 250, 500] as n (n)}
			<button class="chip" class:on={limit === n} onclick={() => (limit = n)}>{n}</button>
		{/each}
	</div>

	{#if library.loading && !library.songs}
		<div class="empty"><span class="spinner"></span><p>Читаю библиотеку… {library.count ? `треков: ${library.count}` : ''}</p></div>
	{:else if !songs.length}
		<div class="empty">
			<Icon name={info.icon} size={30} />
			<p>
				{#if kind === 'forgotten'}Пока нечего вспомнить: сюда попадают треки, которые вы слушали несколько раз, но давно не включали.
				{:else if kind === 'top'}Сервер пока не насчитал прослушиваний.
				{:else}В библиотеке нет треков.{/if}
			</p>
		</div>
	{:else}
		<SongList {songs} />
	{/if}
</div>

<style>
	.hero {
		display: flex;
		align-items: flex-end;
		gap: 28px;
		margin-bottom: 18px;
	}

	.tile {
		display: grid;
		place-items: center;
		flex: none;
		width: clamp(150px, 20vw, 230px);
		aspect-ratio: 1;
		border-radius: 14px;
		color: #fff;
		background: linear-gradient(135deg, rgb(var(--accent-rgb)), rgb(var(--accent-rgb) / 0.35));
		box-shadow: 0 20px 50px rgb(0 0 0 / 0.5);
	}

	.info {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-width: 0;
	}

	.kind {
		font-size: 11.5px;
		font-weight: 700;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--accent);
	}

	h1 {
		font-size: clamp(26px, 3.6vw, 46px);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		margin-top: 10px;
	}

	.limits {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 14px;
		font-size: 13px;
	}
</style>
