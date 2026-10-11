//! Removing Repo Launcher (DECISIONS D-041): what deleting the app's files can't do by itself.
//!
//! `repo-launcher --uninstall [--delete-data] [--after <pid>]` runs headless, without Tauri. It
//! removes every shortcut the Launcher made, every keychain entry it created (the same engine
//! code as `rl remove-shortcuts` and `rl forget-secrets`) and its start-at-login entry, and with
//! `--delete-data` also its projects, runtimes, logs and WebView data. The Windows uninstallers
//! run it before they delete the program; "Remove all Repo Launcher data" in Settings starts it
//! with `--after` and quits, so the files are no longer in use when it deletes them.

use launcher_core::{AppPaths, Engine, paths};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The bundle identifier, which names Tauri's own folders (WebView data, logs).
const BUNDLE_ID: &str = "app.repolauncher.desktop";
/// The start-at-login entry is named after the product (tauri-plugin-autostart's default).
const PRODUCT_NAME: &str = "Repo Launcher";

/// Runs `--uninstall` if that's what this process was started for; returns its exit code.
pub fn run_if_requested() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.iter().any(|a| a == "--uninstall") {
        return None;
    }
    let delete_data = args.iter().any(|a| a == "--delete-data");
    let after = args
        .iter()
        .position(|a| a == "--after")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse::<u32>().ok());
    if let Some(pid) = after {
        wait_for_exit(pid, Duration::from_secs(60));
    }
    let report = clean_up(delete_data);
    for problem in &report {
        eprintln!("{problem}");
    }
    Some(if report.is_empty() { 0 } else { 1 })
}

/// Waits until the app that started us has quit, so its files are no longer open.
fn wait_for_exit(pid: u32, limit: Duration) {
    let started = Instant::now();
    while launcher_core::procinfo::start_time_ms(pid).is_some() && started.elapsed() < limit {
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Does everything it can and returns what failed (in English: it's for logs, not people).
pub fn clean_up(delete_data: bool) -> Vec<String> {
    let mut failed = Vec::new();
    if let Some(app) = AppPaths::for_current_user() {
        // Opening the engine creates its database; with nothing installed there's nothing to do.
        if app.db().is_file() {
            match Engine::open(app.clone()) {
                Ok(engine) => {
                    if let Err(p) = engine.remove_all_shortcuts() {
                        failed.push(format!("removing shortcuts: {p}"));
                    }
                    if let Err(p) = engine.forget_all_secrets() {
                        failed.push(format!("removing keychain entries: {p}"));
                    }
                }
                Err(p) => failed.push(format!("opening the library: {p}")),
            }
        }
        if delete_data && let Err(e) = paths::remove_tree(&app.root) {
            failed.push(format!("deleting {}: {e}", app.root.display()));
        }
    }
    if let Err(e) = remove_login_item() {
        failed.push(format!("removing the start-at-login entry: {e}"));
    }
    if delete_data {
        for dir in shell_folders() {
            if let Err(e) = paths::remove_tree(&dir) {
                failed.push(format!("deleting {}: {e}", dir.display()));
            }
        }
    }
    failed
}

/// The same entry tauri-plugin-autostart creates: a Launch Agent on macOS, a `Run` value on
/// Windows, an autostart `.desktop` file on Linux. Only its name matters for removing it.
fn remove_login_item() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut builder = auto_launch::AutoLaunchBuilder::new();
    builder
        .set_app_name(PRODUCT_NAME)
        .set_app_path(&exe.to_string_lossy())
        .set_args(&["--background"]);
    #[cfg(target_os = "macos")]
    builder.set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent);
    let entry = builder.build().map_err(|e| e.to_string())?;
    if entry.is_enabled().unwrap_or(true) {
        entry.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Folders Tauri and the WebView keep for the app, outside the engine's own folder.
fn shell_folders() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = [
        dirs::data_dir(),
        dirs::data_local_dir(),
        dirs::cache_dir(),
        dirs::config_dir(),
    ]
    .into_iter()
    .flatten()
    .map(|d| d.join(BUNDLE_ID))
    .collect();
    #[cfg(target_os = "macos")]
    if let Some(home) = dirs::home_dir() {
        let lib = home.join("Library");
        dirs.push(lib.join("Logs").join(BUNDLE_ID));
        dirs.push(lib.join("WebKit").join(BUNDLE_ID));
        dirs.push(lib.join("HTTPStorages").join(BUNDLE_ID));
        dirs.push(
            lib.join("Saved Application State")
                .join(format!("{BUNDLE_ID}.savedState")),
        );
    }
    // The link handler development builds and AppImages register for themselves.
    #[cfg(target_os = "linux")]
    if let (Some(data), Ok(exe)) = (dirs::data_dir(), std::env::current_exe())
        && let Some(name) = exe.file_name()
    {
        let handler = format!("{}-handler.desktop", name.to_string_lossy());
        dirs.push(data.join("applications").join(handler));
    }
    dirs.sort();
    dirs.dedup();
    dirs
}
