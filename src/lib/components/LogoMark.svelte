<script lang="ts">
	// The SoSamb0 Music mark: a record with the tonearm resting on it. Same drawing as
	// branding/logo.svg, but every colour is derived from the interface accent, so the mark
	// takes on the colour of the album that is playing (or the fixed accent from settings).
	let { size = 24 }: { size?: number } = $props();
	const uid = $props.id();
</script>

<svg width={size} height={size} viewBox="64 64 896 896" aria-hidden="true">
	<defs>
		<linearGradient id="{uid}-bg" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" class="light" />
			<stop offset="1" class="deep" />
		</linearGradient>
		<clipPath id="{uid}-tile"><rect x="64" y="64" width="896" height="896" rx="200" /></clipPath>
	</defs>
	<rect x="64" y="64" width="896" height="896" rx="200" fill="url(#{uid}-bg)" />
	<g clip-path="url(#{uid}-tile)">
		<circle cx="420" cy="640" r="440" class="disc" />
		<circle cx="420" cy="640" r="74" class="label" />
	</g>
	<path d="M790 200V470L600 660" fill="none" stroke="#F4F8FF" stroke-width="46" stroke-linecap="round" stroke-linejoin="round" />
	<circle cx="790" cy="200" r="66" fill="#F4F8FF" />
</svg>

<style>
	svg {
		display: block;
		flex: none;
		pointer-events: none;
		--base: rgb(var(--accent-rgb));
	}

	/* Lightness steps match the original blue logo (#5CC8FF → #2457E8 around an accent of #5AA9FF). */
	.light {
		stop-color: oklch(from var(--base) calc(l + 0.08) c h);
		transition: stop-color 0.8s ease;
	}

	.deep {
		stop-color: oklch(from var(--base) calc(l - 0.2) c h);
		transition: stop-color 0.8s ease;
	}

	.disc {
		fill: oklch(from var(--base) 0.2 calc(c * 0.4) h);
		transition: fill 0.8s ease;
	}

	.label {
		fill: oklch(from var(--base) calc(l + 0.1) c h);
		transition: fill 0.8s ease;
	}
</style>
