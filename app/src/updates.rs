//! Updating Repo Launcher itself (DECISIONS D-040): Tauri's updater, which installs only an
//! update signed with the release key whose public half is compiled into the app. Builds made
//! without the release configuration (development builds, anyone's own build) have no key, and
//! say that updates aren't set up instead of checking.
//!
//! By default the app checks at most once a day, a little after starting; the user can turn
//! that off. A check asks GitHub for the latest release's `latest.json` and sends nothing but
//! what any download sends (address, app version in the user agent).

use crate::{AppState, background};
use launcher_core::{Problem, ProblemCode};
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

const AUTO_CHECK: &str = "updates.auto_check";
const LAST_CHECK: &str = "updates.last_check_ms";
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Whether this build was made with an update key (the release configuration).
pub fn configured(config: &tauri::Config) -> bool {
    config
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.trim().is_empty())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatesInfo {
    configured: bool,
    auto_check: bool,
    current_version: String,
    /// A newer version found by the last check, ready to install.
    available: Option<String>,
}

pub fn info(app: &AppHandle) -> UpdatesInfo {
    let state = app.state::<AppState>();
    UpdatesInfo {
        configured: state.updates_configured,
        auto_check: auto_check(&state),
        current_version: app.package_info().version.to_string(),
        available: state
            .update
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|u| u.version.clone()),
    }
}

fn auto_check(state: &AppState) -> bool {
    state.engine.shell_setting(AUTO_CHECK).as_deref() != Some("0")
}

pub fn set_auto_check(app: &AppHandle, enabled: bool) -> Result<(), Problem> {
    let state = app.state::<AppState>();
    state
        .engine
        .set_shell_setting(AUTO_CHECK, Some(if enabled { "1" } else { "0" }))
}

fn failed(e: impl std::fmt::Display) -> Problem {
    Problem::new(ProblemCode::UpdateFailed)
        .fix(launcher_core::FixAction::Retry)
        .detail(e)
}

/// Asks for the latest release; returns its version if it is newer than this one.
pub async fn check(app: &AppHandle) -> Result<Option<String>, Problem> {
    let state = app.state::<AppState>();
    if !state.updates_configured {
        return Ok(None);
    }
    let update = app
        .updater()
        .map_err(failed)?
        .check()
        .await
        .map_err(failed)?;
    let _ = state.engine.set_shell_setting(
        LAST_CHECK,
        Some(&launcher_core::procinfo::now_ms().to_string()),
    );
    let version = update.as_ref().map(|u| u.version.clone());
    *state.update.lock().unwrap_or_else(|e| e.into_inner()) = update;
    Ok(version)
}

/// Downloads and installs the update found by `check`, then restarts into it. Projects stop
/// first, as with quitting (the UI asks before calling this if any are running).
pub async fn install(app: &AppHandle) -> Result<(), Problem> {
    let state = app.state::<AppState>();
    // Kept until it's installed, so "Try again" works after a failed download.
    let update = state
        .update
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .ok_or_else(|| failed("no update was found to install"))?;
    let mut received = 0usize;
    let progress = app.clone();
    let bytes = update
        .download(
            move |chunk, total| {
                received += chunk;
                if let Some(total) = total.filter(|t| *t > 0) {
                    let fraction = (received as f64 / total as f64).min(1.0);
                    let _ = progress.emit("update-progress", fraction);
                }
            },
            || {},
        )
        .await
        .map_err(failed)?;
    state.engine.stop_all().await;
    update.install(bytes).map_err(failed)?;
    state
        .quitting
        .store(true, std::sync::atomic::Ordering::Relaxed);
    app.restart();
}

/// The once-a-day check a little after startup. Tells the main window when there's news.
pub fn schedule_check(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.updates_configured || !auto_check(&state) {
        return;
    }
    let last = state
        .engine
        .shell_setting(LAST_CHECK)
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    if launcher_core::procinfo::now_ms() - last < DAY_MS {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(20)).await;
        match check(&app).await {
            Ok(Some(version)) => {
                background::post(&app, background::UiMessage::UpdateAvailable { version })
            }
            Ok(None) => {}
            Err(p) => log::info!("update check: {p}"),
        }
    });
}
