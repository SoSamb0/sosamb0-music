# Lumen

Hi-fi desktop client for [Navidrome](https://www.navidrome.org/) (and other Subsonic-compatible servers).
Tauri 2 + Svelte 5 for the interface, a native Rust engine for the sound.

## Run

```bash
npm install
npm run tauri dev      # development
npm run tauri build    # installer in src-tauri/target/release/bundle/nsis
```

`npm run dev` alone serves the interface in a browser with a simulated transport and a built-in
demo library ("Посмотреть на демо-библиотеке" on the login screen) — handy for UI work, makes no sound.

## Sound

```
HTTP/cache → symphonia decode → f64 → [ReplayGain] → [resample] → [crossfade]
           → [DSP chain] → lock-free ring → output thread → [volume/dither] → WASAPI
```

Every stage in brackets is skipped entirely when idle. The stream is requested with `format=raw`,
so the server never transcodes. In WASAPI exclusive mode the device is switched to the rate of each
track; with DSP, ReplayGain and software volume idle, the DAC receives the samples stored in the
file. The "Звук → Вывод и тракт" panel shows the live signal path and says honestly whether
playback is bit-perfect right now.

Proof lives in the tests (`cargo test` in `src-tauri`):

- `convert::tests::*_is_bit_exact` — every 16-bit value and random 24/32-bit values survive
  the trip to f64 and out to each integer device format.
- `deck::tests::file_to_device_is_bit_perfect` — a WAV file decoded through the real pipeline
  produces device bytes identical to the bytes in the file.
- `deck::tests::seek_lands_on_the_exact_sample`, `resampler::tests::*` — sample-accurate seeking
  and delay-compensated resampling (what keeps gapless gapless).

## Layout

| Path | What |
| --- | --- |
| `src-tauri/src/audio/engine.rs` | Engine thread: decks, gapless/crossfade, timeline, signal path |
| `src-tauri/src/audio/deck.rs` | One track: source → demux → decode → stereo → resample |
| `src-tauri/src/audio/source.rs` | Progressive HTTP download to a disk cache |
| `src-tauri/src/audio/dsp.rs` | Parametric EQ, bass, crossfeed, width, limiter |
| `src-tauri/src/audio/convert.rs` | f64 → device format, TPDF dither, gain ramp |
| `src-tauri/src/audio/backend.rs` | Contract an output backend implements |
| `src-tauri/src/audio/output.rs` | WASAPI backend (shared + exclusive, event driven) |
| `src-tauri/src/media.rs` | Windows media overlay, media keys, tray |
| `src/lib/subsonic.ts` | Subsonic / OpenSubsonic API client |
| `src/lib/player.svelte.ts` | Queue, transport, scrobbling, lyrics |
| `src/lib/components`, `src/lib/views` | Interface |

## Android

The engine is platform-neutral except for `output.rs`. An Android port needs a second backend
implementing `backend.rs` (AAudio/Oboe), a touch layout for the Svelte views, and
`npm run tauri android init`.
