<script lang="ts">
	import { menu, type MenuItem } from '$lib/menu.svelte';
	import Cover from '../Cover.svelte';
	import Icon from '../Icon.svelte';

	/** Index of the item whose submenu replaces the list, -1 for the main list. */
	let sub = $state(-1);
	let pull = $state(0);
	let startY: number | null = null;
	let list = $state<HTMLElement>();

	// Every menu opens on its main list.
	$effect(() => {
		if (menu.visible) {
			menu.items;
			sub = -1;
			pull = 0;
		}
	});

	const items = $derived(sub >= 0 ? (menu.items[sub]?.children ?? []) : menu.items);

	function run(item: MenuItem, index: number) {
		if (item.children) {
			sub = index;
			list?.scrollTo({ top: 0 });
			return;
		}
		menu.close();
		item.action?.();
	}

	// Drag the sheet down by its top part to close it.
	function down(e: PointerEvent) {
		startY = e.clientY;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}
	function move(e: PointerEvent) {
		if (startY !== null) pull = Math.max(0, e.clientY - startY);
	}
	function up() {
		if (pull > 80) menu.close();
		startY = null;
		pull = 0;
	}
</script>

{#if menu.visible}
	<div class="veil" onclick={() => menu.close()} oncontextmenu={(e) => (e.preventDefault(), menu.close())} role="presentation"></div>
	<div class="sheet" class:pulling={pull > 0} style:translate="0 {pull}px" role="menu">
		<div class="grab" onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} role="presentation">
			<span class="handle"></span>
			{#if menu.header && sub < 0}
				<div class="head">
					{#if menu.header.cover !== undefined}
						<span class="art" class:round={menu.header.round}><Cover id={menu.header.cover} size={120} /></span>
					{/if}
					<span class="text">
						<span class="title ellipsis">{menu.header.title}</span>
						{#if menu.header.subtitle}<span class="sub ellipsis">{menu.header.subtitle}</span>{/if}
					</span>
				</div>
			{/if}
		</div>

		<div class="list" bind:this={list}>
			{#if sub >= 0}
				<button class="item back" onclick={() => (sub = -1)}>
					<span class="ico"><Icon name="left" size={20} /></span>
					<span class="ellipsis">{menu.items[sub]?.label}</span>
				</button>
				<div class="sep"></div>
			{/if}
			{#each items as item, i (i)}
				{#if item.separator}
					<div class="sep"></div>
				{:else}
					<button class="item" class:danger={item.danger} role="menuitem" onclick={() => run(item, i)}>
						<span class="ico">{#if item.icon}<Icon name={item.icon} size={20} />{/if}</span>
						<span class="ellipsis">{item.label}</span>
						{#if item.children}<span class="arrow"><Icon name="right" size={16} /></span>{/if}
					</button>
				{/if}
			{/each}
		</div>
	</div>
{/if}

<style>
	.veil {
		position: fixed;
		inset: 0;
		z-index: 900;
		background: rgb(0 0 0 / 0.5);
		animation: fade 0.2s;
	}

	@keyframes fade {
		from {
			opacity: 0;
		}
	}

	.sheet {
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 901;
		display: flex;
		flex-direction: column;
		max-height: 82dvh;
		padding-bottom: var(--sab, env(safe-area-inset-bottom, 0px));
		border-radius: 22px 22px 0 0;
		background: rgb(26 26 32 / 0.97);
		backdrop-filter: blur(24px);
		border-top: 1px solid var(--line);
		box-shadow: 0 -16px 50px rgb(0 0 0 / 0.5);
		animation: up 0.24s var(--ease);
		transition: translate 0.2s var(--ease);
	}

	.sheet.pulling {
		transition: none;
	}

	@keyframes up {
		from {
			transform: translateY(100%);
		}
	}

	.grab {
		flex: none;
		padding: 10px 20px 4px;
		touch-action: none;
	}

	.handle {
		display: block;
		width: 38px;
		height: 4px;
		margin: 0 auto 10px;
		border-radius: 2px;
		background: rgb(255 255 255 / 0.25);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 4px 0 12px;
		border-bottom: 1px solid var(--line);
	}

	.art {
		flex: none;
		width: 52px;
		border-radius: 8px;
		overflow: hidden;
	}

	.art.round {
		border-radius: 50%;
	}

	.text {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.title {
		font-size: 16px;
		font-weight: 600;
	}

	.sub {
		font-size: 13.5px;
		color: var(--muted);
	}

	.list {
		overflow-y: auto;
		overscroll-behavior: contain;
		padding: 6px 10px 12px;
	}

	.item {
		display: flex;
		align-items: center;
		gap: 16px;
		width: 100%;
		min-height: 52px;
		padding: 0 12px;
		border-radius: 12px;
		font-size: 16px;
		text-align: left;
	}

	.item:active {
		background: var(--surface-hover);
	}

	.item.danger {
		color: var(--danger);
	}

	.item.back {
		color: var(--muted);
	}

	.ico {
		display: grid;
		place-items: center;
		width: 22px;
		color: var(--muted);
	}

	.arrow {
		margin-left: auto;
		color: var(--faint);
	}

	.sep {
		height: 1px;
		margin: 6px 12px;
		background: var(--line);
	}
</style>
