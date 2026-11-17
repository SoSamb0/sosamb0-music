<script lang="ts">
	import type { Snippet } from 'svelte';
	import Cover from './Cover.svelte';

	let {
		cover,
		kind,
		title,
		round = false,
		icon = 'album',
		meta,
		actions
	}: { cover?: string; kind: string; title: string; round?: boolean; icon?: string; meta?: Snippet; actions?: Snippet } = $props();
</script>

<header class="hero">
	<div class="art" class:round><Cover id={cover} size={500} {round} {icon} eager /></div>
	<div class="info">
		<div class="kind">{kind}</div>
		<h1>{title}</h1>
		{#if meta}<div class="meta muted">{@render meta()}</div>{/if}
		{#if actions}<div class="actions">{@render actions()}</div>{/if}
	</div>
</header>

<style>
	.hero {
		display: flex;
		align-items: flex-end;
		gap: 28px;
		margin-bottom: 26px;
	}

	.art {
		flex: none;
		width: clamp(150px, 20vw, 230px);
		border-radius: 14px;
		box-shadow: 0 20px 50px rgb(0 0 0 / 0.5);
	}

	.art.round {
		border-radius: 50%;
	}

	.info {
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-width: 0;
		padding-bottom: 4px;
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
		overflow-wrap: anywhere;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		user-select: text;
	}

	.meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px 8px;
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		margin-top: 10px;
	}

	/* Phones: a large centred cover, then the title and the buttons in one row. */
	@container main (max-width: 560px) {
		.hero {
			flex-direction: column;
			align-items: center;
			gap: 18px;
			margin-bottom: 18px;
			text-align: center;
		}

		.art {
			width: min(64%, 260px);
		}

		.info {
			align-items: center;
			width: 100%;
		}

		h1 {
			font-size: 26px;
		}

		.meta,
		.actions {
			justify-content: center;
		}

		.actions {
			flex-wrap: nowrap;
			margin-top: 6px;
		}
	}
</style>
