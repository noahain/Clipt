mod clipboard;
mod commands;
mod paths;
mod settings;
mod state;
mod storage;
mod tray;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Folder {
                        path: paths::logs_dir(),
                        file_name: Some("clipt".into()),
                    }),
                ])
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .build(),
        )
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--startup"])
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let _ = paths::ensure_dirs();

            let db_path = paths::db_path();
            let db = storage::open_or_create(&db_path)
                .expect("storage open failed even after quarantine");
            if let Err(e) = storage::migrate::run(&db, &paths::app_root(), &paths::media_dir())
            {
                log::warn!("legacy migration failed: {e:#}");
            }

            let settings_path = paths::settings_path();
            let mut st = settings::Settings::load_from_path(&settings_path);
            let already: bool =
                storage::get_meta(&db, "dotenv_import_done").is_some();
            if settings::apply_dotenv_once(&mut st, &paths::legacy_env_path(), already) {
                let _ = storage::set_meta(&db, "dotenv_import_done", "1");
                let _ = st.save_to_path(&settings_path);
            } else if !already && !paths::legacy_env_path().exists() {
                let _ = storage::set_meta(&db, "dotenv_import_done", "1");
            }

            {
                use tauri_plugin_autostart::ManagerExt;
                let handle = app.handle().clone();
                if st.launch_at_login {
                    let _ = handle.autolaunch().enable();
                }
            }

            app.manage(AppState::new(db, st));

            if let Err(e) = tray::build_tray(app.handle()) {
                log::warn!("tray build failed: {e:#}");
            }

            let startup = std::env::args().any(|a| a == "--startup");
            if !startup && let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }

            let handle = app.handle().clone();
            clipboard::spawn_listener(handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_days,
            commands::get_day_data,
            commands::update_day_label,
            commands::search_clips,
            commands::get_pins,
            commands::set_pin,
            commands::remove_pin,
            commands::delete_clip,
            commands::copy_clip,
            commands::get_settings,
            commands::update_settings,
            commands::get_watcher_state,
            commands::set_paused,
            commands::exit_app,
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
