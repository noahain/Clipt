use tauri::{
    menu::{MenuBuilder, MenuItem, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

use crate::state::AppState;

#[derive(Default)]
pub struct PauseMenuItem(pub std::sync::Mutex<Option<MenuItem<tauri::Wry>>>);

pub fn build_tray(app: &AppHandle) -> anyhow::Result<()> {
    let open = MenuItemBuilder::with_id("open", "Open Clipt").build(app)?;
    let pause_item = MenuItemBuilder::with_id("pause", "Pause").build(app)?;
    let exit = MenuItemBuilder::with_id("exit", "Exit").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &pause_item])
        .separator()
        .item(&exit)
        .build()?;

    app.manage(PauseMenuItem(std::sync::Mutex::new(Some(pause_item))));

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no default window icon"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app_handle, event| match event.id().as_ref() {
            "open" => {
                show_main(app_handle);
            }
            "pause" => {
                let state = app_handle.state::<AppState>();
                let now_paused = !state.paused.load(std::sync::atomic::Ordering::SeqCst);
                set_paused_state(app_handle, now_paused);
            }
            "exit" => {
                app_handle.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(&tray.app_handle().clone());
            }
        })
        .build(app)?;
    Ok(())
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn set_paused_state(app: &AppHandle, paused: bool) -> bool {
    let state = app.state::<AppState>();
    let prev = state.paused.load(std::sync::atomic::Ordering::SeqCst);
    if prev == paused {
        return paused;
    }
    state
        .paused
        .store(paused, std::sync::atomic::Ordering::SeqCst);
    if !paused {
        let capture = state.settings.lock().unwrap().capture.clone();
        if let Some(fp) = crate::clipboard::read_fingerprint_now(&capture) {
            *state.baseline.lock().unwrap() = Some(fp);
        }
    }
    update_pause_label(app, paused);
    let _ = app.emit("watcher-state", serde_json::json!({ "paused": paused }));
    paused
}

fn update_pause_label(app: &AppHandle, paused: bool) {
    if let Some(managed) = app.try_state::<PauseMenuItem>()
        && let Some(item) = managed.0.lock().unwrap().as_ref()
    {
        let _ = item.set_text(if paused { "Resume" } else { "Pause" });
    }
}
