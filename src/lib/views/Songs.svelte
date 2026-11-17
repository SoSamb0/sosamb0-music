<script lang="ts">
	import { untrack } from 'svelte';
	import { app, plural } from '$lib/app.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SongList from '$lib/components/SongList.svelte';
	import { player } from '$lib/player.svelte';
	import type { Song } from '$lib/types';

	// Every track of the library, a page at a time, with a filter.
	const PAGE = 200;

	let query = $state('');
	let applied = $state('');
	let songs = $state<Song[]>([]);
	let loading = $state(false);
	let done = $state(false);
	let sentinel = $state<HTMLElement>();
	let list = $state<ReturnType<typeof SongList>>();
	let run = 0;
	let timer: ReturnType<typeof setTimeout>;

	async function more(): Promise<boolean> {
		if (loading || done) return false;
		loading = true;
		const mine = run;
		const page = await app.attempt(() => app.api!.songs(applied, PAGE, songs.length));
		if (mine !== run) return false;
		loading = false;
		if (!page || page.length < PAGE) done = true;
		if (page) {
			const seen = new Set(songs.map((s) => s.id));
			songs.push(...page.filter((s) => !seen.has(s.id)));
		}
		return !!page;
	}

	// Start over whenever the filter changes.
	$effect(() => {
		applied;
		untrack(() => {
			run++;
			songs = [];
			done = false;
			loading = false;
			more();
		});
	});

	$effect(() => {
		if (!sentinel) return;
		const observer = new IntersectionObserver((entries) => entries[0].isIntersecting && more(), { rootMargin: '800px' });
		observer.observe(sentinel);
		return () => observer.disconnect();
	});

	function onInput() {
		clearTimeout(timer);
		timer = setTimeout(() => (applied = query.trim()), 300);
	}

	/** Load the remaining pages so that actions apply to every matching track, not just the visible ones. */
	async function loadAll() {
		let guard = 0;
		while (!done && guard++ < 500) {
			if (loading) await new Promise((r) => setTimeout(r, 50));
			else if (!(await more())) break;
		}
	}

	let busy = $state(false);
	async function withAll(action: (all: Song[]) => void) {
		busy = true;
		await loadAll();
		busy = false;
		action(songs);
	}
</script>

<div class="page">
	<div class="head">
		<div>
			<h1>Треки</h1>
			<p class="muted">
				{#if done}{plural(songs.length, 'трек', 'трека', 'треков')}{:else}Загружено {songs.length}…{/if}
			</p>
		</div>
		<div class="actions">
			<input
				class="field filter"
				bind:value={query}
				oninput={onInput}
				onkeydown={(e) => e.stopPropagation()}
				placeholder="Название или исполнитель"
				aria-label="Фильтр треков"
			/>
			<button class="btn primary" onclick={() => withAll((all) => player.play(all))} disabled={busy || !songs.length}><Icon name="play" size={16} />Слушать</button>
			<button class="btn" onclick={() => withAll((all) => player.play(all, 0, true))} disabled={busy || !songs.length}><Icon name="shuffle" size={16} />Перемешать</button>
			<button class="btn" onclick={() => withAll(() => list?.selectAll())} disabled={busy || !songs.length} title="Выбрать все треки, чтобы добавить их в плейлист">
				{#if busy}<span class="spinner tiny"></span>{:else}<Icon name="checked" size={16} />{/if}Выбрать все
			</button>
		</div>
	</div>

	{#if songs.length}
		<SongList bind:this={list} {songs} />
	{:else if !loading && done}
		<div class="empty"><p>{applied ? 'Ничего не найдено' : 'В библиотеке пока нет треков'}</p></div>
	{/if}
	<div bind:this={sentinel} class="sentinel">{#if loading}<span class="spinner"></span>{/if}</div>
</div>

<style>
	.head {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		flex-wrap: wrap;
		gap: 16px;
		margin-bottom: 20px;
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}

	.filter {
		width: 240px;
		height: 38px;
		border-radius: 999px;
	}

	.spinner.tiny {
		width: 14px;
		height: 14px;
		border-width: 2px;
	}

	.sentinel {
		display: grid;
		place-items: center;
		height: 70px;
	}
</style>
