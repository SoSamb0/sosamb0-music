<script lang="ts">
	import { app } from '$lib/app.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import { applyHotkeys, GLOBAL_HOTKEYS } from '$lib/native';
	import { player } from '$lib/player.svelte';
	import { settings } from '$lib/settings.svelte';
	import { invoke } from '$lib/tauri';

	const data = $derived(settings.data);
	let cache = $state<{ bytes: number; files: number } | null>(null);

	async function refreshCache() {
		cache = (await invoke<{ bytes: number; files: number }>('cache_info')) ?? null;
	}
	refreshCache();

	async function clearCache() {
		await invoke('cache_clear');
		await refreshCache();
		app.toast('Кэш очищен');
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
				<div class="strong">Эквалайзер, эффекты, устройство вывода</div>
				<div class="hint">Эксклюзивный режим WASAPI, ReplayGain, кроссфейд и схема звукового тракта.</div>
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

	<section>
		<h2>Кэш</h2>
		<div class="card">
			<div class="row">
				<div class="grow">
					<div class="strong">Прослушанные треки</div>
					<div class="hint">
						{cache ? `Занято ${size(cache.bytes)}, файлов: ${cache.files}.` : 'Подсчёт…'} Повторное прослушивание идёт с диска, без сети.
					</div>
				</div>
				<button class="btn" onclick={clearCache}><Icon name="trash" size={16} />Очистить</button>
			</div>
			<div class="row">
				<div class="grow"><div class="strong">Предел размера</div></div>
				<select
					class="field limit"
					value={String(data.options.cacheLimitMb)}
					onchange={(e) => {
						data.options.cacheLimitMb = Number(e.currentTarget.value);
						settings.pushOptions();
					}}
					aria-label="Предел размера кэша"
				>
					<option value="512">512 МБ</option>
					<option value="2048">2 ГБ</option>
					<option value="8192">8 ГБ</option>
					<option value="32768">32 ГБ</option>
				</select>
			</div>
		</div>
	</section>

	<section>
		<h2>Клавиши</h2>
		<div class="card keys">
			{#each SHORTCUTS as [keys, what] (keys)}
				<div class="key"><kbd>{keys}</kbd><span class="muted">{what}</span></div>
			{/each}
		</div>
	</section>
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
		background: var(--surface);
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
</style>
