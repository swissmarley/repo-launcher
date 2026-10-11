//! The tray (Windows, Linux) or menu bar (macOS) icon: running projects with Open and Stop,
//! Show Repo Launcher, and Quit. Nothing depends on it: some Linux desktops have no tray.

use crate::strings::t;
use crate::{AppState, background, commands};
use std::sync::atomic::Ordering;
use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

const TRAY_ID: &str = "main";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(t("tray.tooltip", &[]))
        .menu(&build_menu(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()));
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.state::<AppState>()
        .tray_available
        .store(true, Ordering::Relaxed);
    Ok(())
}

/// Rebuilds the menu after a project starts or stops.
pub fn refresh(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = handle.tray_by_id(TRAY_ID)
            && let Ok(menu) = build_menu(&handle)
        {
            let _ = tray.set_menu(Some(menu));
        }
    });
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let engine = app.state::<AppState>().engine.clone();
    let running: Vec<_> = engine
        .list()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| engine.is_running(&p.id))
        .collect();
    let menu = Menu::new(app)?;
    if running.is_empty() {
        menu.append(&MenuItem::new(
            app,
            t("tray.nothingRunning", &[]),
            false,
            None::<&str>,
        )?)?;
    } else {
        menu.append(&MenuItem::new(
            app,
            t("tray.running", &[]),
            false,
            None::<&str>,
        )?)?;
        for p in &running {
            let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
            if engine.running_url(&p.id).is_some() {
                items.push(Box::new(MenuItem::with_id(
                    app,
                    format!("open:{}", p.id),
                    t("tray.open", &[]),
                    true,
                    None::<&str>,
                )?));
            } else if p.kind == Some(launcher_core::recipe::AppKind::Terminal)
                && !p.use_system_terminal
            {
                items.push(Box::new(MenuItem::with_id(
                    app,
                    format!("open:{}", p.id),
                    t("tray.showConsole", &[]),
                    true,
                    None::<&str>,
                )?));
            }
            items.push(Box::new(MenuItem::with_id(
                app,
                format!("stop:{}", p.id),
                t("tray.stop", &[]),
                true,
                None::<&str>,
            )?));
            let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|i| i.as_ref()).collect();
            menu.append(&Submenu::with_items(app, &p.name, true, &refs)?)?;
        }
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "show",
        t("tray.show", &[]),
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        t("tray.quit", &[]),
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

fn on_menu(app: &AppHandle, id: &str) {
    match id.split_once(':') {
        _ if id == "show" => background::show_main(app),
        _ if id == "quit" => background::request_quit(app),
        Some(("open", project)) => {
            let engine = app.state::<AppState>().engine.clone();
            if let Ok(p) = engine.get(project) {
                let _ = commands::bring_to_front(app, &p);
            }
        }
        Some(("stop", project)) => {
            let engine = app.state::<AppState>().engine.clone();
            let project = project.to_owned();
            tauri::async_runtime::spawn(async move { engine.stop(&project).await });
        }
        _ => {}
    }
}

/// The macOS app menu. Its Quit (⌘Q) is our own item rather than the system one, so quitting
/// while projects run asks first (D-005); the system's quit can't be held back.
#[cfg(target_os = "macos")]
pub fn app_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    use tauri::menu::AboutMetadata;
    let p = |item: tauri::Result<PredefinedMenuItem<Wry>>| item;
    let name = app.package_info().name.clone();
    let app_items = Submenu::with_items(
        app,
        &name,
        true,
        &[
            &p(PredefinedMenuItem::about(
                app,
                None,
                Some(AboutMetadata::default()),
            ))?,
            &p(PredefinedMenuItem::separator(app))?,
            &p(PredefinedMenuItem::services(app, None))?,
            &p(PredefinedMenuItem::separator(app))?,
            &p(PredefinedMenuItem::hide(app, None))?,
            &p(PredefinedMenuItem::hide_others(app, None))?,
            &p(PredefinedMenuItem::show_all(app, None))?,
            &p(PredefinedMenuItem::separator(app))?,
            &MenuItem::with_id(
                app,
                APP_QUIT,
                t("tray.quit", &[]),
                true,
                Some("CmdOrCtrl+Q"),
            )?,
        ],
    )?;
    // Copy and paste in text fields need the Edit menu's shortcuts.
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &p(PredefinedMenuItem::undo(app, None))?,
            &p(PredefinedMenuItem::redo(app, None))?,
            &p(PredefinedMenuItem::separator(app))?,
            &p(PredefinedMenuItem::cut(app, None))?,
            &p(PredefinedMenuItem::copy(app, None))?,
            &p(PredefinedMenuItem::paste(app, None))?,
            &p(PredefinedMenuItem::select_all(app, None))?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &p(PredefinedMenuItem::minimize(app, None))?,
            &p(PredefinedMenuItem::fullscreen(app, None))?,
            &p(PredefinedMenuItem::separator(app))?,
            &p(PredefinedMenuItem::close_window(app, None))?,
        ],
    )?;
    Menu::with_items(app, &[&app_items, &edit, &window])
}

pub const APP_QUIT: &str = "app-quit";
