//! Operating-system media integration: the Windows media overlay, hardware media keys,
//! and the tray icon. Everything here forwards intent to the UI as an event; the UI owns
//! the queue and decides what "next" means.

use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
pub struct Media {
    controls: Mutex<Option<MediaControls>>,
    /// Identity of the metadata last sent, to avoid re-sending (and re-fetching art) every tick.
    last: Mutex<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    title: String,
    artist: String,
    album: String,
    cover_url: String,
    duration: f64,
    position: f64,
    state: String,
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Hook up the system media controls. Failure is not fatal: the player works without them.
pub fn init_controls(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    #[cfg(windows)]
    let hwnd = match window.hwnd() {
        Ok(h) => Some(h.0 as *mut std::ffi::c_void),
        Err(_) => return,
    };
    #[cfg(not(windows))]
    let hwnd = {
        let _ = &window;
        None
    };

    let config = PlatformConfig { display_name: "Lumen", dbus_name: "lumen", hwnd };
    let Ok(mut controls) = MediaControls::new(config) else { return };
    let handle = app.clone();
    let attached = controls.attach(move |event| {
        let action = match event {
            MediaControlEvent::Play => "play".to_string(),
            MediaControlEvent::Pause => "pause".to_string(),
            MediaControlEvent::Toggle => "toggle".to_string(),
            MediaControlEvent::Next => "next".to_string(),
            MediaControlEvent::Previous => "previous".to_string(),
            MediaControlEvent::Stop => "stop".to_string(),
            MediaControlEvent::SetPosition(MediaPosition(at)) => format!("seek:{}", at.as_secs_f64()),
            MediaControlEvent::Raise => {
                show_main(&handle);
                return;
            }
            _ => return,
        };
        let _ = handle.emit("media-key", action);
    });
    if attached.is_ok() {
        *app.state::<Media>().controls.lock().unwrap() = Some(controls);
    }
}

#[tauri::command]
pub fn media_update(state: State<Media>, info: MediaInfo) {
    let mut guard = state.controls.lock().unwrap();
    let Some(controls) = guard.as_mut() else { return };

    let identity = format!("{}\u{0}{}\u{0}{}\u{0}{}", info.title, info.artist, info.album, info.duration);
    let mut last = state.last.lock().unwrap();
    if *last != identity {
        let _ = controls.set_metadata(MediaMetadata {
            title: Some(&info.title),
            artist: Some(&info.artist),
            album: Some(&info.album),
            cover_url: (!info.cover_url.is_empty()).then_some(info.cover_url.as_str()),
            duration: (info.duration > 0.0).then(|| Duration::from_secs_f64(info.duration)),
        });
        *last = identity;
    }

    let progress = Some(MediaPosition(Duration::from_secs_f64(info.position.max(0.0))));
    let playback = match info.state.as_str() {
        "playing" => MediaPlayback::Playing { progress },
        "paused" => MediaPlayback::Paused { progress },
        _ => MediaPlayback::Stopped,
    };
    let _ = controls.set_playback(playback);
}

pub fn init_tray(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, label: &str| MenuItem::with_id(app, id, label, true, None::<&str>);
    let menu = Menu::with_items(
        app,
        &[
            &item("toggle", "Играть / пауза")?,
            &item("next", "Следующий трек")?,
            &item("previous", "Предыдущий трек")?,
            &PredefinedMenuItem::separator(app)?,
            &item("show", "Открыть Lumen")?,
            &item("quit", "Выйти")?,
        ],
    )?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Lumen")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            action => {
                let _ = app.emit("tray", action);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
