<script lang="ts">
	import { isMobile } from '$lib/platform';
	import { app } from '$lib/app.svelte';
	import { logFreqs, response } from '$lib/eqmath';
	import { player } from '$lib/player.svelte';
	import { EQ_PRESETS, GRAPHIC_FREQS, settings, type BandKind } from '$lib/settings.svelte';
	import { invoke } from '$lib/tauri';
	import Icon from './Icon.svelte';

	type Tab = 'eq' | 'fx' | 'out';
	let tab = $state<Tab>('eq');
	let devices = $state<{ id: string; name: string; isDefault: boolean }[]>([]);
	let fileInput = $state<HTMLInputElement>();

	const dsp = $derived(settings.data.dsp);
	const options = $derived(settings.data.options);
	const output = $derived(settings.data.output);
	const signal = $derived(player.signal);

	// The ten fixed bands get sliders; an imported parametric profile gets a table.
	const graphic = $derived(dsp.bands.length === GRAPHIC_FREQS.length && dsp.bands.every((b, i) => b.kind === 'peak' && b.freq === GRAPHIC_FREQS[i]));

	const FREQS = logFreqs(140);
	const W = 480;
	const H = 150;
	const RANGE = 15;
	const curve = $derived.by(() => {
		const db = response(dsp, FREQS);
		const y = (v: number) => H / 2 - (Math.max(-RANGE, Math.min(RANGE, v)) / RANGE) * (H / 2 - 6);
		const points = db.map((v, i) => `${((i / (FREQS.length - 1)) * W).toFixed(1)},${y(v).toFixed(1)}`);
		return { line: `M${points.join('L')}`, area: `M0,${H / 2}L${points.join('L')}L${W},${H / 2}Z`, peak: Math.max(...db) };
	});
	const gridX = [50, 100, 200, 500, 1000, 2000, 5000, 10000].map((f) => ({ f, x: (Math.log10(f / 20) / 3) * W }));

	$effect(() => {
		if (tab === 'out') invoke<typeof devices>('list_output_devices').then((list) => (devices = list ?? []));
	});

	function label(freq: number) {
		return freq >= 1000 ? `${+(freq / 1000).toFixed(1)}k` : String(freq);
	}

	function setBand(i: number, gain: number) {
		dsp.bands[i].gainDb = gain;
		settings.data.eqPreset = 'Свой';
		if (!dsp.enabled) dsp.enabled = true;
		settings.pushDsp();
	}

	function resetGraphic() {
		settings.applyPreset('Ровно');
	}

	async function importProfile(e: Event) {
		const file = (e.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		const count = settings.importAutoEq(await file.text());
		app.toast(count ? `Профиль загружен: фильтров — ${count}` : 'В файле не найдено фильтров', count ? 'info' : 'error');
		(e.currentTarget as HTMLInputElement).value = '';
	}

	const rate = (hz: number) => `${+(hz / 1000).toFixed(1)} кГц`;
	const KINDS: Record<BandKind, string> = { peak: 'Пик', lowshelf: 'НЧ-полка', highshelf: 'ВЧ-полка', lowpass: 'ФНЧ', highpass: 'ФВЧ', notch: 'Режектор' };
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (app.soundOpen = false)} />

<div class="veil" onclick={() => (app.soundOpen = false)} role="presentation"></div>
<aside class="drawer" aria-label="Звук">
	<header>
		<h2>Звук</h2>
		<button class="icon-btn" onclick={() => (app.soundOpen = false)} aria-label="Закрыть"><Icon name="x" /></button>
	</header>

	<div class="tabs" role="tablist">
		<button class="chip" class:on={tab === 'eq'} role="tab" aria-selected={tab === 'eq'} onclick={() => (tab = 'eq')}>Эквалайзер</button>
		<button class="chip" class:on={tab === 'fx'} role="tab" aria-selected={tab === 'fx'} onclick={() => (tab = 'fx')}>Эффекты</button>
		<button class="chip" class:on={tab === 'out'} role="tab" aria-selected={tab === 'out'} onclick={() => (tab = 'out')}>Вывод и тракт</button>
	</div>

	<div class="body">
		{#if tab !== 'out'}
			<div class="master">
				<div>
					<div class="strong">Обработка звука</div>
					<div class="hint">Выключено — сигнал идёт без изменений, эквалайзер и эффекты полностью обходятся.</div>
				</div>
				<button
					class="switch"
					role="switch"
					aria-checked={dsp.enabled}
					aria-label="Обработка звука"
					onclick={() => {
						dsp.enabled = !dsp.enabled;
						settings.pushDsp();
					}}
				></button>
			</div>
		{/if}

		{#if tab === 'eq'}
			<div class="block" class:off={!dsp.enabled}>
				<svg viewBox="0 0 {W} {H}" class="curve" preserveAspectRatio="none" aria-hidden="true">
					{#each gridX as g (g.f)}<line x1={g.x} x2={g.x} y1="0" y2={H} class="grid" />{/each}
					<line x1="0" x2={W} y1={H / 2} y2={H / 2} class="zero" />
					<path d={curve.area} class="area" />
					<path d={curve.line} class="trace" />
				</svg>
				<div class="axis faint"><span>20 Гц</span><span>200</span><span>2k</span><span>20k</span></div>

				<div class="row">
					<select
						class="field"
						value={settings.data.eqPreset}
						onchange={(e) => settings.applyPreset(e.currentTarget.value)}
						aria-label="Пресет эквалайзера"
					>
						{#if !(settings.data.eqPreset in EQ_PRESETS)}<option value={settings.data.eqPreset}>{settings.data.eqPreset}</option>{/if}
						{#each Object.keys(EQ_PRESETS) as name (name)}<option value={name}>{name}</option>{/each}
					</select>
					<button class="btn small" onclick={() => fileInput?.click()} title="Файл ParametricEQ.txt из проекта AutoEQ">Профиль наушников…</button>
					<input type="file" accept=".txt,text/plain" bind:this={fileInput} onchange={importProfile} hidden />
				</div>

				{#if graphic}
					<div class="bands">
						{#each dsp.bands as band, i (i)}
							<label class="band">
								<span class="val tnum">{band.gainDb > 0 ? '+' : ''}{band.gainDb.toFixed(1)}</span>
								<input
									type="range"
									class="vertical"
									min="-12"
									max="12"
									step="0.5"
									value={band.gainDb}
									oninput={(e) => setBand(i, +e.currentTarget.value)}
									ondblclick={() => setBand(i, 0)}
									aria-label="{label(band.freq)} Гц"
								/>
								<span class="freq faint">{label(band.freq)}</span>
							</label>
						{/each}
					</div>
				{:else}
					<div class="filters">
						{#each dsp.bands as band, i (i)}
							<div class="filter" class:disabled={!band.enabled}>
								<button
									class="switch mini"
									role="switch"
									aria-checked={band.enabled}
									aria-label="Фильтр {i + 1}"
									onclick={() => {
										band.enabled = !band.enabled;
										settings.pushDsp();
									}}
								></button>
								<span class="kind">{KINDS[band.kind]}</span>
								<span class="tnum">{Math.round(band.freq)} Гц</span>
								<span class="tnum">{band.gainDb > 0 ? '+' : ''}{band.gainDb.toFixed(1)} дБ</span>
								<span class="tnum faint">Q {band.q.toFixed(2)}</span>
							</div>
						{/each}
					</div>
					<button class="btn small" onclick={resetGraphic}>Вернуть 10-полосный эквалайзер</button>
				{/if}

				<label class="slider">
					<span>Предусиление</span>
					<input
						type="range"
						min="-15"
						max="6"
						step="0.5"
						value={dsp.preampDb}
						style:--fill="{((dsp.preampDb + 15) / 21) * 100}%"
						oninput={(e) => {
							dsp.preampDb = +e.currentTarget.value;
							settings.pushDsp();
						}}
					/>
					<span class="tnum num">{dsp.preampDb > 0 ? '+' : ''}{dsp.preampDb.toFixed(1)} дБ</span>
				</label>
				{#if curve.peak > 0.05}
					<p class="hint warn">Кривая поднимается выше 0 дБ на {curve.peak.toFixed(1)} дБ. Лимитер не даст звуку исказиться, но чище — снизить предусиление.</p>
				{/if}
			</div>
		{:else if tab === 'fx'}
			<div class="block" class:off={!dsp.enabled}>
				<label class="slider">
					<span>Бас</span>
					<input type="range" min="-6" max="12" step="0.5" value={dsp.bassDb} style:--fill="{((dsp.bassDb + 6) / 18) * 100}%"
						oninput={(e) => { dsp.bassDb = +e.currentTarget.value; settings.pushDsp(); }} />
					<span class="tnum num">{dsp.bassDb > 0 ? '+' : ''}{dsp.bassDb.toFixed(1)} дБ</span>
				</label>
				<label class="slider">
					<span>Crossfeed</span>
					<input type="range" min="0" max="1" step="0.05" value={dsp.crossfeed} style:--fill="{dsp.crossfeed * 100}%"
						oninput={(e) => { dsp.crossfeed = +e.currentTarget.value; settings.pushDsp(); }} />
					<span class="tnum num">{Math.round(dsp.crossfeed * 100)}%</span>
				</label>
				<p class="hint">Crossfeed подмешивает немного сигнала в противоположный канал — в наушниках звук перестаёт «сидеть в голове».</p>
				<label class="slider">
					<span>Ширина стерео</span>
					<input type="range" min="0" max="2" step="0.05" value={dsp.width} style:--fill="{(dsp.width / 2) * 100}%"
						oninput={(e) => { dsp.width = +e.currentTarget.value; settings.pushDsp(); }} />
					<span class="tnum num">{Math.round(dsp.width * 100)}%</span>
				</label>
				<div class="toggle">
					<div>
						<div class="strong">Лимитер</div>
						<div class="hint">Защищает от клиппинга, когда эквалайзер усиливает сигнал.</div>
					</div>
					<button class="switch" role="switch" aria-checked={dsp.limiter} aria-label="Лимитер"
						onclick={() => { dsp.limiter = !dsp.limiter; settings.pushDsp(); }}></button>
				</div>
			</div>

			<div class="block">
				<h3>Громкость и переходы</h3>
				<label class="line">
					<span>ReplayGain</span>
					<select class="field" value={options.replayGain}
						onchange={(e) => { options.replayGain = e.currentTarget.value as typeof options.replayGain; settings.pushOptions(); }}>
						<option value="off">Выключен</option>
						<option value="track">По треку</option>
						<option value="album">По альбому</option>
					</select>
				</label>
				{#if options.replayGain !== 'off'}
					<label class="slider">
						<span>Усиление RG</span>
						<input type="range" min="-6" max="6" step="0.5" value={options.replayGainPreampDb} style:--fill="{((options.replayGainPreampDb + 6) / 12) * 100}%"
							oninput={(e) => { options.replayGainPreampDb = +e.currentTarget.value; settings.pushOptions(); }} />
						<span class="tnum num">{options.replayGainPreampDb > 0 ? '+' : ''}{options.replayGainPreampDb.toFixed(1)} дБ</span>
					</label>
				{/if}
				<label class="slider">
					<span>Кроссфейд</span>
					<input type="range" min="0" max="12" step="1" value={options.crossfadeSecs} style:--fill="{(options.crossfadeSecs / 12) * 100}%"
						oninput={(e) => { options.crossfadeSecs = +e.currentTarget.value; settings.pushOptions(); }} />
					<span class="tnum num">{options.crossfadeSecs ? `${options.crossfadeSecs} с` : 'Выкл.'}</span>
				</label>
				<p class="hint">Треки одного альбома всегда идут без паузы и без кроссфейда — как задумано автором.</p>
			</div>
		{:else}
			{#if isMobile}
			<div class="block">
				<p class="hint">
					Звук идёт через системный микшер Android в формате 32 бит float — без обрезки разрядности.
					Если частота трека совпадает с частотой телефона (обычно 48 кГц), пересчёта нет; иначе её
					пересчитывает ресемплер плеера. Громкость — кнопками телефона: ползунок в плеере на 100% звук не трогает.
				</p>
			</div>
			{:else}
			<div class="block">
				<label class="line">
					<span>Устройство</span>
					<select class="field" value={output.deviceId ?? ''}
						onchange={(e) => { output.deviceId = e.currentTarget.value || null; settings.pushOutput(); }}>
						<option value="">Системное по умолчанию</option>
						{#each devices as d (d.id)}<option value={d.id}>{d.name}</option>{/each}
					</select>
				</label>
				<div class="toggle">
					<div>
						<div class="strong">Эксклюзивный режим (WASAPI)</div>
						<div class="hint">Плеер забирает устройство целиком и сам переключает частоту под каждый трек. Другие программы в это время не звучат.</div>
					</div>
					<button class="switch" role="switch" aria-checked={output.exclusive} aria-label="Эксклюзивный режим"
						onclick={() => { output.exclusive = !output.exclusive; settings.pushOutput(); }}></button>
				</div>
			</div>
			{/if}

			<div class="block">
				<h3>Звуковой тракт</h3>
				{#if signal}
					<div class="verdict" class:perfect={signal.bitPerfect}>
						<Icon name={signal.bitPerfect ? 'check' : 'wave'} size={18} />
						<div>
							<div class="strong">{signal.bitPerfect ? 'Bit-perfect' : signal.source.lossless ? 'Без потерь, с обработкой' : 'Источник с потерями'}</div>
							<div class="hint">
								{signal.bitPerfect
									? 'ЦАП получает ровно те отсчёты, что записаны в файле.'
									: 'Ниже видно, что именно меняет сигнал на пути к устройству.'}
							</div>
						</div>
					</div>
					<ol class="path">
						<li>
							<span class="step">Файл</span>
							<span>{signal.source.codec.toUpperCase()} · {signal.source.bits ? `${signal.source.bits} бит · ` : ''}{rate(signal.source.rate)} · {signal.source.channels} кан.</span>
							<span class="state ok">{signal.source.lossless ? 'без потерь' : 'с потерями'}</span>
						</li>
						<li class:changed={signal.replayGainDb !== 0}>
							<span class="step">ReplayGain</span>
							<span>{signal.replayGainDb !== 0 ? `${signal.replayGainDb > 0 ? '+' : ''}${signal.replayGainDb.toFixed(1)} дБ` : 'не применяется'}</span>
						</li>
						<li class:changed={signal.resampled}>
							<span class="step">Частота</span>
							<span>{signal.resampled ? `${rate(signal.source.rate)} → ${rate(signal.device.rate)}` : 'родная, без пересчёта'}</span>
						</li>
						<li class:changed={signal.dspActive}>
							<span class="step">Обработка</span>
							<span>{signal.dspActive ? 'эквалайзер / эффекты включены' : 'обходится'}</span>
						</li>
						<li class:changed={!signal.volumeUnity}>
							<span class="step">Громкость</span>
							<span>{signal.volumeUnity ? '100% — без изменения' : 'программная регулировка'}</span>
						</li>
						<li class:changed={!signal.device.exclusive}>
							<span class="step">Устройство</span>
							<span>
								{signal.device.deviceName} · {signal.device.float ? '32 бит float' : `${signal.device.bits} бит`} · {rate(signal.device.rate)}
								· {signal.device.exclusive ? 'эксклюзивно' : isMobile ? 'через микшер Android' : 'через микшер Windows'}{signal.dithered ? ' · дизеринг' : ''}
							</span>
						</li>
					</ol>
					{#if signal.device.note}<p class="hint warn">{signal.device.note}</p>{/if}
					{#if !signal.bitPerfect && signal.source.lossless}
						<p class="hint">Для bit-perfect: включите эксклюзивный режим, поставьте громкость на 100% и отключите обработку и ReplayGain.</p>
					{/if}
				{:else}
					<p class="hint">Включите трек — здесь появится весь путь сигнала от файла до ЦАП.</p>
				{/if}
			</div>
		{/if}
	</div>
</aside>

<style>
	.veil {
		position: fixed;
		inset: 0;
		z-index: 60;
		background: rgb(0 0 0 / 0.45);
		animation: fade 0.2s;
	}

	@keyframes fade {
		from {
			opacity: 0;
		}
	}

	.drawer {
		position: fixed;
		top: 0;
		right: 0;
		bottom: 0;
		z-index: 61;
		display: flex;
		flex-direction: column;
		width: min(540px, 100vw);
		background: rgb(20 20 25 / 0.94);
		backdrop-filter: blur(40px);
		border-left: 1px solid var(--line);
		box-shadow: -20px 0 60px rgb(0 0 0 / 0.5);
		animation: slide 0.28s var(--ease);
	}

	@keyframes slide {
		from {
			transform: translateX(40px);
			opacity: 0;
		}
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 18px 16px 6px 24px;
	}

	.tabs {
		display: flex;
		gap: 6px;
		padding: 8px 24px 14px;
	}

	.body {
		flex: 1;
		overflow-y: auto;
		padding: 0 24px 28px;
	}

	.master,
	.toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
	}

	.master {
		padding: 14px 16px;
		margin-bottom: 16px;
		border-radius: var(--radius);
		background: var(--surface);
	}

	.strong {
		font-weight: 600;
	}

	.hint {
		font-size: 12.5px;
		color: var(--muted);
		line-height: 1.4;
	}

	.hint.warn {
		color: #f5c26b;
	}

	.block {
		display: flex;
		flex-direction: column;
		gap: 14px;
		margin-bottom: 26px;
		transition: opacity 0.2s;
	}

	.block.off {
		opacity: 0.45;
	}

	h3 {
		font-size: 12px;
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--faint);
	}

	.curve {
		width: 100%;
		height: 150px;
		border-radius: var(--radius);
		background: rgb(0 0 0 / 0.28);
	}

	.grid {
		stroke: rgb(255 255 255 / 0.05);
	}

	.zero {
		stroke: rgb(255 255 255 / 0.16);
		stroke-dasharray: 3 4;
	}

	.area {
		fill: rgb(var(--accent-rgb) / 0.16);
	}

	.trace {
		fill: none;
		stroke: var(--accent);
		stroke-width: 2;
		vector-effect: non-scaling-stroke;
	}

	.axis {
		display: flex;
		justify-content: space-between;
		margin-top: -8px;
		font-size: 11px;
	}

	.row {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.row .btn {
		height: 40px;
		flex: none;
	}

	.bands {
		display: grid;
		grid-template-columns: repeat(10, 1fr);
		gap: 2px;
		padding: 4px 0;
	}

	.band {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
	}

	.val {
		font-size: 10.5px;
		color: var(--muted);
	}

	.freq {
		font-size: 11px;
	}

	input.vertical {
		writing-mode: vertical-lr;
		direction: rtl;
		width: 18px;
		height: 140px;
	}

	input.vertical::-webkit-slider-runnable-track {
		width: 4px;
		height: auto;
		background: var(--surface-strong);
	}

	input.vertical::-webkit-slider-thumb {
		margin-top: 0;
		margin-left: -4.5px;
		background: var(--accent);
	}

	.slider {
		display: grid;
		grid-template-columns: 124px 1fr 72px;
		align-items: center;
		gap: 12px;
	}

	.num {
		text-align: right;
		color: var(--muted);
		font-size: 13px;
	}

	.line {
		display: grid;
		grid-template-columns: 124px 1fr;
		align-items: center;
		gap: 12px;
	}

	.filters {
		display: flex;
		flex-direction: column;
		gap: 2px;
		max-height: 260px;
		overflow-y: auto;
	}

	.filter {
		display: grid;
		grid-template-columns: 34px 90px 1fr 1fr 70px;
		align-items: center;
		gap: 8px;
		height: 34px;
		padding: 0 8px;
		border-radius: 7px;
		font-size: 13px;
	}

	.filter:nth-child(odd) {
		background: var(--surface);
	}

	.filter.disabled {
		opacity: 0.45;
	}

	.switch.mini {
		transform: scale(0.75);
		transform-origin: left center;
	}

	.kind {
		color: var(--muted);
	}

	.verdict {
		display: flex;
		gap: 12px;
		align-items: center;
		padding: 14px 16px;
		border-radius: var(--radius);
		background: var(--surface);
	}

	.verdict.perfect {
		background: rgb(74 222 128 / 0.1);
		color: var(--good);
	}

	.verdict.perfect .hint {
		color: rgb(74 222 128 / 0.8);
	}

	.path {
		list-style: none;
		margin: 0;
		padding: 0;
		border-left: 2px solid var(--line);
	}

	.path li {
		position: relative;
		display: flex;
		flex-wrap: wrap;
		gap: 4px 10px;
		padding: 7px 0 7px 18px;
		font-size: 13px;
	}

	.path li::before {
		content: '';
		position: absolute;
		left: -6px;
		top: 13px;
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: #2a2a31;
		border: 2px solid var(--good);
	}

	.path li.changed::before {
		border-color: #f5c26b;
	}

	.step {
		width: 96px;
		color: var(--muted);
	}

	.state {
		color: var(--faint);
	}
</style>
