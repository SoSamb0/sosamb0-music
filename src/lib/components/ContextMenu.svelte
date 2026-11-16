<script lang="ts">
	import { menu, type MenuItem } from '$lib/menu.svelte';
	import Icon from './Icon.svelte';

	let el = $state<HTMLElement>();
	let pos = $state({ x: 0, y: 0 });
	let sub = $state<MenuItem | null>(null);
	let subLeft = $state(false);

	// Keep the menu inside the window.
	$effect(() => {
		if (!menu.visible || !el) return;
		sub = null;
		const { offsetWidth: w, offsetHeight: h } = el;
		pos = {
			x: Math.max(8, Math.min(menu.x, window.innerWidth - w - 8)),
			y: Math.max(8, Math.min(menu.y, window.innerHeight - h - 8))
		};
		subLeft = pos.x + w + 230 > window.innerWidth;
	});

	function run(item: MenuItem) {
		if (item.children) {
			sub = sub === item ? null : item;
			return;
		}
		menu.close();
		item.action?.();
	}
</script>

<svelte:window
	onkeydown={(e) => e.key === 'Escape' && menu.close()}
	onblur={() => menu.close()}
	onresize={() => menu.close()}
/>

{#if menu.visible}
	<div class="veil" onpointerdown={() => menu.close()} oncontextmenu={(e) => (e.preventDefault(), menu.close())} role="presentation"></div>
	<div class="menu" bind:this={el} style:left="{pos.x}px" style:top="{pos.y}px" role="menu">
		{#each menu.items as item, i (i)}
			{#if item.separator}
				<div class="sep"></div>
			{:else}
				<div class="wrap" onpointerenter={() => (sub = item.children ? item : null)} role="presentation">
					<button class="item" class:danger={item.danger} class:open={sub === item} role="menuitem" onclick={() => run(item)}>
						<span class="ico">{#if item.icon}<Icon name={item.icon} size={16} />{/if}</span>
						<span class="ellipsis">{item.label}</span>
						{#if item.children}<span class="arrow"><Icon name="right" size={14} /></span>{/if}
					</button>
					{#if sub === item && item.children}
						<div class="menu submenu" class:left={subLeft} role="menu">
							{#each item.children as child, j (j)}
								<button class="item" role="menuitem" onclick={() => run(child)}>
									<span class="ico">{#if child.icon}<Icon name={child.icon} size={16} />{/if}</span>
									<span class="ellipsis">{child.label}</span>
								</button>
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		{/each}
	</div>
{/if}

<style>
	.veil {
		position: fixed;
		inset: 0;
		z-index: 900;
	}

	.menu {
		position: fixed;
		z-index: 901;
		min-width: 220px;
		max-width: 300px;
		padding: 6px;
		border-radius: var(--radius);
		background: rgb(28 28 34 / 0.92);
		backdrop-filter: blur(24px) saturate(1.4);
		border: 1px solid var(--line);
		box-shadow: 0 18px 50px rgb(0 0 0 / 0.55);
		animation: pop 0.14s var(--ease);
		transform-origin: top left;
	}

	@keyframes pop {
		from {
			opacity: 0;
			transform: scale(0.96);
		}
	}

	.wrap {
		position: relative;
	}

	.submenu {
		position: absolute;
		left: calc(100% + 2px);
		top: -6px;
		max-height: 320px;
		overflow-y: auto;
	}

	.submenu.left {
		left: auto;
		right: calc(100% + 2px);
	}

	.item {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		height: 34px;
		padding: 0 10px;
		border-radius: 7px;
		text-align: left;
	}

	.item:hover,
	.item.open {
		background: var(--surface-hover);
	}

	.item.danger {
		color: var(--danger);
	}

	.ico {
		display: grid;
		place-items: center;
		width: 18px;
		color: var(--muted);
	}

	.arrow {
		margin-left: auto;
		color: var(--faint);
	}

	.sep {
		height: 1px;
		margin: 5px 8px;
		background: var(--line);
	}
</style>
