<script lang="ts">
	import { app } from '$lib/app.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import { applyHotkeys, GLOBAL_HOTKEYS } from '$lib/native';
	import { player } from '$lib/player.svelte';
	import { ACCENTS, DEFAULT_DISCORD_APP_ID, settings, type Look } from '$lib/settings.svelte';
	import { isMobile } from '$lib/platform';
	import { inTauri, invoke } from '$lib/tauri';

	const data = $derived(settings.data);
	const look = $derived(settings.data.look);
	let cache = $state<{ bytes: number; files: number } | null>(null);
	const queueCached = $derived(player.queue.filter((q) => player.cached.has(player.cacheKey(q.song))).length);

	async function refreshCache() {
		cache = (await invoke<{ bytes: number; files: number }>('cache_info')) ?? null;
	}
	refreshCache();
	// Keep the numbers live while the queue downloads.
	$effect(() => {
		const id = setInterval(refreshCache, 3000);
		return () => clearInterval(id);
	});

	async function clearCache() {
		await invoke('cache_clear');
		await refreshCache();
		await player.syncPrefetch();
		app.toast('Кэш очищен');
	}

	function setDiscord<K extends keyof typeof settings.data.discord>(key: K, value: (typeof settings.data.discord)[K]) {
		settings.data.discord[key] = value;
		settings.pushDiscord();
		player.notifyDiscord();
	}

	async function openPortal() {
		const url = 'https://discord.com/developers/applications';
		if (inTauri) {
			const { openUrl } = await import('@tauri-apps/plugin-opener');
			await openUrl(url).catch(() => app.toast('Не удалось открыть браузер', 'error'));
		} else {
			window.open(url, '_blank');
		}
	}

	function setLook<K extends keyof Look>(key: K, value: Look[K]) {
		settings.data.look[key] = value;
		settings.save();
	}

	function setCache<K extends keyof typeof settings.data.cache>(key: K, value: (typeof settings.data.cache)[K]) {
		settings.data.cache[key] = value;
		settings.save();
		player.syncPrefetch();
	}

	function size(bytes: number): string {
		if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(2)} ГБ`;
		return `${Math.round(bytes / 1024 ** 2)} МБ`;
	}

	function toggle(key: 'lrclib' | 'scrobble' | 'closeToTray') {
		data[key] = !data[key];
		settings.save();
	}

	async function toggleHotkeys() {
		data.hotkeys = !data.hotkeys;
		settings.save();
		const ok = await applyHotkeys();
		if (!ok) app.toast('Часть сочетаний занята другой программой и не будет работать', 'error');
	}

	function logout() {
		player.reset();
		app.logout();
	}

	const SHORTCUTS = [
		['Пробел', 'Играть / пауза'],
		['Ctrl + → / ←', 'Следующий / предыдущий трек'],
		['→ / ←', 'Перемотка на 5 секунд'],
		['↑ / ↓', 'Громкость'],
		['M', 'Выключить звук'],
		['Ctrl + K', 'Поиск'],
		['L', 'Текст песни'],
		['Q', 'Очередь'],
		['E', 'Звук и эквалайзер'],
		['F', 'Развернуть плеер'],
		['Alt + ← / →', 'Назад / вперёд']
	];
</script>

<div class="page narrow">
	<h1>Настройки</h1>

	<section>
		<h2>Сервер</h2>
		<div class="card row">
			<div class="grow">
				{#if data.demo}
					<div class="strong">Демо-библиотека</div>
					<div class="hint">Звук не воспроизводится — это режим для знакомства с интерфейсом.</div>
				{:else if data.server}
					<div class="strong ellipsis">{data.server.url}</div>
					<div class="hint">Пользователь: {data.server.username}</div>
				{/if}
			</div>
			<button class="btn" onclick={logout}><Icon name="logout" size={16} />{data.demo ? 'Выйти из демо' : 'Сменить сервер'}</button>
		</div>
	</section>

	<section>
		<h2>Звук</h2>
		<div class="card row">
			<div class="grow">
				<div class="strong">{isMobile ? 'Эквалайзер и эффекты' : 'Эквалайзер, эффекты, устройство вывода'}</div>
				<div class="hint">{isMobile ? 'ReplayGain, кроссфейд и схема звукового тракта.' : 'Эксклюзивный режим WASAPI, ReplayGain, кроссфейд и схема звукового тракта.'}</div>
			</div>
			<button class="btn" onclick={() => (app.soundOpen = true)}><Icon name="sliders" size={16} />Открыть</button>
		</div>
	</section>

	<section>
		<h2>Тексты и скробблинг</h2>
		<div class="card">
			<div class="row">
				<div class="grow">
					<div class="strong">Искать тексты на LRCLIB</div>
					<div class="hint">Если на сервере нет синхронного текста, название трека, исполнитель и альбом отправляются на lrclib.net.</div>
				</div>
				<button class="switch" role="switch" aria-checked={data.lrclib} aria-label="Искать тексты на LRCLIB" onclick={() => toggle('lrclib')}></button>
			</div>
			<div class="row">
				<div class="grow">
					<div class="strong">Скробблинг</div>
					<div class="hint">Прослушивания передаются в Navidrome, а он — в Last.fm и ListenBrainz, если они подключены в его настройках.</div>
				</div>
				<button class="switch" role="switch" aria-checked={data.scrobble} aria-label="Скробблинг" onclick={() => toggle('scrobble')}></button>
			</div>
		</div>
	</section>

	{#if !isMobile}
	<section>
		<h2>Discord</h2>
		<div class="card">
			<div class="row">
				<div class="grow">
					<div class="strong">Показывать в Discord, что играет</div>
					<div class="hint">Статус «Слушает» с названием трека, исполнителем, обложкой и временем в вашем профиле. Нужен запущенный Discord на этом компьютере.</div>
				</div>
				<button class="switch" role="switch" aria-checked={data.discord.enabled} aria-label="Статус в Discord" onclick={() => setDiscord('enabled', !data.discord.enabled)} disabled={data.demo}></button>
			</div>
			{#if data.discord.enabled}
				<div class="row sub">
					<div class="grow discord-state">
						{#if settings.discordStatus?.connected}
							<span class="dot on"></span><span>Подключено к Discord</span>
							{#if settings.discordStatus.cover}<img src={settings.discordStatus.cover} alt="" class="thumb" />{/if}
							{#if settings.discordStatus.message}<span class="muted"> · {settings.discordStatus.message}</span>{/if}
						{:else}
							<span class="dot"></span><span class="muted">{settings.discordStatus?.message ?? 'Подключение…'} · повтор каждые 15 секунд</span>
						{/if}
					</div>
				</div>
				<div class="row sub">
					<div class="grow">
						<div class="strong">Обложка в статусе</div>
						<div class="hint">
							Discord загружает картинку сам, а ваш сервер ему недоступен. Поэтому SoSamb0 Music ищет тот же альбом в открытых
							каталогах — MusicBrainz, Apple Music, Deezer — и показывает их картинку. Туда уходят только название
							альбома и исполнителя; сервер остаётся закрытым.
						</div>
					</div>
					<button class="switch" role="switch" aria-checked={data.discord.covers} aria-label="Искать обложку" onclick={() => setDiscord('covers', !data.discord.covers)}></button>
				</div>
				<div class="row sub">
					<div class="grow"><div class="strong">Скрывать статус на паузе</div></div>
					<button class="switch" role="switch" aria-checked={data.discord.hidePaused} aria-label="Скрывать на паузе" onclick={() => setDiscord('hidePaused', !data.discord.hidePaused)}></button>
				</div>
				<details class="row sub advanced" open={!!data.discord.appId && data.discord.appId !== DEFAULT_DISCORD_APP_ID}>
					<summary class="muted">Дополнительно: своё приложение Discord</summary>
					<div class="adv">
						<div class="hint">
							По умолчанию статус идёт от приложения «SoSamb0 Music». Чтобы в статусе было другое имя, создайте своё приложение на
							<button class="link" onclick={openPortal}>портале разработчиков Discord</button> и вставьте его Application ID.
							Пустое поле — приложение SoSamb0 Music.
						</div>
						<input
							class="field appid"
							value={data.discord.appId === DEFAULT_DISCORD_APP_ID ? '' : data.discord.appId}
							placeholder="SoSamb0 Music (по умолчанию)"
							inputmode="numeric"
							spellcheck="false"
							onkeydown={(e) => e.stopPropagation()}
							onchange={(e) => setDiscord('appId', e.currentTarget.value.replace(/\D/g, ''))}
							aria-label="Свой ID приложения Discord"
						/>
					</div>
				</details>
			{/if}
		</div>
	</section>
	{/if}

	{#if !isMobile}
	<section>
		<h2>Система</h2>
		<div class="card">
			<div class="row">
				<div class="grow">
					<div class="strong">Сворачивать в трей при закрытии</div>
					<div class="hint">Музыка продолжит играть; выйти можно через значок в трее.</div>
				</div>
				<button class="switch" role="switch" aria-checked={data.closeToTray} aria-label="Сворачивать в трей" onclick={() => toggle('closeToTray')}></button>
			</div>
			<div class="row">
				<div class="grow">
					<div class="strong">Глобальные горячие клавиши</div>
					<div class="hint">
						Работают, даже когда окно не в фокусе: {GLOBAL_HOTKEYS.map((h) => `${h.keys} — ${h.label.toLowerCase()}`).join('; ')}.
						Мультимедийные клавиши клавиатуры работают всегда.
					</div>
				</div>
				<button class="switch" role="switch" aria-checked={data.hotkeys} aria-label="Глобальные горячие клавиши" onclick={toggleHotkeys}></button>
			</div>
		</div>
	</section>
	{/if}

	<section>
		<h2>Внешний вид</h2>
		<div class="card">
			<div class="row">
				<div class="grow">
					<div class="strong">Фон полноэкранного плеера</div>
					<div class="hint">«Живой фон» медленно движется — немного нагружает видеокарту.</div>
				</div>
			</div>
			<div class="row options">
				{#each [['blur', 'Размытая обложка'], ['motion', 'Живой фон'], ['dim', 'Затемнение (как раньше)'], ['solid', 'Сплошной тёмный']] as [value, label] (value)}
					<button class="chip" class:on={look.npBackground === value} onclick={() => setLook('npBackground', value as Look['npBackground'])}>{label}</button>
				{/each}
			</div>
			{#if look.npBackground !== 'solid'}
				<label class="row sub slider">
					<span class="strong">Яркость фона</span>
					<input
						type="range"
						min="0.2"
						max="1"
						step="0.02"
						value={look.npStrength}
						style:--fill="{((look.npStrength - 0.2) / 0.8) * 100}%"
						oninput={(e) => setLook('npStrength', +e.currentTarget.value)}
					/>
					<span class="tnum muted">{Math.round(look.npStrength * 100)}%</span>
				</label>
				{#if look.npBackground !== 'dim'}
					<label class="row sub slider">
						<span class="strong">Размытие</span>
						<input
							type="range"
							min="0"
							max="80"
							step="2"
							value={look.npBlur}
							style:--fill="{(look.npBlur / 80) * 100}%"
							oninput={(e) => setLook('npBlur', +e.currentTarget.value)}
						/>
						<span class="tnum muted">{look.npBlur === 0 ? 'Нет' : `${Math.round((look.npBlur / 80) * 100)}%`}</span>
					</label>
				{/if}
			{/if}

			<div class="row">
				<div class="grow"><div class="strong">Обложка</div></div>
				<div class="seg">
					{#each [['square', 'Квадрат'], ['round', 'Круг'], ['vinyl', 'Винил']] as [value, label] (value)}
						<button class="chip" class:on={look.coverShape === value} onclick={() => setLook('coverShape', value as Look['coverShape'])}>{label}</button>
					{/each}
				</div>
			</div>
			{#if look.coverShape === 'vinyl'}
				<div class="row sub">
					<div class="grow"><div class="strong">Скорость вращения</div></div>
					<div class="seg">
						{#each [[30, 'Медленно'], [12, 'Обычно'], [1.8, 'Как пластинка']] as [value, label] (value)}
							<button class="chip" class:on={look.vinylSeconds === value} onclick={() => setLook('vinylSeconds', value as number)}>{label}</button>
						{/each}
					</div>
				</div>
				<div class="row sub">
					<div class="grow">
						<div class="strong">Крутить и маленькую обложку</div>
						<div class="hint">В панели плеера внизу и в мини-плеере.</div>
					</div>
					<button class="switch" role="switch" aria-checked={look.vinylEverywhere} aria-label="Крутить маленькую обложку" onclick={() => setLook('vinylEverywhere', !look.vinylEverywhere)}></button>
				</div>
			{/if}

			<div class="row">
				<div class="grow">
					<div class="strong">Акцентный цвет</div>
					<div class="hint">«Из обложки» подстраивает интерфейс под каждый альбом.</div>
				</div>
				<div class="swatches">
					<button class="swatch auto" class:on={look.accent === 'cover'} onclick={() => setLook('accent', 'cover')} title="Из обложки" aria-label="Цвет из обложки"></button>
					{#each ACCENTS as color (color)}
						<button class="swatch" class:on={look.accent === color} style:background={color} onclick={() => setLook('accent', color)} aria-label="Цвет {color}"></button>
					{/each}
				</div>
			</div>

			<div class="row">
				<div class="grow">
					<div class="strong">Размытая обложка на фоне приложения</div>
					<div class="hint">Цветное свечение за всем интерфейсом.</div>
				</div>
				<button class="switch" role="switch" aria-checked={look.appBackdrop} aria-label="Обложка на фоне" onclick={() => setLook('appBackdrop', !look.appBackdrop)}></button>
			</div>
			{#if look.appBackdrop}
				<label class="row sub slider">
					<span class="strong">Яркость свечения</span>
					<input
						type="range"
						min="0.1"
						max="0.8"
						step="0.02"
						value={look.backdropStrength}
						style:--fill="{((look.backdropStrength - 0.1) / 0.7) * 100}%"
						oninput={(e) => setLook('backdropStrength', +e.currentTarget.value)}
					/>
					<span class="tnum muted">{Math.round(look.backdropStrength * 100)}%</span>
				</label>
			{/if}

			<div class="row">
				<div class="grow"><div class="strong">Текст песни</div></div>
				<div class="seg">
					{#each [['s', 'Мелкий'], ['m', 'Средний'], ['l', 'Крупный']] as [value, label] (value)}
						<button class="chip" class:on={look.lyricsSize === value} onclick={() => setLook('lyricsSize', value as Look['lyricsSize'])}>{label}</button>
					{/each}
				</div>
			</div>
			<div class="row sub">
				<div class="grow"><div class="strong">Выравнивание</div></div>
				<div class="seg">
					<button class="chip" class:on={look.lyricsAlign === 'left'} onclick={() => setLook('lyricsAlign', 'left')}>По левому краю</button>
					<button class="chip" class:on={look.lyricsAlign === 'center'} onclick={() => setLook('lyricsAlign', 'center')}>По центру</button>
				</div>
			</div>
			<div class="row sub">
				<div class="grow"><div class="strong">Показывать текст в полноэкранном плеере</div></div>
				<button class="switch" role="switch" aria-checked={look.npLyrics} aria-label="Текст в полноэкранном плеере" onclick={() => setLook('npLyrics', !look.npLyrics)}></button>
			</div>

			<div class="row">
				<div class="grow">
					<div class="strong">Визуализатор</div>
					<div class="hint">В полноэкранном плеере. Анализ звука работает, только пока визуализатор на экране, и не влияет на bit-perfect.</div>
				</div>
			</div>
			<div class="row options">
				{#each [['off', 'Выключен'], ['bars', 'Полосы'], ['wave', 'Волна'], ['ring', 'Кольцо'], ['vu', 'Стрелочные индикаторы'], ['glow', 'Свечение']] as [value, label] (value)}
					<button class="chip" class:on={look.vizStyle === value} onclick={() => setLook('vizStyle', value as Look['vizStyle'])}>{label}</button>
				{/each}
			</div>
			{#if look.vizStyle !== 'off'}
				<div class="row sub">
					<div class="grow"><div class="strong">Где показывать</div></div>
					<div class="seg">
						{#each [['below', 'Под текстом'], ['replace', 'Вместо текста'], ['background', 'На фоне']] as [value, label] (value)}
							<button class="chip" class:on={look.vizPlace === value} onclick={() => setLook('vizPlace', value as Look['vizPlace'])}>{label}</button>
						{/each}
					</div>
				</div>
			{/if}

			{#if !isMobile}
			<div class="row">
				<div class="grow"><div class="strong">Масштаб интерфейса</div></div>
				<div class="seg">
					{#each [0.9, 1, 1.1, 1.25] as value (value)}
						<button class="chip" class:on={look.scale === value} onclick={() => setLook('scale', value)}>{Math.round(value * 100)}%</button>
					{/each}
				</div>
			</div>
			{/if}
			<div class="row">
				<div class="grow">
					<div class="strong">Анимации интерфейса</div>
					<div class="hint">Выключите на слабом компьютере. Вращение пластинки при этом остаётся.</div>
				</div>
				<button class="switch" role="switch" aria-checked={look.animations} aria-label="Анимации" onclick={() => setLook('animations', !look.animations)}></button>
			</div>
		</div>
	</section>

	<section>
		<h2>Кэш</h2>
		<div class="card">
			<div class="row">
				<div class="grow">
					<div class="strong">Скачивать очередь заранее</div>
					<div class="hint">
						Как только вы включаете альбом или плейлист, треки очереди скачиваются в фоне. Переключение и перемотка — мгновенные,
						скачанное играет даже без сети.
					</div>
				</div>
				<button class="switch" role="switch" aria-checked={data.cache.prefetch} aria-label="Скачивать очередь заранее" onclick={() => setCache('prefetch', !data.cache.prefetch)}></button>
			</div>
			{#if data.cache.prefetch}
				<div class="row sub">
					<div class="grow"><div class="strong">Сколько треков наперёд</div></div>
					<div class="seg">
						{#each [[3, '3'], [10, '10'], [25, '25'], [0, 'Вся очередь']] as [value, label] (value)}
							<button class="chip" class:on={data.cache.ahead === value} onclick={() => setCache('ahead', value as number)}>{label}</button>
						{/each}
					</div>
				</div>
				{#if player.queue.length}
					<div class="row sub">
						<div class="grow hint">
							Сейчас в очереди: в кэше {queueCached} из {player.queue.length}{player.downloading.size ? ', качается…' : ''}.
							{#if player.cacheLimited}Очередь больше предела кэша — часть треков не будет скачана заранее.{/if}
						</div>
					</div>
				{/if}
			{/if}
			<div class="row">
				<div class="grow">
					<div class="strong">Предел размера</div>
					<div class="hint">Когда кэш заполнен, удаляются давно не игравшие треки.</div>
				</div>
				<select
					class="field limit"
					value={String(data.options.cacheLimitMb)}
					onchange={(e) => {
						data.options.cacheLimitMb = Number(e.currentTarget.value);
						settings.pushOptions();
						player.syncPrefetch();
					}}
					aria-label="Предел размера кэша"
				>
					{#each [[1024, '1 ГБ'], [2048, '2 ГБ'], [4096, '4 ГБ'], [8192, '8 ГБ'], [16384, '16 ГБ'], [32768, '32 ГБ'], [65536, '64 ГБ'], [262144, '256 ГБ']] as [value, label] (value)}
						<option value={String(value)}>{label}</option>
					{/each}
				</select>
			</div>
			<div class="row">
				<div class="grow">
					<div class="strong">Занято</div>
					<div class="hint">
						{cache ? `${size(cache.bytes)} из ${size(data.options.cacheLimitMb * 1024 * 1024)}, треков: ${cache.files}` : 'Подсчёт…'}
					</div>
					{#if cache}
						<div class="meter"><div style:width="{Math.min(100, (cache.bytes / (data.options.cacheLimitMb * 1024 * 1024)) * 100)}%"></div></div>
					{/if}
				</div>
				<button class="btn" onclick={() => invoke('cache_open_dir')} disabled={!inTauri}><Icon name="folder" size={16} />Открыть папку</button>
				<button class="btn" onclick={clearCache}><Icon name="trash" size={16} />Очистить</button>
			</div>
		</div>
	</section>

	{#if !isMobile}
	<section>
		<h2>Клавиши</h2>
		<div class="card keys">
			{#each SHORTCUTS as [keys, what] (keys)}
				<div class="key"><kbd>{keys}</kbd><span class="muted">{what}</span></div>
			{/each}
		</div>
	</section>
	{/if}
</div>

<style>
	.narrow {
		max-width: 820px;
	}

	section {
		margin-top: 28px;
	}

	section h2 {
		margin-bottom: 10px;
		font-size: 12px;
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--faint);
	}

	.card {
		display: flex;
		flex-direction: column;
		border-radius: var(--radius);
		background: var(--card);
		border: 1px solid var(--line);
	}

	.row {
		display: flex;
		align-items: center;
		gap: 18px;
		padding: 16px 18px;
	}

	.card > .row + .row {
		border-top: 1px solid var(--line);
	}

	.grow {
		flex: 1;
		min-width: 0;
	}

	.strong {
		font-weight: 600;
	}

	.hint {
		margin-top: 2px;
		font-size: 12.5px;
		color: var(--muted);
		line-height: 1.45;
	}

	.limit {
		width: 140px;
	}

	.row.sub {
		padding-top: 4px;
		padding-left: 34px;
	}

	.card > .row.sub {
		border-top: 0;
	}

	.row.options {
		flex-wrap: wrap;
		gap: 6px;
		padding-top: 0;
		border-top: 0 !important;
	}

	.seg {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 6px;
	}

	.slider {
		display: grid;
		grid-template-columns: 1fr 200px 44px;
	}

	.swatches {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 7px;
		max-width: 330px;
	}

	.swatch {
		width: 26px;
		height: 26px;
		border-radius: 50%;
		box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.18);
		transition: transform 0.15s var(--ease);
	}

	.swatch:hover {
		transform: scale(1.12);
	}

	.swatch.on {
		box-shadow:
			0 0 0 2px var(--bg),
			0 0 0 4px #fff;
	}

	.swatch.auto {
		background: conic-gradient(#fb7185, #fbbf24, #4ade80, #2dd4bf, #5aa9ff, #8b7cf6, #e879f9, #fb7185);
	}

	.advanced {
		display: block;
	}

	.advanced summary {
		cursor: pointer;
		font-size: 13px;
	}

	.adv {
		display: flex;
		align-items: center;
		gap: 16px;
		margin-top: 10px;
	}

	.appid {
		flex: none;
		width: 210px;
		font-variant-numeric: tabular-nums;
	}

	.link {
		color: var(--accent);
		text-decoration: underline;
	}

	.discord-state {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--faint);
		flex: none;
	}

	.dot.on {
		background: #5865f2;
		box-shadow: 0 0 8px #5865f2;
	}

	.thumb {
		width: 28px;
		height: 28px;
		border-radius: 5px;
		object-fit: cover;
	}

	.meter {
		height: 4px;
		margin-top: 8px;
		border-radius: 999px;
		background: var(--surface-strong);
		overflow: hidden;
	}

	.meter div {
		height: 100%;
		background: var(--accent);
		border-radius: inherit;
	}

	.keys {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: 10px 24px;
		padding: 16px 18px;
	}

	.key {
		display: flex;
		align-items: center;
		gap: 12px;
		font-size: 13px;
	}

	kbd {
		min-width: 96px;
		padding: 4px 8px;
		border-radius: 6px;
		background: var(--surface-hover);
		font: 600 12px/1.3 inherit;
		text-align: center;
		white-space: nowrap;
	}

	/* Narrow screens (phones): choices go under their label instead of squeezing it. */
	@container main (max-width: 560px) {
		.row {
			padding: 14px 16px;
		}

		.row:has(> .seg),
		.row:has(> .swatches) {
			flex-direction: column;
			align-items: stretch;
			gap: 10px;
		}

		.seg,
		.swatches {
			justify-content: flex-start;
			max-width: none;
		}

		.row.sub {
			padding-left: 30px;
		}

		.slider {
			grid-template-columns: 1fr 44px;
			row-gap: 8px;
		}

		.slider > .strong {
			grid-column: 1 / -1;
		}
	}
</style>
