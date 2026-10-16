use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::clipboard;
use crate::paths;
use crate::settings::Settings;
use crate::state::{AppState, ClipFingerprint};
use crate::storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayData {
    pub clips: Vec<storage::Clip>,
    pub metadata: storage::DayInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherState {
    pub paused: bool,
}

#[tauri::command]
pub fn get_days(state: State<AppState>) -> Result<Vec<storage::DayInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    storage::get_days(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_day_data(state: State<AppState>, date: String) -> Result<DayData, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let media = paths::media_dir();
    let (clips, metadata) =
        storage::get_day_data(&db, &date, &media).map_err(|e| e.to_string())?;
    Ok(DayData { clips, metadata })
}

#[tauri::command]
pub fn update_day_label(state: State<AppState>, date: String, label: String) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    Ok(storage::update_day_label(&db, &date, &label))
}

#[tauri::command]
pub fn search_clips(
    state: State<AppState>,
    query: String,
    date: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<storage::Clip>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let media = paths::media_dir();
    storage::search_clips(&db, &query, date.as_deref(), limit, &media).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pins(state: State<AppState>) -> Result<Vec<storage::Pin>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    storage::get_pins(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_pin(
    state: State<AppState>,
    kind: String,
    ref_id: String,
    note: Option<String>,
    preview: Option<String>,
    date: Option<String>,
) -> Result<storage::Pin, String> {
    if kind != "clip" && kind != "day" {
        return Err("invalid pin kind".to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    storage::set_pin(
        &db,
        &kind,
        &ref_id,
        note.as_deref().unwrap_or(""),
        preview.as_deref().unwrap_or(""),
        date.as_deref().unwrap_or(""),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_pin(state: State<AppState>, kind: String, ref_id: String) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    storage::remove_pin(&db, &kind, &ref_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_clip(app: AppHandle, state: State<AppState>, id: i64) -> Result<bool, String> {
    let media = paths::media_dir();
    let date = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let clip = storage::get_clip_by_id(&db, id, &media).map_err(|e| e.to_string())?;
        let Some(clip) = clip else { return Ok(false) };
        let date = clip.date.clone();
        let ok = storage::delete_clip(&db, id, &media).map_err(|e| e.to_string())?;
        if !ok {
            return Ok(false);
        }
        date
    };
    let _ = app.emit("clips-changed", serde_json::json!({ "date": date }));
    Ok(true)
}

fn retry_three(mut op: impl FnMut() -> bool) -> bool {
    for _ in 0..3 {
        if op() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}

fn write_text_clipboard(text: &str) -> bool {
    retry_three(|| arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)).is_ok())
}

fn write_image_clipboard(png_bytes: &[u8]) -> bool {
    let Ok(img) = image::load_from_memory(png_bytes) else {
        return false;
    };
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let raw = rgba.into_raw();
    retry_three(|| {
        let Ok(mut cb) = arboard::Clipboard::new() else {
            return false;
        };
        cb.set_image(arboard::ImageData {
            width: w as usize,
            height: h as usize,
            bytes: std::borrow::Cow::from(&raw),
        })
        .is_ok()
    })
}

#[cfg(windows)]
fn write_files_clipboard(paths: &[String]) -> bool {
    use clipboard_win::raw;
    retry_three(|| {
        let Ok(_guard) = clipboard_win::Clipboard::new_attempts(10) else {
            return false;
        };
        let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        raw::set_file_list(&refs).is_ok()
    })
}

#[cfg(not(windows))]
fn write_files_clipboard(_paths: &[String]) -> bool {
    false
}

#[tauri::command]
pub fn copy_clip(app: AppHandle, state: State<AppState>, id: i64) -> Result<bool, String> {
    let media = paths::media_dir();
    let clip = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        storage::get_clip_by_id(&db, id, &media).map_err(|e| e.to_string())?
    };
    let Some(clip) = clip else { return Ok(false) };
    match clip.kind.as_str() {
        "text" => {
            let text = clip.text.unwrap_or_default();
            if write_text_clipboard(&text) {
                clipboard::note_self_written_fingerprint(
                    &app,
                    ClipFingerprint::Text(text),
                );
                Ok(true)
            } else {
                Ok(false)
            }
        }
        "image" => {
            let Some(mp) = clip.media_path else { return Ok(false) };
            let Ok(bytes) = std::fs::read(&mp) else {
                return Ok(false);
            };
            let hash = crate::storage::media::hash_bytes(&bytes);
            if write_image_clipboard(&bytes) {
                clipboard::note_self_written_fingerprint(
                    &app,
                    ClipFingerprint::Image(hash),
                );
                Ok(true)
            } else {
                Ok(false)
            }
        }
        "files" => {
            let paths = clip.file_paths.unwrap_or_default();
            let existing: Vec<String> = paths
                .iter()
                .filter(|p| std::path::Path::new(p).exists())
                .cloned()
                .collect();
            if existing.is_empty() {
                return Ok(false);
            }
            if !write_files_clipboard(&existing) {
                return Ok(false);
            }
            clipboard::note_self_written_fingerprint(
                &app,
                ClipFingerprint::Files(clipboard::normalize_file_set(&existing)),
            );
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    Ok(state.settings.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<AppState>,
    patch: Option<serde_json::Value>,
) -> Result<Settings, String> {
    let settings = {
        let mut s = state.settings.lock().map_err(|e| e.to_string())?;
        if let Some(p) = patch {
            s.apply_patch(&p);
        }
        let path = paths::settings_path();
        s.save_to_path(&path).map_err(|e| e.to_string())?;
        s.clone()
    };
    apply_autostart(&app, settings.launch_at_login);
    Ok(settings)
}

fn apply_autostart(app: &AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let r = if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };
    if let Err(e) = r {
        log::warn!("autostart apply failed: {e}");
    }
}

#[tauri::command]
pub fn get_watcher_state(state: State<AppState>) -> Result<WatcherState, String> {
    Ok(WatcherState {
        paused: state.paused.load(std::sync::atomic::Ordering::SeqCst),
    })
}

#[tauri::command]
pub fn set_paused(app: AppHandle, state: State<AppState>, paused: bool) -> Result<WatcherState, String> {
    let _ = state;
    let now = crate::tray::set_paused_state(&app, paused);
    Ok(WatcherState { paused: now })
}

#[tauri::command]
pub fn exit_app(app: AppHandle) {
    app.exit(0);
}
