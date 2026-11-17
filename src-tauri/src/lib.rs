mod audio;
mod discord;
mod media;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};

use audio::prefetch::Prefetcher;
use audio::{Command, DeviceInfo, DspConfig, Engine, EngineHandle, Options, OutputConfig, TrackReq};

struct AppState {
    engine: EngineHandle,
    discord: discord::Discord,
    prefetch: Prefetcher,
    cache_dir: PathBuf,
}

#[tauri::command]
fn discord_configure(state: State<AppState>, config: discord::DiscordConfig) {
    state.discord.configure(config);
}

#[tauri::command]
fn discord_update(state: State<AppState>, now: Option<discord::NowPlaying>) {
    state.discord.update(now);
}

#[tauri::command]
fn player_play(state: State<AppState>, track: TrackReq) {
    state.engine.send(Command::Play(track));
}

#[tauri::command]
fn player_set_next(state: State<AppState>, track: Option<TrackReq>) {
    state.engine.send(Command::SetNext(track));
}

#[tauri::command]
fn player_pause(state: State<AppState>) {
    state.engine.send(Command::Pause);
}

#[tauri::command]
fn player_resume(state: State<AppState>) {
    state.engine.send(Command::Resume);
}

#[tauri::command]
fn player_stop(state: State<AppState>) {
    state.engine.send(Command::Stop);
}

#[tauri::command]
fn player_seek(state: State<AppState>, position: f64) {
    state.engine.send(Command::Seek(position));
}

#[tauri::command]
fn player_set_volume(state: State<AppState>, volume: f64) {
    state.engine.send(Command::SetVolume(volume));
}

#[tauri::command]
fn player_set_dsp(state: State<AppState>, config: DspConfig) {
    state.engine.send(Command::SetDsp(config));
}

#[tauri::command]
fn player_set_visualizer(state: State<AppState>, on: bool) {
    state.engine.send(Command::SetVisualizer(on));
}

#[tauri::command]
fn player_set_output(state: State<AppState>, config: OutputConfig) {
    state.engine.send(Command::SetOutput(config));
}

#[tauri::command]
fn player_set_options(state: State<AppState>, options: Options) {
    state.engine.send(Command::SetOptions(options));
}

#[tauri::command]
async fn list_output_devices(state: State<'_, AppState>) -> Result<Vec<DeviceInfo>, String> {
    Ok(state.engine.list_devices())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheInfo {
    bytes: u64,
    files: u64,
    path: String,
}

#[tauri::command]
async fn cache_info(state: State<'_, AppState>) -> Result<CacheInfo, String> {
    let (bytes, files) = audio::source::cache_usage(&state.cache_dir);
    Ok(CacheInfo { bytes, files, path: state.cache_dir.to_string_lossy().into_owned() })
}

/// Download these tracks into the cache in the background, in order. Replaces the previous list.
#[tauri::command]
fn cache_prefetch(state: State<AppState>, tracks: Vec<TrackReq>, limit_mb: u64) {
    state.prefetch.set(tracks, limit_mb * 1024 * 1024);
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheKey {
    cache_key: String,
    #[serde(default)]
    suffix: String,
}

#[tauri::command]
async fn cache_contains(state: State<'_, AppState>, items: Vec<CacheKey>) -> Result<Vec<bool>, String> {
    Ok(items.iter().map(|i| audio::source::is_cached(&state.cache_dir, &i.cache_key, &i.suffix)).collect())
}

#[tauri::command]
async fn cache_open_dir(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    std::fs::create_dir_all(&state.cache_dir).map_err(|e| e.to_string())?;
    app.opener().open_path(state.cache_dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cache_clear(state: State<'_, AppState>) -> Result<(), String> {
    audio::source::clear_cache(&state.cache_dir);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch just brings the running player to the front.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let cache_dir = app.path().app_cache_dir()?.join("stream");
            let handle = app.handle().clone();
            let engine = Engine::spawn(cache_dir.clone(), move |event| {
                let _ = handle.emit("player", event);
            });
            let handle = app.handle().clone();
            let prefetch = Prefetcher::spawn(cache_dir.clone(), move |event| {
                let _ = handle.emit("cache", event);
            });
            let handle = app.handle().clone();
            let discord = discord::Discord::spawn(move |status| {
                let _ = handle.emit("discord", status);
            });
            app.manage(AppState { engine, discord, prefetch, cache_dir });
            app.manage(media::Media::default());
            media::init_controls(app.handle());
            media::init_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            player_play,
            player_set_next,
            player_pause,
            player_resume,
            player_stop,
            player_seek,
            player_set_volume,
            player_set_dsp,
            player_set_output,
            player_set_visualizer,
            player_set_options,
            list_output_devices,
            cache_info,
            cache_clear,
            cache_prefetch,
            discord_configure,
            discord_update,
            cache_contains,
            cache_open_dir,
            media::media_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SoSamb0 Music");
}
