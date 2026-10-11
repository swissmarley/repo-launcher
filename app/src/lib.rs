mod commands;

use launcher_core::{AppPaths, Engine};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{Manager, RunEvent};
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub engine: Arc<Engine>,
    /// Cancellation handles for installs in progress, by project id.
    pub installs: Mutex<HashMap<String, CancellationToken>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let paths = AppPaths::for_current_user().ok_or("no per-user data folder")?;
            let engine = Engine::open(paths).map_err(|p| p.to_string())?;
            app.manage(AppState {
                engine: Arc::new(engine),
                installs: Mutex::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::add_project,
            commands::install_project,
            commands::cancel_install,
            commands::launch_project,
            commands::open_project,
            commands::stop_project,
            commands::uninstall_project,
            commands::project_log,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build the app");

    app.run(|handle, event| {
        // Projects never outlive the Launcher (DECISIONS D-005).
        if let RunEvent::Exit = event {
            let engine = handle.state::<AppState>().engine.clone();
            tauri::async_runtime::block_on(engine.stop_all());
        }
    });
}
