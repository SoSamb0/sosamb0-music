mod audio;
mod media;

use std::path::PathBuf;

use serde::Serialize;
use tauri::{Emitter, Manager, State};

use audio::{Command, DeviceInfo, DspConfig, Engine, EngineHandle, Options, OutputConfig, TrackReq};

struct AppState {
    engine: EngineHandle,
    cache_dir: PathBuf,
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
}

#[tauri::command]
async fn cache_info(state: State<'_, AppState>) -> Result<CacheInfo, String> {
    let (bytes, files) = audio::source::cache_usage(&state.cache_dir);
    Ok(CacheInfo { bytes, files })
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
            app.manage(AppState { engine, cache_dir });
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
            player_set_options,
            list_output_devices,
            cache_info,
            cache_clear,
            media::media_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Lumen");
}
