//! Installs and launches real public repos from the corpus, exactly as a user would: by link,
//! anonymously. Slow and network-heavy, so ignored by default and run on demand:
//! `cargo test -p launcher-core --test real_repos -- --ignored --test-threads=1`

use launcher_core::events::null_sink;
use launcher_core::{AppPaths, Engine, ProjectStatus};
use std::path::PathBuf;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

fn engine() -> Engine {
    let root: PathBuf = std::env::var_os("RL_TEST_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("rl-real-repos"));
    Engine::open(AppPaths::at(root)).unwrap()
}

/// Adds by link, installs, and returns the project id.
async fn install(engine: &Engine, link: &str) -> String {
    let project = engine
        .add(link)
        .await
        .unwrap_or_else(|p| panic!("add {link}: {p}"));
    let installed = engine
        .install(&project.id, null_sink(), CancellationToken::new())
        .await
        .unwrap_or_else(|p| panic!("install {link}: {p}"));
    assert_eq!(installed.status, ProjectStatus::Installed);
    project.id
}

async fn web_app_answers(link: &str, expect_in_page: &str) {
    let engine = engine();
    let id = install(&engine, link).await;
    let info = engine
        .launch(&id, null_sink())
        .await
        .unwrap_or_else(|p| panic!("launch {link}: {p}"));
    let url = info.url.expect("web app has an address");
    let body = reqwest::get(&url).await.unwrap().text().await.unwrap();
    assert!(
        body.contains(expect_in_page),
        "{link}: page at {url} lacks {expect_in_page:?}"
    );
    engine.stop(&id).await;
    engine.uninstall(&id).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "installs real projects from GitHub"]
async fn streamlit_app() {
    web_app_answers("https://github.com/streamlit/demo-uber-nyc-pickups", "<").await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "installs real projects from GitHub"]
async fn gradio_demo_in_a_subfolder() {
    web_app_answers(
        "https://github.com/gradio-app/gradio/tree/main/demo/hello_world",
        "gradio",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "installs real projects from GitHub"]
async fn vite_react_template() {
    web_app_answers(
        "https://github.com/vitejs/vite/tree/main/packages/create-vite/template-react",
        "<div id=\"root\">",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "installs real projects from GitHub"]
async fn astro_site() {
    web_app_answers(
        "https://github.com/withastro/astro/tree/main/examples/basics",
        "Astro",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "installs real projects from GitHub"]
async fn node_cli() {
    let engine = engine();
    let id = install(&engine, "https://github.com/piuccio/cowsay").await;
    engine.launch(&id, null_sink()).await.unwrap();
    for _ in 0..100 {
        if !engine.is_running(&id) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        !engine.console_scrollback(&id).is_empty(),
        "printed nothing"
    );
    engine.uninstall(&id).await.unwrap();
}
