//! End-to-end: install and launch real fixture projects with managed uv + Python.
//! These download uv, Python and packages, so they're ignored by default:
//! `cargo test -p launcher-core --test python_projects -- --ignored`

use launcher_core::events::null_sink;
use launcher_core::{AppPaths, Engine, ProjectStatus};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

fn fixture(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic")
        .join(name);
    std::fs::canonicalize(p)
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

/// Shares one data folder per test binary run so uv and Python download once.
fn engine() -> Engine {
    let root: PathBuf = std::env::var_os("RL_TEST_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("rl-integration"));
    Engine::open(AppPaths::at(root)).unwrap()
}

async fn install(engine: &Engine, name: &str) -> String {
    let project = engine.add(&fixture(name)).await.unwrap();
    // The safety gate: review what will happen, then agree to it.
    let report = engine
        .review(&project.id, null_sink(), CancellationToken::new())
        .await
        .unwrap_or_else(|p| panic!("review failed: {p}"));
    engine
        .consent(&project.id, &report.fingerprint, false)
        .unwrap();
    let installed = engine
        .install(&project.id, null_sink(), CancellationToken::new())
        .await
        .unwrap_or_else(|p| panic!("install {name} failed: {p}"));
    assert_eq!(installed.status, ProjectStatus::Installed);
    project.id
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads runtimes and packages"]
async fn old_pinned_packages_install_on_an_older_python() {
    // numpy 1.21 can't be built on Python 3.12 (no distutils) and has no ready-made builds for
    // 3.11 or 3.12; 3.10 has them.
    let engine = engine();
    let id = install(&engine, "py-old-pins").await;
    engine.launch(&id, null_sink()).await.unwrap();
    for _ in 0..100 {
        if !engine.is_running(&id) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let out = engine.run_history(&id).join("\n");
    assert!(out.contains("numpy 1.21.6 on Python 3.1"), "{out}");
    assert!(!out.contains("on Python 3.12"), "{out}");
    engine.uninstall(&id).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads runtimes and packages"]
async fn web_project_starts_responds_and_stops() {
    let engine = engine();
    let id = install(&engine, "py-flask").await;
    let info = engine.launch(&id, null_sink()).await.unwrap();
    let url = info.url.expect("web project has a URL");
    let body = reqwest::get(&url).await.unwrap().text().await.unwrap();
    assert!(body.contains("Hello from py-flask"), "{body}");

    engine.stop(&id).await;
    assert!(!engine.is_running(&id));
    assert!(
        reqwest::get(&url).await.is_err(),
        "server still answering after stop"
    );
    engine.uninstall(&id).await.unwrap();
    assert!(
        !engine.paths().project(&id).root.exists(),
        "uninstall left files behind"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads runtimes and packages"]
async fn stop_kills_grandchildren() {
    let engine = engine();
    let id = install(&engine, "py-tree").await;
    engine.launch(&id, null_sink()).await.unwrap();

    let pid_file = engine.paths().project(&id).src().join("grandchild.pid");
    let mut pid = None;
    for _ in 0..100 {
        if let Ok(s) = std::fs::read_to_string(&pid_file)
            && let Ok(p) = s.trim().parse::<u32>()
        {
            pid = Some(p);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let pid = pid.expect("grandchild never started");
    assert!(alive(pid), "grandchild should be running before stop");

    engine.stop(&id).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!alive(pid), "grandchild {pid} survived Stop");
    engine.uninstall(&id).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads runtimes and packages"]
async fn terminal_project_runs_to_completion() {
    let engine = engine();
    let id = install(&engine, "py-cli").await;
    engine.launch(&id, null_sink()).await.unwrap();
    for _ in 0..100 {
        if !engine.is_running(&id) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!engine.is_running(&id));
    let out = engine.run_history(&id).join("\n");
    assert!(out.contains("hello from py-cli on Python 3.12"), "{out}");
    engine.uninstall(&id).await.unwrap();
}

fn alive(pid: u32) -> bool {
    if cfg!(windows) {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    } else {
        // `ps -p` exits 0 only if the process exists (zombies count; ours are reaped by init).
        std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "stat="])
            .output()
            .map(|o| {
                o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().starts_with('Z')
            })
            .unwrap_or(false)
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads runtimes and packages"]
async fn console_is_interactive() {
    let engine = engine();
    let id = install(&engine, "py-prompt").await;
    engine.launch(&id, null_sink()).await.unwrap();
    let mut asked = false;
    for _ in 0..100 {
        if engine
            .console_scrollback(&id)
            .contains("What is your name?")
        {
            asked = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        asked,
        "prompt never appeared: {:?}",
        engine.console_scrollback(&id)
    );
    engine.console_write(&id, "Ada\r").unwrap();
    for _ in 0..100 {
        if !engine.is_running(&id) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let out = engine.run_history(&id).join("\n");
    assert!(out.contains("Hello, Ada!"), "{out}");
    engine.uninstall(&id).await.unwrap();
}
