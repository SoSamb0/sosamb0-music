<script lang="ts">
	import { app, formatDuration, plural } from '$lib/app.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import { player, type HistoryEntry } from '$lib/player.svelte';

	const RANGES = [
		{ days: 7, label: '7 дней' },
		{ days: 30, label: '30 дней' },
		{ days: 365, label: 'Год' },
		{ days: 0, label: 'Всё время' }
	];

	let days = $state(30);
	const all = player.history();

	const entries = $derived(days ? all.filter((e) => e.at >= Date.now() - days * 86_400_000) : all);

	function top(key: (e: HistoryEntry) => string, sub: (e: HistoryEntry) => string) {
		const map = new Map<string, { name: string; sub: string; plays: number; seconds: number }>();
		for (const e of entries) {
			const k = key(e);
			if (!k) continue;
			const row = map.get(k) ?? { name: k, sub: sub(e), plays: 0, seconds: 0 };
			row.plays++;
			row.seconds += e.seconds;
			map.set(k, row);
		}
		return [...map.values()].sort((a, b) => b.plays - a.plays).slice(0, 10);
	}

	const tracks = $derived(top((e) => `${e.title}\u0000${e.artist}`, (e) => e.artist).map((r) => ({ ...r, name: r.name.split('\u0000')[0] })));
	const artists = $derived(top((e) => e.artist, () => ''));
	const albums = $derived(top((e) => (e.album ? `${e.album}\u0000${e.artist}` : ''), (e) => e.artist).map((r) => ({ ...r, name: r.name.split('\u0000')[0] })));
	const total = $derived(entries.reduce((sum, e) => sum + e.seconds, 0));
</script>

<div class="page">
	<h1>Статистика</h1>
	<div class="ranges">
		{#each RANGES as r (r.days)}
			<button class="chip" class:on={days === r.days} onclick={() => (days = r.days)}>{r.label}</button>
		{/each}
	</div>

	{#if entries.length === 0}
		<div class="empty">
			<Icon name="stats" size={30} />
			<p>Пока нечего показать. Статистика собирается на этом компьютере по мере прослушивания.</p>
			<button class="btn" onclick={() => app.go({ name: 'home' })}>На главную</button>
		</div>
	{:else}
		<div class="totals">
			<div class="total"><b>{plural(entries.length, 'прослушивание', 'прослушивания', 'прослушиваний')}</b><span>за период</span></div>
			<div class="total"><b>{formatDuration(total) || 'меньше минуты'}</b><span>музыки</span></div>
			<div class="total"><b>{plural(new Set(entries.map((e) => e.artist)).size, 'исполнитель', 'исполнителя', 'исполнителей')}</b><span>разных</span></div>
		</div>

		<div class="columns">
			{#each [{ title: 'Треки', rows: tracks }, { title: 'Исполнители', rows: artists }, { title: 'Альбомы', rows: albums }] as col (col.title)}
				<section>
					<h2>{col.title}</h2>
					{#each col.rows as row, i (row.name + row.sub)}
						<div class="rank">
							<span class="n tnum">{i + 1}</span>
							<div class="text">
								<div class="ellipsis name">{row.name}</div>
								{#if row.sub}<div class="ellipsis sub">{row.sub}</div>{/if}
							</div>
							<span class="plays tnum">{row.plays}</span>
							<div class="bar" style:width="{(row.plays / col.rows[0].plays) * 100}%"></div>
						</div>
					{/each}
				</section>
			{/each}
		</div>
	{/if}
</div>

<style>
	.ranges {
		display: flex;
		gap: 6px;
		margin: 16px 0 22px;
	}

	.totals {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 12px;
		margin-bottom: 28px;
	}

	.total {
		display: flex;
		flex-direction: column;
		padding: 16px 18px;
		border-radius: var(--radius);
		background: var(--surface);
	}

	.total b {
		font: 650 20px/1.3 'Segoe UI Variable Display', 'Segoe UI', system-ui, sans-serif;
	}

	.total span {
		font-size: 13px;
		color: var(--muted);
	}

	.columns {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: 28px;
	}

	section h2 {
		margin-bottom: 10px;
	}

	.rank {
		position: relative;
		display: grid;
		grid-template-columns: 24px minmax(0, 1fr) auto;
		align-items: center;
		gap: 10px;
		height: 48px;
		padding: 0 10px;
		border-radius: var(--radius-sm);
		overflow: hidden;
		isolation: isolate;
	}

	.bar {
		position: absolute;
		inset: 0 auto 0 0;
		background: var(--accent-soft);
		z-index: -1;
		border-radius: var(--radius-sm);
	}

	.n {
		color: var(--faint);
		font-size: 13px;
		text-align: center;
	}

	.text {
		min-width: 0;
	}

	.name {
		font-weight: 550;
	}

	.sub {
		font-size: 12.5px;
		color: var(--muted);
	}

	.plays {
		font-size: 13px;
		color: var(--muted);
	}
</style>
