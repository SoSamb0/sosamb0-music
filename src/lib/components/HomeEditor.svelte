<script lang="ts">
	import { app } from '$lib/app.svelte';
	import { SMART } from '$lib/library.svelte';
	import { HOME_SECTIONS, settings, type HomeSection } from '$lib/settings.svelte';
	import { gapToIndex, touchReorder } from '$lib/touchdrag';
	import Icon from './Icon.svelte';

	// Lets the user choose and order the sections of the home screen and pin playlists to it.
	let { onclose }: { onclose: () => void } = $props();

	const home = $derived(settings.data.home);
	let gap = $state<number | null>(null);

	function toggle(s: HomeSection) {
		const hidden = home.hidden;
		settings.data.home.hidden = hidden.includes(s) ? hidden.filter((h) => h !== s) : [...hidden, s];
		settings.save();
	}

	function move(from: number, to: number) {
		const order = [...home.order];
		const [item] = order.splice(from, 1);
		order.splice(to, 0, item);
		settings.data.home.order = order;
		settings.save();
	}

	const pinnable = $derived([
		...Object.entries(SMART).map(([id, s]) => ({ id: `smart:${id}`, title: s.title, icon: s.icon })),
		...app.playlists.map((p) => ({ id: p.id, title: p.name, icon: 'playlist' }))
	]);
</script>

<div class="veil" onclick={onclose} role="presentation"></div>
<div class="editor" role="dialog" aria-label="Настройка главной">
	<header>
		<h2>Главная</h2>
		<button class="btn primary" onclick={onclose}>Готово</button>
	</header>

	<div class="body">
		<p class="hint">Включайте разделы и меняйте порядок, перетаскивая за ≡.</p>
		<div class="list">
			{#each home.order as s, i (s)}
				<div class="item" data-i={i} class:off={home.hidden.includes(s)} class:gap-before={gap === i} class:gap-after={gap === home.order.length && i === home.order.length - 1}>
					<span class="name">{HOME_SECTIONS[s]}</span>
					<button class="switch" role="switch" aria-checked={!home.hidden.includes(s)} aria-label={HOME_SECTIONS[s]} onclick={() => toggle(s)}></button>
					<button class="grip" onpointerdown={(e) => touchReorder(e, i, (g) => (gap = g), (from, g) => move(from, gapToIndex(from, g)))} aria-label="Перетащить {HOME_SECTIONS[s]}">
						<Icon name="grip" size={20} />
					</button>
				</div>
			{/each}
		</div>

		<h3>Закрепить на главной</h3>
		<p class="hint">Закреплённое показывается плитками быстрого доступа.</p>
		<div class="list">
			{#each pinnable as p (p.id)}
				<button class="item pick" class:on={settings.isPinned(p.id)} onclick={() => settings.togglePin(p.id)}>
					<span class="pi"><Icon name={p.icon} size={18} /></span>
					<span class="name ellipsis">{p.title}</span>
					<span class="mark"><Icon name={settings.isPinned(p.id) ? 'checked' : 'checkbox'} size={20} /></span>
				</button>
			{/each}
		</div>
	</div>
</div>

<style>
	.veil {
		position: fixed;
		inset: 0;
		z-index: 70;
		background: rgb(0 0 0 / 0.55);
		animation: fade 0.2s;
	}

	@keyframes fade {
		from {
			opacity: 0;
		}
	}

	.editor {
		position: fixed;
		left: 50%;
		bottom: 0;
		z-index: 71;
		display: flex;
		flex-direction: column;
		width: min(560px, 100%);
		max-height: 88dvh;
		translate: -50% 0;
		padding-bottom: var(--sab, 0px);
		border-radius: 22px 22px 0 0;
		background: var(--sheet);
		border: 1px solid var(--line);
		border-bottom: 0;
		box-shadow: 0 -16px 50px rgb(0 0 0 / 0.5);
		animation: up 0.25s var(--ease);
	}

	@keyframes up {
		from {
			transform: translateY(40%);
			opacity: 0;
		}
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 16px 16px 8px 20px;
	}

	.body {
		overflow-y: auto;
		overscroll-behavior: contain;
		padding: 0 12px 20px;
	}

	h3 {
		margin: 22px 8px 0;
		font-size: 15px;
	}

	.hint {
		margin: 4px 8px 10px;
		font-size: 13px;
		color: var(--muted);
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.item {
		position: relative;
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 52px;
		padding: 0 4px 0 14px;
		border-radius: 12px;
		background: var(--card);
		text-align: left;
	}

	.item.off .name {
		color: var(--faint);
	}

	.item.gap-before::before,
	.item.gap-after::after {
		content: '';
		position: absolute;
		left: 10px;
		right: 10px;
		height: 2px;
		border-radius: 2px;
		background: var(--accent);
	}

	.item.gap-before::before {
		top: -3px;
	}

	.item.gap-after::after {
		bottom: -3px;
	}

	.name {
		flex: 1;
		min-width: 0;
		font-weight: 550;
	}

	.pick {
		padding: 0 14px 0 8px;
	}

	.pi {
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border-radius: 9px;
		background: var(--accent-soft);
		color: var(--accent);
	}

	.mark {
		color: var(--faint);
	}

	.pick.on .mark {
		color: var(--accent);
	}
</style>
