//! Thin adapters between the WebView and the engine. No business logic lives here.

use crate::AppState;
use launcher_core::run::LaunchInfo;
use launcher_core::{EngineEvent, EventSink, Problem, ProblemCode, Project};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
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
}

fn view(state: &AppState, project: Project) -> ProjectView {
    let running = state.engine.is_running(&project.id);
    let url = state.engine.running_url(&project.id);
    ProjectView {
        project,
        running,
        url,
    }
}

fn sink(app: &AppHandle) -> EventSink {
    let app = app.clone();
    Arc::new(move |event: EngineEvent| {
        let _ = app.emit("engine", event);
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
    let token = CancellationToken::new();
    {
        let mut installs = state.installs.lock().unwrap_or_else(|e| e.into_inner());
        if installs.contains_key(&id) {
            return Err(Problem::new(ProblemCode::Internal).detail("already installing"));
        }
        installs.insert(id.clone(), token.clone());
    }
    let result = state.engine.install(&id, sink(&app), token).await;
    state
        .installs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&id);
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

#[tauri::command]
pub async fn launch_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<LaunchInfo> {
    let info = state.engine.launch(&id, sink(&app)).await?;
    if let Some(url) = &info.url {
        open_in_browser(&app, url)?;
    }
    Ok(info)
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
