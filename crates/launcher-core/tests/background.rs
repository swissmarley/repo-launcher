//! Phase 4 engine behavior that needs no downloads: shortcuts follow their project, and
//! projects left running by a Launcher that stopped unexpectedly are found and stopped.

use launcher_core::engine::EngineOptions;
use launcher_core::procinfo;
use launcher_core::secrets::MemoryStore;
use launcher_core::shortcuts::{Launcher, ShortcutDirs};
use launcher_core::store::{RunRecord, Store};
use launcher_core::{AppPaths, Engine};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

fn engine(home: &Path) -> Engine {
    let opts = EngineOptions {
        secrets: Arc::new(MemoryStore::new()),
        shortcut_dirs: ShortcutDirs::at(&home.join("shortcuts")),
        ..EngineOptions::default()
    };
    Engine::with_options(AppPaths::at(home.join("data")), opts).unwrap()
}

fn launcher(exe: &Path) -> Launcher {
    Launcher {
        exe: exe.to_path_buf(),
        icon_png: None,
        icon_icns: None,
    }
}

async fn local_project(engine: &Engine, dir: &Path) -> String {
    std::fs::write(dir.join("main.py"), "print('hi')\n").unwrap();
    engine.add(&dir.to_string_lossy()).await.unwrap().id
}

#[tokio::test]
async fn shortcuts_follow_the_project() {
    let home = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let engine = engine(home.path());
    let id = local_project(&engine, src.path()).await;
    let app = launcher(&home.path().join("bin").join("repo-launcher"));

    let made = engine.create_shortcuts(&id, &app).unwrap();
    assert!(!made.is_empty());
    for s in &made {
        assert!(Path::new(&s.path).exists(), "{}", s.path);
    }
    let mut recorded = engine.shortcuts(&id).unwrap();
    recorded.sort_by(|a, b| a.path.cmp(&b.path));
    let mut expected = made.clone();
    expected.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(recorded, expected);

    // Creating again rewrites the same files instead of adding numbered copies.
    let again = engine.create_shortcuts(&id, &app).unwrap();
    assert_eq!(again, made);

    // The app moved: shortcuts that start it by path are rewritten. macOS shortcuts open a
    // link instead, which keeps working wherever the app is.
    let moved = launcher(&home.path().join("elsewhere").join("repo-launcher"));
    let fixed = engine.repair_shortcuts(&moved).unwrap();
    assert_eq!(
        fixed,
        if cfg!(target_os = "macos") {
            0
        } else {
            made.len()
        }
    );
    assert_eq!(engine.repair_shortcuts(&moved).unwrap(), 0);

    // A shortcut the user deleted is forgotten, not brought back.
    launcher_core::shortcuts::remove(Path::new(&made[0].path)).unwrap();
    engine.repair_shortcuts(&moved).unwrap();
    assert!(!Path::new(&made[0].path).exists());
    assert_eq!(engine.shortcuts(&id).unwrap().len(), made.len() - 1);

    // Removing the project removes its shortcuts.
    engine.create_shortcuts(&id, &moved).unwrap();
    engine.uninstall(&id).await.unwrap();
    for s in &made {
        assert!(!Path::new(&s.path).exists(), "{} left behind", s.path);
    }
}

/// A program that keeps running for a while, in its own process group like a project's.
fn long_running() -> std::process::Child {
    let mut cmd = if cfg!(windows) {
        let mut c = std::process::Command::new("cmd.exe");
        c.args(["/c", "ping -n 60 127.0.0.1 >nul"]);
        c
    } else {
        let mut c = std::process::Command::new("/bin/sh");
        c.args(["-c", "sleep 60 & sleep 60"]);
        c
    };
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    cmd.spawn().unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn projects_left_running_by_a_crash_are_stopped() {
    let home = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let first = engine(home.path());
    let id = local_project(&first, src.path()).await;
    drop(first);

    // What the previous Launcher recorded before it stopped unexpectedly: a project still
    // running, and a run whose pid now belongs to some other program (a different start time).
    let mut child = long_running();
    let pid = child.id();
    std::thread::sleep(Duration::from_millis(200));
    let store = Store::open(&home.path().join("data").join("state.db")).unwrap();
    let record = |start: Option<i64>| RunRecord {
        id: 0,
        project_id: id.clone(),
        pid: Some(pid),
        pgid: cfg!(unix).then_some(pid),
        proc_start_ms: start,
        boot_ms: procinfo::boot_time_ms(),
        url: Some("http://127.0.0.1:9/".into()),
    };
    store.begin_run(&record(Some(1_000))).unwrap();
    drop(store);

    // The reused pid is left alone.
    let engine = engine(home.path());
    assert!(engine.reconcile().await.unwrap().is_empty());
    assert!(
        procinfo::start_time_ms(pid).is_some(),
        "not ours: still running"
    );

    // The real one is stopped, once.
    let store = Store::open(&home.path().join("data").join("state.db")).unwrap();
    store
        .begin_run(&record(procinfo::start_time_ms(pid)))
        .unwrap();
    drop(store);
    assert_eq!(engine.reconcile().await.unwrap(), vec![id.clone()]);
    let _ = child.wait();
    assert!(
        procinfo::start_time_ms(pid).is_none(),
        "the tree was stopped"
    );
    assert!(!engine.is_running(&id));
    assert_eq!(
        engine.latest_run(&id).unwrap().map(|(_, open)| open),
        Some(false)
    );
    assert!(engine.reconcile().await.unwrap().is_empty());
}
