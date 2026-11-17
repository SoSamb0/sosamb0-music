<script lang="ts">
	// The SoSamb0 Music mark: a record with the tonearm resting on it.
	// Same drawing as branding/logo.svg, cropped to the tile.
	let { size = 24 }: { size?: number } = $props();
	const uid = $props.id();
</script>

<svg width={size} height={size} viewBox="64 64 896 896" aria-hidden="true">
	<defs>
		<linearGradient id="{uid}-bg" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0" stop-color="#5CC8FF" />
			<stop offset="1" stop-color="#2457E8" />
		</linearGradient>
		<clipPath id="{uid}-tile"><rect x="64" y="64" width="896" height="896" rx="200" /></clipPath>
	</defs>
	<rect x="64" y="64" width="896" height="896" rx="200" fill="url(#{uid}-bg)" />
	<g clip-path="url(#{uid}-tile)">
		<circle cx="420" cy="640" r="440" fill="#0B1630" />
		<circle cx="420" cy="640" r="74" fill="#7DD3FF" />
	</g>
	<path d="M790 200V470L600 660" fill="none" stroke="#F4F8FF" stroke-width="46" stroke-linecap="round" stroke-linejoin="round" />
	<circle cx="790" cy="200" r="66" fill="#F4F8FF" />
</svg>

<style>
	svg {
		display: block;
		flex: none;
		pointer-events: none;
	}
</style>
