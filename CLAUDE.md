# SoSamb0 Music

Hi-fi клиент для Navidrome (Subsonic API). Версия для Windows и Android (Android в работе: состояние и
план — [docs/ANDROID.md](docs/ANDROID.md)).

## Пользователь

- Пишет по-русски — отвечать по-русски. Весь интерфейс приложения на русском.
- Хочет «лучший звук» (lossless, bit-perfect), красивый интерфейс и чтобы программа не грузила компьютер.
- Дизайн: не любит «ИИшный» вид (неоновое свечение, градиенты ради градиентов, пластинки-ноты-эквалайзеры как
  клише). Фирменный цвет — голубой `#5AA9FF`, логотип — пластинка с тонармом (`branding/logo.svg`).
- Свой сервер Navidrome в домашней сети, открывать его в интернет не хочет.

## Имена (не менять без нужды)

- Название: **SoSamb0 Music**. Папка проекта и Rust-крейт исторически называются `Lumen` / `lumen`.
- Идентификатор приложения `app.lumen.player` и ключи `localStorage` `lumen.*` **менять нельзя**: от них
  зависят настройки, вход на сервер, очередь и кэш у уже установленных копий.
- Discord Application ID приложения встроен: `DEFAULT_DISCORD_APP_ID` в `src/lib/settings.svelte.ts`.

## Команды

```bash
npm run dev                         # интерфейс в браузере: http://127.0.0.1:1420, есть демо-библиотека
npm run check                       # проверка типов Svelte/TS
cd src-tauri && cargo test --lib    # тесты звукового движка (26 шт., 2 живых — с --ignored)
npm run tauri build                 # релиз: src-tauri/target/release/SoSamb0 Music.exe и NSIS-установщик
powershell -ExecutionPolicy Bypass -File scripts/android-build.ps1 [-Install]   # APK для телефона
```

- Телефонная раскладка включается по user agent (`src/lib/platform.ts`, `isMobile`): в браузерной панели —
  `resize_window` с пресетом `mobile`. Мобильные компоненты — `src/lib/components/mobile`.
- Превью интерфейса в браузерной панели: `preview_start` с именем `lumen-ui` (`.claude/launch.json`).
  На экране входа «Посмотреть на демо-библиотеке» — интерфейс работает без сервера, а `SimEngine`
  в `src/lib/tauri.ts` имитирует движок (позиция, сигнальный тракт, спектр для визуализатора).
- Перед `tauri build` закрыть запущенное приложение — иначе exe занят:
  `Get-Process 'SoSamb0 Music' | Stop-Process`.
- Git: имя и почта автора заданы локально для этого репозитория. Коммитить после каждой законченной задачи.

## Устройство

```
HTTP/кэш → symphonia → f64 → [ReplayGain] → [ресемплер] → [кроссфейд] → [DSP]
        → кольцевой буфер (rtrb) → поток вывода → [громкость/дизеринг] → WASAPI
```

Каждая ступень в скобках полностью пропускается, когда не нужна: при эксклюзивном режиме, громкости 100%,
выключенных DSP и ReplayGain на ЦАП идут ровно байты из файла (доказано тестами `convert` и `deck`).

| Файл | Что |
| --- | --- |
| `src-tauri/src/audio/engine.rs` | Поток движка: деки, gapless/кроссфейд, таймлайн позиции, сигнальный тракт, визуализатор |
| `src-tauri/src/audio/deck.rs` | Один трек: источник → демукс → декод → стерео → ресемплинг |
| `src-tauri/src/audio/source.rs` | Прогрессивная загрузка в дисковый кэш; одна закачка на трек (плеер и предзагрузка делят её) |
| `src-tauri/src/audio/prefetch.rs` | Фоновая загрузка очереди в кэш |
| `src-tauri/src/audio/backend.rs` | **Контракт вывода звука** (`OutCmd`, `OutShared`) — то, что реализует платформа |
| `src-tauri/src/audio/output.rs` | Вывод для Windows (WASAPI shared/exclusive) |
| `src-tauri/src/audio/output_android.rs` | Вывод для Android (AAudio, float, родная частота устройства) |
| `src-tauri/src/audio/dsp.rs`, `convert.rs`, `resampler.rs`, `analyzer.rs` | EQ/эффекты, конвертация в формат устройства, ресемплер, спектр для визуализатора |
| `src-tauri/src/media.rs` | Системные медиакнопки Windows (souvlaki) и трей |
| `src-tauri/src/android_media.rs` + `gen/android/…/MediaBridge.kt`, `PlaybackService.kt` | Android: медиасеанс, уведомление, фоновая служба, аудиофокус |
| `src-tauri/src/discord.rs` | Статус Discord через локальный pipe + поиск публичной обложки |
| `src/lib/subsonic.ts` | Клиент Subsonic/OpenSubsonic |
| `src/lib/player.svelte.ts` | Очередь, транспорт, скробблинг, тексты, предзагрузка, Discord |
| `src/lib/settings.svelte.ts` | Все настройки (persist в localStorage) |
| `src/lib/components`, `src/lib/views` | Интерфейс (Svelte 5, runes) |

## Важные решения и грабли (уже наступали)

- Поток запрашивается с `format=raw` — сервер никогда не перекодирует.
- Ошибки Subsonic приходят как HTTP 200 с JSON/XML — `source.rs` это проверяет.
- Все треки библиотеки — `search3` с запросом `""` (`songs()` в `subsonic.ts`).
- Перестановка в плейлисте: в Subsonic нет «move», делается одним `updatePlaylist` — удалить все индексы и
  добавить id в новом порядке. На реальном сервере пользователя не проверялось.
- Windows/Tauri: HTML5 drag-and-drop работает только с `"dragDropEnabled": false` у окна.
- Tauri: если в JS слушать закрытие окна (`onCloseRequested`), окно закрывает JS через `destroy()` — нужно
  разрешение `core:window:allow-destroy`. Новые вызовы API окна = новые разрешения в `capabilities/default.json`.
- Svelte 5: эффект, который пишет состояние, которое сам читает, зацикливается — используйте `untrack`.
  Объекты из `$state`-массивов — прокси, сравнивайте по индексу/id, а не по ссылке. Большие списки —
  `$state.raw`. `{@const}` только прямо внутри блока (`{#each}`, `{#if}`).
- Анализатор визуализатора только читает звук (`try_lock`, никогда не блокирует поток вывода) и работает,
  лишь пока визуализатор на экране.
- Dev-сервер Vite иногда отдаёт старый CSS компонента после правки скриптом — `touch` файла помогает.
- Десктопный код (трей, souvlaki, горячие клавиши, single-instance) — под `#[cfg(desktop)]`, зависимости в
  `cfg(not(any(android, ios)))`. Разрешения: `capabilities/default.json` — десктоп, `mobile.json` — телефон.
- Android: сборку (`scripts/android-build.ps1`) запускать из Bash-инструмента (`powershell.exe -File …`), не из
  PowerShell-инструмента — его песочница пишет в изолированную копию, и на диске остаётся старый APK.
- Android: крейт `ndk` 0.9 считает ошибкой успешный `AAudioStream_write` — используем `ndk-sys` напрямую.
- Отладка на телефоне: `adb forward tcp:9222 localabstract:webview_devtools_remote_<pid>` → DevTools-протокол
  WebView (debug-APK). `eprintln!` из Rust виден в `adb logcat` с тегом `RustStdoutStderr`.
- Android WebView не поддерживает `scrollbar-width` — полосы прокрутки прячет `::-webkit-scrollbar`.
- Телефонная раскладка экранов — через `@container main (max-width: 560px)`, общие плотные поверхности —
  `--card`, `--card-hover`, `--sheet` в `app.css`. Раскладка главной — `settings.data.home` (`HomeEditor.svelte`).
- Длинные heredoc в Bash-инструменте могут обрезаться — длинные скрипты писать файлом через Write.
- Проверка в браузерной панели: синтетические `.click()` срабатывают, но иногда реальная мышь ведёт себя
  иначе (так нашли баг подменю) — важное проверять настоящими кликами.

## Что не проверено на настоящем сервере

Перестановка треков в плейлисте, раздел «Треки» (зависит от `search3 ""`), умные плейлисты (нужны
`playCount`/`played`/`created` от сервера), визуализатор на настоящем звуке.
