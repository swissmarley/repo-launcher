//! End-to-end: Node projects with managed Node + package managers, port discovery, and
//! launcher.json. Ignored by default (downloads runtimes and packages):
//! `cargo test -p launcher-core --test node_projects -- --ignored --test-threads=1`

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

async fn launch_and_fetch(engine: &Engine, id: &str) -> (String, String) {
    let info = engine
        .launch(id, null_sink())
        .await
        .unwrap_or_else(|p| panic!("launch failed: {p}"));
    let url = info.url.expect("web project has a URL");
    let body = reqwest::get(&url).await.unwrap().text().await.unwrap();
    (url, body)
}

async fn finish(engine: &Engine, id: &str, url: &str) {
    engine.stop(id).await;
    assert!(
        reqwest::get(url).await.is_err(),
        "still answering after stop"
    );
    engine.uninstall(id).await.unwrap();
    assert!(!engine.paths().project(id).root.exists());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads runtimes and packages"]
async fn vite_app_with_pnpm() {
    let engine = engine();
    let id = install(&engine, "node-vite").await;
    let (url, body) = launch_and_fetch(&engine, &id).await;
    assert!(body.contains("Hello from node-vite"), "{body}");
    finish(&engine, &id, &url).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads runtimes and packages"]
async fn hardcoded_port_is_discovered_from_output() {
    let engine = engine();
    let id = install(&engine, "node-fixed-port").await;
    let (url, body) = launch_and_fetch(&engine, &id).await;
    assert!(url.contains(":38417"), "{url}");
    assert!(body.contains("Hello from node-fixed-port"), "{body}");
    finish(&engine, &id, &url).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads runtimes and packages"]
async fn node_cli_runs() {
    let engine = engine();
    let id = install(&engine, "node-cli").await;
    engine.launch(&id, null_sink()).await.unwrap();
    for _ in 0..100 {
        if !engine.is_running(&id) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let out = engine.run_history(&id).join("\n");
    assert!(out.contains("hello from node-cli on Node 24."), "{out}");
    engine.uninstall(&id).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads runtimes and packages"]
async fn launcher_json_project() {
    let engine = engine();
    let id = install(&engine, "manifest-web").await;
    assert_eq!(engine.get(&id).unwrap().name, "Manifest Fixture");
    let (url, body) = launch_and_fetch(&engine, &id).await;
    assert!(body.contains("Hello from manifest-web"), "{body}");
    finish(&engine, &id, &url).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads runtimes and packages"]
async fn second_install_resumes_instead_of_redoing() {
    let engine = engine();
    let id = install(&engine, "node-cli").await;
    // Installing again: steps already done for this source are skipped (local sources get a
    // fresh id, so here we only check it still succeeds quickly and leaves things installed).
    let started = std::time::Instant::now();
    engine
        .install(&id, null_sink(), CancellationToken::new())
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(60));
    engine.uninstall(&id).await.unwrap();
}
