//! Thin adapters between the WebView and the engine. No business logic lives here.

use crate::{AppState, background, exec};
use launcher_core::ai::Preview;
use launcher_core::engine::{AiOutcome, AppSettings, EnvVarView};
use launcher_core::recipe::AppKind;
use launcher_core::run::LaunchInfo;
use launcher_core::safety::SafetyReport;
use launcher_core::source::auth::{DeviceCode, GitHubAccount, RemoteRepo};
use launcher_core::store::Shortcut;
use launcher_core::{EngineEvent, EventSink, Problem, ProblemCode, Project};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;
use tokio_util::sync::CancellationToken;

type Result<T> = std::result::Result<T, Problem>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    #[serde(flatten)]
    project: Project,
    running: bool,
    url: Option<String>,
    shortcuts: Vec<Shortcut>,
}

fn view(state: &AppState, project: Project) -> ProjectView {
    let running = state.engine.is_running(&project.id);
    let url = state.engine.running_url(&project.id);
    let shortcuts = state.engine.shortcuts(&project.id).unwrap_or_default();
    ProjectView {
        project,
        running,
        url,
        shortcuts,
    }
}

pub fn sink(app: &AppHandle) -> EventSink {
    let app = app.clone();
    Arc::new(move |event: EngineEvent| {
        let state_changed = matches!(event, EngineEvent::RunState { .. });
        let _ = app.emit("engine", event);
        if state_changed {
            background::on_run_state(&app);
        }
    })
}

fn open_in_browser(app: &AppHandle, url: &str) -> Result<()> {
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| Problem::internal(format!("open {url}: {e}")))
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectView>> {
    Ok(state
        .engine
        .list()?
        .into_iter()
        .map(|p| view(&state, p))
        .collect())
}

#[tauri::command]
pub async fn add_project(state: State<'_, AppState>, input: String) -> Result<ProjectView> {
    let project = state.engine.add(&input).await?;
    Ok(view(&state, project))
}

#[tauri::command]
pub async fn install_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<ProjectView> {
    let token = begin_install(&state, &id)?;
    let result = state.engine.install(&id, sink(&app), token).await;
    end_install(&state, &id);
    Ok(view(&state, result?))
}

#[tauri::command]
pub fn cancel_install(state: State<'_, AppState>, id: String) {
    if let Some(token) = state
        .installs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&id)
    {
        token.cancel();
    }
}

/// Splits options typed into the console ("--size 512 'my file.png'") into arguments.
fn split_args(args: Option<&str>) -> Vec<String> {
    let Some(args) = args.map(str::trim).filter(|a| !a.is_empty()) else {
        return Vec::new();
    };
    match launcher_core::recipe::shell_words::split(args) {
        Some(simple) if simple.env.is_empty() => simple.argv,
        _ => args.split_whitespace().map(str::to_owned).collect(),
    }
}

#[tauri::command]
pub async fn launch_project(
    app: AppHandle,
    id: String,
    args: Option<String>,
) -> Result<LaunchInfo> {
    launch(&app, &id, args.as_deref()).await
}

/// Launches a project the way its settings say: web projects open in the browser, terminal
/// programs in their console window or the system's terminal. Shared by the UI's Launch button
/// and by shortcuts.
pub async fn launch(app: &AppHandle, id: &str, args: Option<&str>) -> Result<LaunchInfo> {
    let state = app.state::<AppState>();
    let project = state.engine.get(id)?;
    if project.kind == Some(AppKind::Terminal) {
        if project.use_system_terminal {
            // Checked here so problems show in the app; the terminal reads the keys itself.
            state.engine.exec_command(id, &[]).await?;
            let scratch = state.engine.paths().project(id).meta();
            exec::open_in_terminal(&state.launcher.exe, id, &scratch)?;
            return Ok(LaunchInfo {
                url: None,
                port: None,
            });
        }
        open_console_window(app, id, &project.name)?;
    }
    let info = state
        .engine
        .launch_with(id, sink(app), &split_args(args))
        .await?;
    if let Some(url) = &info.url {
        open_in_browser(app, url)?;
    }
    Ok(info)
}

/// Shows a running project: its page in the browser, or its console window.
pub fn bring_to_front(app: &AppHandle, project: &Project) -> Result<()> {
    let state = app.state::<AppState>();
    if let Some(url) = state.engine.running_url(&project.id) {
        return open_in_browser(app, &url);
    }
    if project.kind == Some(AppKind::Terminal) && !project.use_system_terminal {
        return open_console_window(app, &project.id, &project.name);
    }
    Ok(())
}

/// Terminal programs get their own small window with a real terminal in it.
fn open_console_window(app: &AppHandle, id: &str, name: &str) -> Result<()> {
    let label = format!("console-{id}");
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.set_focus();
        return Ok(());
    }
    let url = WebviewUrl::App(format!("index.html#console/{id}").into());
    WebviewWindowBuilder::new(app, label, url)
        .title(name)
        .inner_size(860.0, 540.0)
        .min_inner_size(480.0, 320.0)
        .build()
        .map(|_| ())
        .map_err(|e| Problem::internal(format!("console window: {e}")))
}

#[tauri::command]
pub fn open_console(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    let project = state.engine.get(&id)?;
    open_console_window(&app, &id, &project.name)
}

#[tauri::command]
pub fn console_write(state: State<'_, AppState>, id: String, data: String) -> Result<()> {
    state.engine.console_write(&id, &data)
}

#[tauri::command]
pub fn console_resize(state: State<'_, AppState>, id: String, cols: u16, rows: u16) {
    state.engine.console_resize(&id, cols, rows);
}

#[tauri::command]
pub fn console_scrollback(state: State<'_, AppState>, id: String) -> String {
    state.engine.console_scrollback(&id)
}

#[tauri::command]
pub fn get_project(state: State<'_, AppState>, id: String) -> Result<ProjectView> {
    let project = state.engine.get(&id)?;
    Ok(view(&state, project))
}

/// Returns true if a newer version was installed.
#[tauri::command]
pub async fn update_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<bool> {
    let token = begin_install(&state, &id)?;
    let result = state.engine.update(&id, sink(&app), token).await;
    end_install(&state, &id);
    result
}

#[tauri::command]
pub async fn repair_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<ProjectView> {
    let token = begin_install(&state, &id)?;
    let result = state.engine.repair(&id, sink(&app), token).await;
    end_install(&state, &id);
    Ok(view(&state, result?))
}

fn begin_install(state: &AppState, id: &str) -> Result<CancellationToken> {
    let mut installs = state.installs.lock().unwrap_or_else(|e| e.into_inner());
    if installs.contains_key(id) {
        return Err(Problem::new(ProblemCode::Internal).detail("already installing"));
    }
    let token = CancellationToken::new();
    installs.insert(id.to_owned(), token.clone());
    Ok(token)
}

fn end_install(state: &AppState, id: &str) {
    state
        .installs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(id);
}

#[tauri::command]
pub fn open_project(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    match state.engine.running_url(&id) {
        Some(url) => open_in_browser(&app, &url),
        None => Err(Problem::new(ProblemCode::NotInstalled)),
    }
}

#[tauri::command]
pub async fn stop_project(state: State<'_, AppState>, id: String) -> Result<()> {
    state.engine.stop(&id).await;
    Ok(())
}

#[tauri::command]
pub async fn uninstall_project(state: State<'_, AppState>, id: String) -> Result<()> {
    state.engine.uninstall(&id).await
}

#[tauri::command]
pub fn project_log(state: State<'_, AppState>, id: String) -> Vec<String> {
    state.engine.recent_log(&id, 500)
}

/// Downloads the project (nothing runs) and returns the safety gate's summary and signals.
#[tauri::command]
pub async fn review_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<SafetyReport> {
    let token = begin_install(&state, &id)?;
    let result = state.engine.review(&id, sink(&app), token).await;
    end_install(&state, &id);
    result
}

#[tauri::command]
pub fn consent_project(
    state: State<'_, AppState>,
    id: String,
    fingerprint: String,
    trust_owner: bool,
) -> Result<()> {
    state.engine.consent(&id, &fingerprint, trust_owner)
}

#[tauri::command]
pub fn env_vars(state: State<'_, AppState>, id: String) -> Result<Vec<EnvVarView>> {
    state.engine.env_vars(&id)
}

// Commands that touch the keychain are async so a system keychain prompt never blocks the
// app's main thread.
#[tauri::command]
pub async fn set_env_value(
    state: State<'_, AppState>,
    id: String,
    name: String,
    value: Option<String>,
) -> Result<()> {
    state.engine.set_env_value(&id, &name, value.as_deref())
}

#[tauri::command]
pub fn mark_keys_reviewed(state: State<'_, AppState>, id: String) -> Result<()> {
    state.engine.mark_keys_reviewed(&id)
}

#[tauri::command]
pub async fn app_settings(state: State<'_, AppState>) -> Result<AppSettings> {
    state.engine.settings()
}

#[tauri::command]
pub fn set_trusted_publisher(
    state: State<'_, AppState>,
    owner: String,
    trusted: bool,
) -> Result<()> {
    state.engine.set_trusted_publisher(&owner, trusted)
}

#[tauri::command]
pub fn set_ai_enabled(state: State<'_, AppState>, enabled: bool) -> Result<()> {
    state.engine.set_ai_enabled(enabled)
}

#[tauri::command]
pub async fn set_ai_key(state: State<'_, AppState>, key: Option<String>) -> Result<()> {
    state.engine.set_ai_key(key.as_deref())
}

#[tauri::command]
pub async fn github_begin_sign_in(state: State<'_, AppState>) -> Result<DeviceCode> {
    state.engine.github_begin_sign_in().await
}

/// Waits until the user approves the code on github.com (or cancels).
#[tauri::command]
pub async fn github_finish_sign_in(state: State<'_, AppState>) -> Result<GitHubAccount> {
    let token = CancellationToken::new();
    *state.sign_in.lock().unwrap_or_else(|e| e.into_inner()) = Some(token.clone());
    let result = state.engine.github_finish_sign_in(token).await;
    *state.sign_in.lock().unwrap_or_else(|e| e.into_inner()) = None;
    result
}

#[tauri::command]
pub fn github_cancel_sign_in(state: State<'_, AppState>) {
    if let Some(t) = state
        .sign_in
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        t.cancel();
    }
}

/// Returns GitHub's page where access can also be revoked.
#[tauri::command]
pub async fn github_sign_out(state: State<'_, AppState>) -> Result<String> {
    state.engine.github_sign_out()
}

#[tauri::command]
pub async fn github_repositories(state: State<'_, AppState>) -> Result<Vec<RemoteRepo>> {
    state.engine.github_repositories().await
}

/// Opens a web page in the user's browser: GitHub's sign-in page, "Where do I get this key?"
/// links. Only https links are opened.
#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> Result<()> {
    if !url.starts_with("https://") {
        return Err(Problem::internal("only https links can be opened"));
    }
    open_in_browser(&app, &url)
}

/// Exactly what asking AI about this project would send.
#[tauri::command]
pub async fn ai_preview(state: State<'_, AppState>, id: String) -> Result<Preview> {
    state.engine.ai_preview(&id)
}

/// Sends the previewed message; `digest` must match what the user saw.
#[tauri::command]
pub async fn ai_suggest(
    state: State<'_, AppState>,
    id: String,
    digest: String,
) -> Result<AiOutcome> {
    state.engine.ai_suggest(&id, &digest).await
}

// --- shortcuts, starting at login, background mode -------------------------------------------

#[tauri::command]
pub fn create_shortcut(state: State<'_, AppState>, id: String) -> Result<Vec<Shortcut>> {
    state.engine.create_shortcuts(&id, &state.launcher)
}

#[tauri::command]
pub fn remove_shortcut(state: State<'_, AppState>, id: String) -> Result<()> {
    state.engine.remove_shortcuts(&id)
}

/// Returns true if Repo Launcher itself was set to start at login so this project can start.
#[tauri::command]
pub fn set_start_at_login(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<bool> {
    state.engine.set_start_at_login(&id, enabled)?;
    let autostart = app.autolaunch();
    if enabled && !autostart.is_enabled().unwrap_or(false) {
        autostart.enable().map_err(login_item_failed)?;
        return Ok(true);
    }
    Ok(false)
}

#[tauri::command]
pub fn set_use_system_terminal(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<()> {
    state.engine.set_use_system_terminal(&id, enabled)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellSettings {
    launch_at_login: bool,
    /// `macos`, `windows` or `linux`, for wording ("menu bar", "Start menu").
    platform: &'static str,
}

#[tauri::command]
pub fn shell_settings(app: AppHandle) -> ShellSettings {
    ShellSettings {
        launch_at_login: app.autolaunch().is_enabled().unwrap_or(false),
        platform: std::env::consts::OS,
    }
}

#[tauri::command]
pub fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<()> {
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable()
    } else {
        autostart.disable()
    }
    .map_err(login_item_failed)
}

fn login_item_failed(e: impl std::fmt::Display) -> Problem {
    Problem::new(ProblemCode::LoginItemFailed).detail(e)
}

/// Messages queued for the main window (see `background::UiMessage`).
#[tauri::command]
pub fn take_messages(state: State<'_, AppState>) -> Vec<background::UiMessage> {
    std::mem::take(&mut *state.messages.lock().unwrap_or_else(|e| e.into_inner()))
}

/// The user confirmed quitting while projects run.
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    background::quit_now(&app);
}
