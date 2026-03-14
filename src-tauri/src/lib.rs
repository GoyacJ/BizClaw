use tauri::{tray::TrayIconBuilder, Manager};

pub mod app_menu;
pub mod commands;
pub mod config_store;
pub mod install;
pub mod operation_supervisor;
pub mod runtime;
pub mod runtime_state;
pub mod runtime_supervisor;
pub mod secret_store;
pub mod types;
pub mod validation;

pub fn run() {
    tauri::Builder::default()
        .manage(commands::AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_handle = app.app_handle();
            let saved_ui_preferences =
                commands::load_ui_preferences(&app_handle).unwrap_or_default();
            let _ = commands::sync_main_window_appearance(&app_handle, &saved_ui_preferences);

            if let Some(window) = app.get_webview_window("main") {
                let window_handle = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_handle.hide();
                    }
                });
                let _ = window.show();
            }

            let app_menu = app_menu::build_app_menu(app.app_handle())?;
            app_menu.set_as_app_menu()?;
            let tray_menu_state = app_menu::build_tray_menu(app.app_handle())?;
            let tray_menu = tray_menu_state.tray_menu.clone();
            app.manage(tray_menu_state);
            app_menu::refresh_status_menu_from_state(app.app_handle())?;
            let default_icon = app.default_window_icon().cloned();

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| handle_app_menu_event(app, event.id().as_ref()));

            if let Some(icon) = default_icon {
                tray_builder = tray_builder.icon(icon);
            }

            tray_builder.build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::detect_environment,
            commands::install_openclaw,
            commands::check_openclaw_update,
            commands::update_openclaw,
            commands::get_operation_status,
            commands::get_operation_events,
            commands::stop_openclaw_operation,
            commands::open_manual_install,
            commands::open_support_url,
            commands::save_profile,
            commands::save_ui_preferences,
            commands::test_connection,
            commands::start_runtime,
            commands::stop_runtime,
            commands::get_runtime_status,
            commands::stream_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn handle_app_menu_event(app: &tauri::AppHandle, event_id: &str) {
    match event_id {
        app_menu::MENU_SHOW_ID => show_window(app),
        app_menu::MENU_REFRESH_ID => app_menu::emit_refresh_request(app),
        app_menu::MENU_QUIT_ID => {
            let state = app.state::<commands::AppState>();
            let locale = state
                .environment_cache
                .lock()
                .expect("environment cache mutex poisoned")
                .as_ref()
                .map(|snapshot| snapshot.ui_preferences.locale)
                .unwrap_or_default();
            let _ = runtime_supervisor::stop_runtime_processes(
                state.runtime.clone(),
                app.clone(),
                locale,
            );
            let _ = commands::stop_operation_for_exit(app);
            app.exit(0);
        }
        _ => {}
    }
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
