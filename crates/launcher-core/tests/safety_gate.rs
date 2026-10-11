//! The safety gate's consent rules, offline: nothing here installs or runs a project.

use launcher_core::engine::EngineOptions;
use launcher_core::events::null_sink;
use launcher_core::secrets::MemoryStore;
use launcher_core::{AppPaths, Engine, ProblemCode};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

fn engine(home: &std::path::Path) -> Engine {
    let opts = EngineOptions {
        secrets: Arc::new(MemoryStore::new()),
        ..EngineOptions::default()
    };
    Engine::with_options(AppPaths::at(home), opts).unwrap()
}

#[tokio::test]
async fn files_created_after_download_dont_change_the_plan() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("main.py"),
        "if __name__ == '__main__':\n    print('hi')\n",
    )
    .unwrap();
    let home = tempfile::tempdir().unwrap();
    let engine = engine(home.path());
    let p = engine.add(&project.path().to_string_lossy()).await.unwrap();

    let first = engine
        .review(&p.id, null_sink(), CancellationToken::new())
        .await
        .unwrap();
    assert!(!first.consented);
    engine.consent(&p.id, &first.fingerprint, false).unwrap();

    // Something the install created inside the project folder (like npm's lockfile) would
    // change detection; it isn't part of the download, so the plan stays the same.
    let src = home.path().join("p").join(&p.id).join("src");
    std::fs::write(src.join("requirements.txt"), "flask\n").unwrap();
    let again = engine
        .review(&p.id, null_sink(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(again.fingerprint, first.fingerprint);
    assert!(again.consented);

    // A change to the project's own files does change it, and consent is asked again.
    std::fs::write(project.path().join("requirements.txt"), "flask\n").unwrap();
    let changed = engine
        .review(&p.id, null_sink(), CancellationToken::new())
        .await
        .unwrap();
    assert_ne!(changed.fingerprint, first.fingerprint);
    assert!(!changed.consented);
    assert_eq!(
        engine
            .consent(&p.id, &first.fingerprint, false)
            .unwrap_err()
            .code,
        ProblemCode::ConsentRequired,
        "consent to an outdated plan is refused"
    );
}

#[tokio::test]
async fn trusting_an_owner_is_remembered() {
    let home = tempfile::tempdir().unwrap();
    let engine = engine(home.path());
    engine.set_trusted_publisher("Streamlit", true).unwrap();
    assert_eq!(
        engine.settings().unwrap().trusted_publishers,
        vec!["streamlit".to_owned()]
    );
    engine.set_trusted_publisher("streamlit", false).unwrap();
    assert!(engine.settings().unwrap().trusted_publishers.is_empty());
}
