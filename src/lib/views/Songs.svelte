<script lang="ts">
	import { untrack } from 'svelte';
	import { app, plural } from '$lib/app.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SongList from '$lib/components/SongList.svelte';
	import { menu } from '$lib/menu.svelte';
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

	// ------------------------------------------------------------- sorting
	// Subsonic has no sorted "all tracks" call, so any order other than the server's loads the whole
	// list first and sorts it here.

	type SortKey = 'server' | 'created' | 'title' | 'artist' | 'album' | 'year' | 'duration' | 'plays' | 'played' | 'quality';
	/** Label, and whether the natural direction is descending (newest, most played, longest first). */
	const SORTS: Record<SortKey, [string, boolean]> = {
		server: ['Как на сервере', false],
		created: ['Дата добавления', true],
		title: ['Название', false],
		artist: ['Исполнитель', false],
		album: ['Альбом', false],
		year: ['Год', true],
		duration: ['Длительность', true],
		plays: ['Прослушивания', true],
		played: ['Недавно игравшие', true],
		quality: ['Качество', true]
	};
	const SORT_KEY = 'lumen.songsSort';

	function savedSort(): { key: SortKey; desc: boolean } {
		try {
			const v = JSON.parse(localStorage.getItem(SORT_KEY) ?? 'null');
			if (v && v.key in SORTS) return { key: v.key, desc: !!v.desc };
		} catch {
			// Storage unavailable: default order.
		}
		return { key: 'server', desc: false };
	}

	const initial = savedSort();
	let sortKey = $state<SortKey>(initial.key);
	let desc = $state(initial.desc);

	function setSort(key: SortKey, direction = SORTS[key][1]) {
		sortKey = key;
		desc = direction;
		try {
			localStorage.setItem(SORT_KEY, JSON.stringify({ key, desc: direction }));
		} catch {
			// Not remembered, still applied.
		}
	}

	function sortMenu(e: MouseEvent) {
		menu.open(
			e,
			(Object.keys(SORTS) as SortKey[]).map((key) => ({
				label: SORTS[key][0],
				icon: key === sortKey ? 'check' : undefined,
				action: () => setSort(key)
			})),
			{ title: 'Сортировка' }
		);
	}

	const text = (v: string | undefined) => (v ?? '').trim();
	const time = (v: string | undefined) => (v ? Date.parse(v) || 0 : 0);
	const collator = new Intl.Collator('ru', { sensitivity: 'base', numeric: true });

	/** Sort value of a track; null (unknown) always goes last. */
	function value(s: Song, key: SortKey): string | number | null {
		switch (key) {
			case 'created':
				return time(s.created) || null;
			case 'title':
				return text(s.title) || null;
			case 'artist':
				return text(s.artist) || null;
			case 'album':
				return text(s.album) || null;
			case 'year':
				return s.year || null;
			case 'duration':
				return s.duration || null;
			case 'plays':
				return s.playCount ?? 0;
			case 'played':
				return time(s.played) || null;
			case 'quality':
				return (s.bitDepth ?? 0) * 1e7 + (s.samplingRate ?? 0) * 10 + (s.bitRate ?? 0) / 1000 || null;
			default:
				return null;
		}
	}

	const sorted = $derived.by(() => {
		if (sortKey === 'server') return songs;
		const key = sortKey;
		const sign = desc ? -1 : 1;
		const keyed = songs.map((s, i) => ({ s, i, v: value(s, key) }));
		keyed.sort((a, b) => {
			if (a.v === null || b.v === null) return a.v === b.v ? a.i - b.i : a.v === null ? 1 : -1;
			const c = typeof a.v === 'string' ? collator.compare(a.v, b.v as string) : a.v - (b.v as number);
			// Same value (an album's tracks added together): keep album order.
			return c ? c * sign : collator.compare(text(a.s.album), text(b.s.album)) || (a.s.discNumber ?? 0) - (b.s.discNumber ?? 0) || (a.s.track ?? 0) - (b.s.track ?? 0) || a.i - b.i;
		});
		return keyed.map((k) => k.s);
	});

	// A real order needs every track.
	$effect(() => {
		if (sortKey !== 'server' && !done) untrack(() => loadAll());
	});

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
		action(sorted);
	}
</script>

<div class="page">
	<div class="head">
		<div>
			<h1>Треки</h1>
			<p class="muted">
				{#if done}{plural(songs.length, 'трек', 'трека', 'треков')}{:else if sortKey !== 'server'}Загружаем всё для сортировки: {songs.length}…{:else}Загружено {songs.length}…{/if}
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
			<div class="sort">
				<button class="btn" onclick={sortMenu} aria-label="Сортировка"><Icon name="sort" size={16} />{SORTS[sortKey][0]}</button>
				{#if sortKey !== 'server'}
					<button class="icon-btn" onclick={() => setSort(sortKey, !desc)} aria-label={desc ? 'По убыванию' : 'По возрастанию'} title={desc ? 'По убыванию' : 'По возрастанию'}>
						<Icon name={desc ? 'down' : 'up'} size={18} />
					</button>
				{/if}
			</div>
			<button class="btn primary" onclick={() => withAll((all) => player.play(all))} disabled={busy || !songs.length}><Icon name="play" size={16} />Слушать</button>
			<button class="btn" onclick={() => withAll((all) => player.play(all, 0, true))} disabled={busy || !songs.length}><Icon name="shuffle" size={16} />Перемешать</button>
			<button class="btn" onclick={() => withAll(() => list?.selectAll())} disabled={busy || !songs.length} title="Выбрать все треки, чтобы добавить их в плейлист">
				{#if busy}<span class="spinner tiny"></span>{:else}<Icon name="checked" size={16} />{/if}Выбрать все
			</button>
		</div>
	</div>

	{#if songs.length}
		{#if sortKey !== 'server' && !done}
			<div class="empty"><span class="spinner"></span></div>
		{:else}
			<SongList bind:this={list} songs={sorted} />
		{/if}
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

	.sort {
		display: flex;
		align-items: center;
		gap: 2px;
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

	/* Phones: the filter takes the whole line, buttons go below it. */
	@container main (max-width: 560px) {
		.head {
			align-items: stretch;
		}

		.actions {
			width: 100%;
		}

		.filter {
			width: 100%;
		}
	}

	.sentinel {
		display: grid;
		place-items: center;
		height: 70px;
	}
</style>
