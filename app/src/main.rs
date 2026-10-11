// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // "Open in my terminal": a headless run in the system's terminal, no window.
    if let Some(code) = repo_launcher_lib::exec_if_requested() {
        std::process::exit(code);
    }
    // Uninstallers and "Remove all Repo Launcher data": headless cleanup, no window.
    if let Some(code) = repo_launcher_lib::uninstall_if_requested() {
        std::process::exit(code);
    }
    repo_launcher_lib::run();
}
