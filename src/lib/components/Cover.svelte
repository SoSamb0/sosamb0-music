<script lang="ts">
	import { app } from '$lib/app.svelte';
	import Icon from './Icon.svelte';

	let {
		id,
		size = 300,
		round = false,
		icon = 'album',
		eager = false
	}: { id?: string; size?: number; round?: boolean; icon?: string; eager?: boolean } = $props();

	const src = $derived(app.api?.coverUrl(id, size) ?? '');
	let failed = $state(false);
	let loaded = $state(false);

	$effect(() => {
		// A new image gets a fresh chance to load.
		src;
		failed = false;
		loaded = false;
	});
</script>

<div class="cover" class:round>
	{#if src && !failed}
		<img
			{src}
			alt=""
			loading={eager ? 'eager' : 'lazy'}
			decoding="async"
			draggable="false"
			class:loaded
			onload={() => (loaded = true)}
			onerror={() => (failed = true)}
		/>
	{:else}
		<div class="blank"><Icon name={icon} size={28} /></div>
	{/if}
</div>

<style>
	.cover {
		position: relative;
		width: 100%;
		aspect-ratio: 1;
		border-radius: inherit;
		overflow: hidden;
		background: var(--surface);
	}

	.cover.round {
		border-radius: 50%;
	}

	img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
		opacity: 0;
		transition: opacity 0.3s;
	}

	img.loaded {
		opacity: 1;
	}

	.blank {
		display: grid;
		place-items: center;
		width: 100%;
		height: 100%;
		color: var(--faint);
		background: linear-gradient(135deg, rgb(255 255 255 / 0.07), rgb(255 255 255 / 0.02));
	}
</style>
