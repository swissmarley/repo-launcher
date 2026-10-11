mod background;
mod commands;
mod exec;
mod strings;
mod tray;

pub use exec::run_if_requested as exec_if_requested;

use background::Request;
use launcher_core::shortcuts::Launcher;
use launcher_core::{AppPaths, Engine};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_deep_link::DeepLinkExt;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub engine: Arc<Engine>,
    /// Cancellation handles for installs in progress, by project id.
    pub installs: Mutex<HashMap<String, CancellationToken>>,
    /// Cancels a GitHub sign-in that is waiting for the user.
    pub sign_in: Mutex<Option<CancellationToken>>,
    /// What shortcuts start: this app.
    pub launcher: Launcher,
    /// Messages for the main window, collected until it asks for them.
    pub messages: Mutex<Vec<background::UiMessage>>,
    /// Quit was confirmed (or nothing was running): let the app exit.
    pub quitting: AtomicBool,
    /// A shortcut or login start asked for something: keep the main window hidden.
    pub background_request: AtomicBool,
    pub started_at_login: bool,
    pub tray_available: AtomicBool,
}

fn launcher() -> Launcher {
    let mut l = Launcher::current().unwrap_or_default();
    l.icon_png = Some(include_bytes!("../icons/128x128.png").to_vec());
    l.icon_icns = Some(include_bytes!("../icons/icon.icns").to_vec());
    l
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // First, so a second start hands its request to this one and exits. With the
        // `deep-link` feature, links in its arguments reach `on_open_url` below.
        .plugin(tauri_plugin_single_instance::init(
            |app, argv, _cwd| match background::parse_args(argv.get(1..).unwrap_or_default()) {
                Some(request) => background::handle(app, request),
                None if argv.iter().any(|a| a.starts_with("repolauncher:")) => {}
                None => background::show_main(app),
            },
        ))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Until we know whether a shortcut started us, there is no Dock icon.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let paths = AppPaths::for_current_user().ok_or("no per-user data folder")?;
            let engine = Arc::new(Engine::open(paths).map_err(|p| p.to_string())?);
            let args: Vec<String> = std::env::args().skip(1).collect();
            let mut request = background::parse_args(&args);
            let handle = app.handle().clone();
            if let Some(links) = app.deep_link().get_current()? {
                request = request.or(links.iter().find_map(background::parse_link));
            }
            app.manage(AppState {
                engine: engine.clone(),
                installs: Mutex::default(),
                sign_in: Mutex::default(),
                launcher: launcher(),
                messages: Mutex::default(),
                quitting: AtomicBool::new(false),
                background_request: AtomicBool::new(request.is_some()),
                started_at_login: request == Some(Request::Login),
                tray_available: AtomicBool::new(false),
            });

            // Links while running (and on macOS also the one that started the app).
            let h = handle.clone();
            app.deep_link().on_open_url(move |event| {
                for request in event.urls().iter().filter_map(background::parse_link) {
                    background::handle(&h, request);
                }
            });
            // Installers register the link scheme; development builds and AppImages do it here.
            #[cfg(any(windows, target_os = "linux"))]
            if let Err(e) = app.deep_link().register_all() {
                log::warn!("registering repolauncher:// links: {e}");
            }
            #[cfg(target_os = "macos")]
            app.set_menu(tray::app_menu(&handle)?)?;
            if let Err(e) = tray::create(&handle) {
                log::warn!("no tray icon: {e}");
            }

            // Projects still running from before an unexpected stop are stopped before any
            // shortcut request is handled, so they aren't running twice.
            let h = handle.clone();
            tauri::async_runtime::spawn(async move {
                let state = h.state::<AppState>();
                match state.engine.reconcile().await {
                    Ok(stopped) if !stopped.is_empty() => {
                        log::info!("stopped projects left running from last time: {stopped:?}");
                        background::post(
                            &h,
                            background::UiMessage::Orphans {
                                project_ids: stopped,
                            },
                        );
                    }
                    Ok(_) => {}
                    Err(p) => log::warn!("checking for projects left running: {p}"),
                }
                match state.engine.repair_shortcuts(&state.launcher) {
                    Ok(n) if n > 0 => log::info!("repaired {n} shortcuts"),
                    Ok(_) => {}
                    Err(p) => log::warn!("repairing shortcuts: {p}"),
                }
                if let Some(request) = request {
                    log::info!("background start: {request:?}; the main window stays hidden");
                    background::handle(&h, request);
                }
            });

            if !app
                .state::<AppState>()
                .background_request
                .load(Ordering::Relaxed)
            {
                // macOS delivers the link that started the app just after startup; give it a
                // moment before showing the window.
                let h = handle.clone();
                let wait = if cfg!(target_os = "macos") { 400 } else { 0 };
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(wait)).await;
                    let state = h.state::<AppState>();
                    if !state.background_request.load(Ordering::Relaxed) {
                        background::show_main(&h);
                    }
                });
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id().as_ref() == tray::APP_QUIT {
                background::request_quit(app);
            }
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && let WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                background::on_close_main(window.app_handle());
            }
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
            commands::get_project,
            commands::open_console,
            commands::console_write,
            commands::console_resize,
            commands::console_scrollback,
            commands::update_project,
            commands::repair_project,
            commands::review_project,
            commands::consent_project,
            commands::env_vars,
            commands::set_env_value,
            commands::mark_keys_reviewed,
            commands::app_settings,
            commands::set_trusted_publisher,
            commands::set_ai_enabled,
            commands::set_ai_key,
            commands::github_begin_sign_in,
            commands::github_finish_sign_in,
            commands::github_cancel_sign_in,
            commands::github_sign_out,
            commands::github_repositories,
            commands::open_external,
            commands::ai_preview,
            commands::ai_suggest,
            commands::create_shortcut,
            commands::remove_shortcut,
            commands::set_start_at_login,
            commands::set_use_system_terminal,
            commands::shell_settings,
            commands::set_launch_at_login,
            commands::take_messages,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build the app");

    app.run(|handle, event| match event {
        // Quit while projects run asks first (D-005), wherever it came from.
        RunEvent::ExitRequested { api, .. } => {
            let state = handle.state::<AppState>();
            if !state.quitting.load(Ordering::Relaxed) && !state.engine.running_ids().is_empty() {
                api.prevent_exit();
                background::request_quit(handle);
            }
        }
        // Projects never outlive the Launcher (DECISIONS D-005).
        RunEvent::Exit => {
            let engine = handle.state::<AppState>().engine.clone();
            tauri::async_runtime::block_on(engine.stop_all());
        }
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => background::show_main(handle),
        _ => {}
    });
}
