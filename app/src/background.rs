//! Background mode (DECISIONS D-004): shortcuts and starting at login run the app with only a
//! tray icon. A shortcut's request (`--launch <id>`, or the link `repolauncher://launch/<id>`)
//! launches its project straight away: web projects open in the browser, terminal programs in
//! their console window. The main window only appears when the user asks for it, or when a
//! launch needs them (a problem, or keys to enter first).

use crate::{AppState, commands, tray};
use launcher_core::{Problem, ProblemCode};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};

/// What a start of the app (or a second start, forwarded to this one) asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    /// Launch this project without showing the main window.
    Launch(String),
    /// Started at login: stay in the tray and start the projects set to start at login.
    Login,
}

/// Reads `--launch <id>` / `--background` from the command line. Links are handled separately
/// (by the deep-link plugin), so they are never acted on twice.
pub fn parse_args(args: &[String]) -> Option<Request> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--launch" => return it.next().and_then(|id| valid_id(id)).map(Request::Launch),
            "--background" => return Some(Request::Login),
            _ => {}
        }
    }
    None
}

/// `repolauncher://launch/<id>`. Anything else is ignored: links can come from any web page,
/// and they can only ever launch a project that is already installed.
pub fn parse_link(link: &url::Url) -> Option<Request> {
    if link.scheme() != "repolauncher" || link.host_str() != Some("launch") {
        return None;
    }
    let id = link.path().trim_matches('/');
    valid_id(id).map(Request::Launch)
}

fn valid_id(id: &str) -> Option<String> {
    (!id.is_empty() && id.len() <= 16 && id.bytes().all(|b| b.is_ascii_alphanumeric()))
        .then(|| id.to_ascii_lowercase())
}

/// A message for the main window. Queued, because the window may still be loading when a
/// background launch fails; the UI drains the queue when told there is something new.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UiMessage {
    #[serde(rename_all = "camelCase")]
    LaunchProblem {
        project_id: String,
        problem: Problem,
    },
    /// The launch needs the UI first (the keys form before a first launch).
    #[serde(rename_all = "camelCase")]
    LaunchInUi { project_id: String },
    /// Quit was chosen while projects are running: ask first (D-005).
    QuitRequested,
    /// Projects that were still running after the Launcher stopped unexpectedly, and were
    /// stopped when it started again.
    #[serde(rename_all = "camelCase")]
    Orphans { project_ids: Vec<String> },
    /// The daily check found a newer version of Repo Launcher.
    UpdateAvailable { version: String },
}

pub fn post(app: &AppHandle, msg: UiMessage) {
    let state = app.state::<AppState>();
    state
        .messages
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(msg);
    let _ = app.emit("shell", ());
}

pub fn handle(app: &AppHandle, request: Request) {
    app.state::<AppState>()
        .background_request
        .store(true, Ordering::Relaxed);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match request {
            Request::Launch(id) => launch(&app, &id).await,
            Request::Login => {
                let engine = app.state::<AppState>().engine.clone();
                for p in engine.login_projects().unwrap_or_default() {
                    launch(&app, &p.id).await;
                }
            }
        }
    });
}

/// Launches a project for a shortcut or login start, without the main window if all goes well.
pub async fn launch(app: &AppHandle, id: &str) {
    let engine = app.state::<AppState>().engine.clone();
    let project = match engine.get(id) {
        Ok(p) => p,
        Err(problem) => return show_problem(app, id, problem),
    };
    if engine.is_running(id) {
        // Already running: bring it to the front.
        if let Err(problem) = commands::bring_to_front(app, &project) {
            show_problem(app, id, problem);
        }
        return;
    }
    let needs_keys_form = !project.keys_reviewed
        && engine
            .env_vars(id)
            .is_ok_and(|vars| vars.iter().any(|v| !v.has_value));
    if needs_keys_form {
        show_main(app);
        return post(
            app,
            UiMessage::LaunchInUi {
                project_id: id.to_owned(),
            },
        );
    }
    if let Err(problem) = commands::launch(app, id, None).await {
        show_problem(app, id, problem);
    }
}

fn show_problem(app: &AppHandle, id: &str, problem: Problem) {
    log::warn!("background launch of {id}: {problem}");
    if problem.code == ProblemCode::AlreadyRunning {
        return;
    }
    show_main(app);
    post(
        app,
        UiMessage::LaunchProblem {
            project_id: id.to_owned(),
            problem,
        },
    );
}

pub fn show_main(app: &AppHandle) {
    log::info!("showing the main window");
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn hide_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    // Only the menu bar icon is left (console windows still show).
    #[cfg(target_os = "macos")]
    if !app
        .webview_windows()
        .values()
        .any(|w| w.is_visible().unwrap_or(false))
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    }
}

/// The main window's close button: the app stays in the tray while projects run (D-005), and
/// when it was started at login. Otherwise closing it quits, as people expect.
pub fn on_close_main(app: &AppHandle) {
    let state = app.state::<AppState>();
    hide_main(app);
    let busy = !state.engine.running_ids().is_empty();
    let others = app
        .webview_windows()
        .values()
        .any(|w| w.label() != "main" && w.is_visible().unwrap_or(false));
    let resident = state.started_at_login && state.tray_available.load(Ordering::Relaxed);
    if !busy && !others && !resident {
        quit_now(app);
    }
}

/// Quit from the tray or the app menu: asks first if anything is running (D-005).
pub fn request_quit(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.engine.running_ids().is_empty() {
        quit_now(app);
    } else {
        show_main(app);
        post(app, UiMessage::QuitRequested);
    }
}

/// Quits; `RunEvent::Exit` then stops every project.
pub fn quit_now(app: &AppHandle) {
    app.state::<AppState>()
        .quitting
        .store(true, Ordering::Relaxed);
    app.exit(0);
}

/// Refreshes what shows a project's state outside the main window.
pub fn on_run_state(app: &AppHandle) {
    tray::refresh(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn reads_requests_from_the_command_line() {
        assert_eq!(
            parse_args(&args(&["--launch", "k3f9x2ab"])),
            Some(Request::Launch("k3f9x2ab".into()))
        );
        assert_eq!(parse_args(&args(&["--background"])), Some(Request::Login));
        assert_eq!(parse_args(&args(&["--launch", "../../etc"])), None);
        assert_eq!(parse_args(&args(&["--launch"])), None);
        assert_eq!(parse_args(&args(&["repolauncher://launch/k3f9x2ab"])), None);
        assert_eq!(parse_args(&args(&[])), None);
    }

    #[test]
    fn reads_only_launch_links() {
        let link = |s: &str| parse_link(&url::Url::parse(s).unwrap());
        assert_eq!(
            link("repolauncher://launch/k3f9x2ab"),
            Some(Request::Launch("k3f9x2ab".into()))
        );
        assert_eq!(
            link("repolauncher://launch/k3f9x2ab/"),
            Some(Request::Launch("k3f9x2ab".into()))
        );
        assert_eq!(link("repolauncher://install/k3f9x2ab"), None);
        assert_eq!(link("repolauncher://launch/a%2Fb"), None);
        assert_eq!(link("repolauncher://launch/"), None);
        assert_eq!(link("https://launch/k3f9x2ab"), None);
    }
}
