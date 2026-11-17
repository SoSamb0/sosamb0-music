//! Bridge to the Android media session (`MediaBridge.kt` in gen/android): lock screen and
//! notification controls, headset buttons, audio focus, the foreground playback service.
//!
//! The interface reports what is playing with `media_update` (the same command the Windows
//! controls use); Kotlin sends button presses back as `media-key` events with the same action
//! strings, so `src/lib/native.ts` handles both platforms alike.

use std::sync::OnceLock;

use jni::objects::{GlobalRef, JClass, JString, JValue};
use jni::{JNIEnv, JavaVM};
use tauri::{AppHandle, Emitter};

static APP: OnceLock<AppHandle> = OnceLock::new();
/// Status bar and gesture bar heights in CSS px, as last reported by the activity.
static INSETS: std::sync::Mutex<(f32, f32)> = std::sync::Mutex::new((0.0, 0.0));

pub fn insets() -> (f32, f32) {
    *INSETS.lock().unwrap()
}

/// `MediaBridge.nativeInsets(top, bottom)`
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_lumen_player_MediaBridge_nativeInsets(_env: JNIEnv, _class: JClass, top: f32, bottom: f32) {
    *INSETS.lock().unwrap() = (top, bottom);
    if let Some(app) = APP.get() {
        let _ = app.emit("insets", serde_json::json!({ "top": top, "bottom": bottom }));
    }
}
/// The JVM and the `MediaBridge` class, captured on the main thread: classes of the app cannot
/// be looked up by name from threads Rust started.
static BRIDGE: OnceLock<(JavaVM, GlobalRef)> = OnceLock::new();

pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

/// `MediaBridge.nativeInit()`
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_lumen_player_MediaBridge_nativeInit(env: JNIEnv, class: JClass) {
    let (Ok(vm), Ok(class)) = (env.get_java_vm(), env.new_global_ref(class)) else { return };
    let _ = BRIDGE.set((vm, class));
}

/// `MediaBridge.nativeAction(action)`: a button on the lock screen, notification or headset.
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_lumen_player_MediaBridge_nativeAction(mut env: JNIEnv, _class: JClass, action: JString) {
    let Ok(action) = env.get_string(&action) else { return };
    let action: String = action.into();
    if let Some(app) = APP.get() {
        let _ = app.emit("media-key", action);
    }
}

#[tauri::command]
pub fn media_update(info: serde_json::Value) {
    let Some((vm, class)) = BRIDGE.get() else { return };
    let Ok(mut env) = vm.attach_current_thread() else { return };
    let Ok(json) = env.new_string(info.to_string()) else { return };
    let class: &JClass = class.as_obj().into();
    let result = env.call_static_method(class, "update", "(Ljava/lang/String;)V", &[JValue::Object(&json)]);
    if result.is_err() {
        let _ = env.exception_clear();
    }
}
