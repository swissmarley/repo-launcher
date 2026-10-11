//! End-to-end: a Go module built with the managed toolchain. Ignored by default:
//! `cargo test -p launcher-core --test go_projects -- --ignored`

use launcher_core::events::null_sink;
use launcher_core::{AppPaths, Engine, ProjectStatus};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "downloads the Go toolchain"]
async fn go_cli_builds_and_runs() {
    let root: PathBuf = std::env::var_os("RL_TEST_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("rl-integration"));
    let engine = Engine::open(AppPaths::at(root)).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic/go-hello");
    let project = engine
        .add(&std::fs::canonicalize(fixture).unwrap().to_string_lossy())
        .await
        .unwrap();
    let installed = engine
        .install(&project.id, null_sink(), CancellationToken::new())
        .await
        .unwrap_or_else(|p| panic!("install failed: {p}"));
    assert_eq!(installed.status, ProjectStatus::Installed);
    engine.launch(&project.id, null_sink()).await.unwrap();
    for _ in 0..100 {
        if !engine.is_running(&project.id) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let out = engine.run_history(&project.id).join("\n");
    assert!(
        out.contains("hello from go-hello built with go1.27"),
        "{out}"
    );
    engine.uninstall(&project.id).await.unwrap();
}
