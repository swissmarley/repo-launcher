//! Credentials end to end, offline: GitHub sign-in and private repositories against a mock
//! GitHub, and keys that reach the project but are never written to disk.
// The launch test runs a shell script, so some imports are used on Unix only.
#![cfg_attr(not(unix), allow(unused_imports))]

use launcher_core::engine::EngineOptions;
use launcher_core::events::{EngineEvent, EventSink, RunState, null_sink};
use launcher_core::secrets::MemoryStore;
use launcher_core::{AppPaths, Engine, FixAction, ProblemCode, ProjectStatus};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const TOKEN: &str = "ghu_mocktokenmocktokenmocktoken0123";
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn engine_with(root: &Path, server: Option<&MockServer>, store: Arc<MemoryStore>) -> Engine {
    let mut opts = EngineOptions {
        secrets: store,
        ..EngineOptions::default()
    };
    if let Some(s) = server {
        opts.github_api = s.uri();
        opts.github_web = s.uri();
    }
    Engine::with_options(AppPaths::at(root), opts).unwrap()
}

async fn review_and_consent(engine: &Engine, id: &str) {
    let report = engine
        .review(id, null_sink(), CancellationToken::new())
        .await
        .unwrap_or_else(|p| panic!("review: {p}"));
    engine.consent(id, &report.fingerprint, false).unwrap();
}

fn pkt(line: &str) -> String {
    format!("{:04x}{}", line.len() + 4, line)
}

fn ref_advertisement() -> String {
    let mut s = pkt("# service=git-upload-pack\n");
    s.push_str("0000");
    s.push_str(&pkt(&format!(
        "{SHA} HEAD\0multi_ack symref=HEAD:refs/heads/main\n"
    )));
    s.push_str(&pkt(&format!("{SHA} refs/heads/main\n")));
    s.push_str("0000");
    s
}

/// A tiny project as a GitHub-style tarball (one top-level folder).
fn tarball(top: &str) -> Vec<u8> {
    let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::fast(),
    ));
    let files = [
        (
            "launcher.json",
            r#"{"version":1,"type":"terminal","run":{"command":["python","main.py"]}}"#,
        ),
        ("main.py", "print('hello')\n"),
    ];
    for (name, body) in files {
        let mut h = tar::Header::new_gnu();
        h.set_size(body.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        builder
            .append_data(&mut h, format!("{top}/{name}"), body.as_bytes())
            .unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

fn repo_json(name: &str, private: bool) -> serde_json::Value {
    serde_json::json!({
        "name": name, "full_name": format!("acme/{name}"), "description": "demo",
        "stargazers_count": 120, "default_branch": "main", "private": private,
        "archived": false, "license": {"spdx_id": "MIT"},
        "created_at": "2020-01-01T00:00:00Z", "pushed_at": "2026-01-01T00:00:00Z",
        "owner": {"avatar_url": null}
    })
}

async fn mock_github(server: &MockServer) {
    let basic = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("x-access-token:{TOKEN}")
        )
    );
    let bearer = format!("Bearer {TOKEN}");

    // Public repository: answers anyone.
    Mock::given(method("GET"))
        .and(path("/repos/acme/open"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repo_json("open", false)))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/acme/open.git/info/refs"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ref_advertisement()))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/acme/open/archive/{SHA}.tar.gz")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(tarball("open-0123456")))
        .mount(server)
        .await;

    // Private repository: only with the token; 404 otherwise (as GitHub does).
    Mock::given(method("GET"))
        .and(path("/repos/acme/secret"))
        .and(header("authorization", bearer.as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(repo_json("secret", true)))
        .with_priority(1)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/acme/secret.git/info/refs"))
        .and(header("authorization", basic.as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_string(ref_advertisement()))
        .with_priority(1)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/acme/secret.git/info/refs"))
        .respond_with(ResponseTemplate::new(401))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/repos/acme/secret/tarball/{SHA}")))
        .and(header("authorization", bearer.as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(tarball("acme-secret-0123456")))
        .mount(server)
        .await;

    // Sign-in (device flow), the account, and the shared repositories.
    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "devicecode0123456789", "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device", "expires_in": 900, "interval": 1
        })))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": TOKEN, "expires_in": 28800,
            "refresh_token": "ghr_refreshrefreshrefreshrefresh0123", "refresh_token_expires_in": 15897600
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .and(header("authorization", bearer.as_str()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"login": "octo", "avatar_url": null})),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user/installations"))
        .and(header_exists("authorization"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"installations": [{"id": 7}]})),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user/installations/7/repositories"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "repositories": [{
                "full_name": "acme/secret", "description": "demo", "private": true,
                "stargazers_count": 0, "html_url": "https://github.com/acme/secret",
                "pushed_at": "2026-01-01T00:00:00Z"
            }]
        })))
        .mount(server)
        .await;
    // Anything else (e.g. a repo that doesn't exist): 404.
}

#[tokio::test]
async fn a_subfolder_project_is_named_after_its_folder() {
    let server = MockServer::start().await;
    mock_github(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let engine = engine_with(dir.path(), Some(&server), Arc::new(MemoryStore::new()));

    let whole = engine.add("https://github.com/acme/open").await.unwrap();
    assert_eq!(whole.name, "open");
    let demo = engine
        .add("https://github.com/acme/open/tree/main/demos/hello_world")
        .await
        .unwrap();
    assert_eq!(demo.name, "hello_world (open)");
    assert_eq!(demo.subdir.as_deref(), Some("demos/hello_world"));
    assert_ne!(demo.id, whole.id);
}

fn carries_token(req: &Request) -> bool {
    req.headers.contains_key("authorization")
}

#[tokio::test]
async fn private_repos_need_sign_in_and_public_ones_never_send_the_token() {
    let server = MockServer::start().await;
    mock_github(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::new());
    let engine = engine_with(dir.path(), Some(&server), store.clone());
    let link = |repo: &str| format!("https://github.com/acme/{repo}");

    // Signed out: a private repo looks like a missing one, with "Connect GitHub" as the fix.
    let err = engine.add(&link("secret")).await.unwrap_err();
    assert_eq!(err.code, ProblemCode::RepoNotFoundOrPrivate);
    assert_eq!(err.fix, Some(FixAction::ConnectGithub));

    // Without a configured GitHub App, signing in explains itself.
    assert_eq!(
        engine.github_begin_sign_in().await.unwrap_err().code,
        ProblemCode::GithubNotConfigured
    );
    engine
        .set_github_app(Some("Iv23mock"), Some("repo-launcher"))
        .unwrap();

    let code = engine.github_begin_sign_in().await.unwrap();
    assert_eq!(code.user_code, "ABCD-1234");
    let account = engine
        .github_finish_sign_in(CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(account.login, "octo");
    assert_eq!(engine.github_account().unwrap().login, "octo");
    assert_eq!(store.keys(), vec!["github.token".to_owned()]);

    // "Your projects".
    let repos = engine.github_repositories().await.unwrap();
    assert_eq!(repos[0].full_name, "acme/secret");

    // Signed in: the private repo installs through the API with the token.
    let private = engine.add(&link("secret")).await.unwrap();
    assert!(private.is_private);
    review_and_consent(&engine, &private.id).await;
    let installed = engine
        .install(&private.id, null_sink(), CancellationToken::new())
        .await
        .unwrap_or_else(|p| panic!("install private: {p}"));
    assert_eq!(installed.status, ProjectStatus::Installed);
    assert!(
        dir.path()
            .join("p")
            .join(&private.id)
            .join("src/main.py")
            .is_file()
    );

    // A repo GitHub still can't find: not shared with the app (or missing).
    let err = engine.add(&link("elsewhere")).await.unwrap_err();
    assert_eq!(err.code, ProblemCode::RepoNotShared);
    assert!(
        matches!(err.fix, Some(FixAction::ChooseRepositories { ref url }) if url.contains("/apps/repo-launcher/"))
    );

    // A public repo, while signed in, never carries the token.
    let before = server.received_requests().await.unwrap().len();
    let public = engine.add(&link("open")).await.unwrap();
    assert!(!public.is_private);
    review_and_consent(&engine, &public.id).await;
    assert_eq!(engine.check_update(&public.id).await.unwrap(), None);
    let after = server.received_requests().await.unwrap();
    let public_requests: Vec<&Request> = after[before..].iter().collect();
    assert!(!public_requests.is_empty());
    for r in public_requests {
        assert!(!carries_token(r), "token sent to {}", r.url);
    }

    // Signing out removes the token from the keychain.
    engine.github_sign_out().unwrap();
    assert!(engine.github_account().is_none());
    assert!(store.keys().is_empty());
    // And uninstalling leaves nothing behind.
    engine.uninstall(&private.id).await.unwrap();
}

/// Every byte under `root`, including the SQLite database and its journal.
fn files_containing(root: &Path, needle: &[u8]) -> Vec<String> {
    let mut hits = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(bytes) = std::fs::read(&p)
                && bytes.windows(needle.len()).any(|w| w == needle)
            {
                hits.push(p.display().to_string());
            }
        }
    }
    hits
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn keys_reach_the_project_but_never_the_disk() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("launcher.json"),
        r#"{"version":1,"type":"terminal","run":{"command":["sh","show.sh"]},
            "env":[{"name":"DEMO_API_KEY","required":true},
                   {"name":"DEMO_MODE","secret":false}]}"#,
    )
    .unwrap();
    std::fs::write(
        project.path().join("show.sh"),
        "echo \"key=$DEMO_API_KEY len=${#DEMO_API_KEY} mode=$DEMO_MODE\"\n",
    )
    .unwrap();

    let home = tempfile::tempdir().unwrap();
    let store = Arc::new(MemoryStore::new());
    let engine = engine_with(home.path(), None, store.clone());
    let p = engine.add(&project.path().to_string_lossy()).await.unwrap();

    // Installing needs consent first.
    let err = engine
        .install(&p.id, null_sink(), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(err.code, ProblemCode::ConsentRequired);
    review_and_consent(&engine, &p.id).await;
    engine
        .install(&p.id, null_sink(), CancellationToken::new())
        .await
        .unwrap();

    // A required key is missing: launching explains which.
    let err = engine.launch(&p.id, null_sink()).await.unwrap_err();
    assert_eq!(err.code, ProblemCode::MissingKeys);
    assert_eq!(err.params["names"], "DEMO_API_KEY");

    // A value no token pattern would recognize, so only the known-value scrubber can hide it.
    let secret = format!("zebra-{}", uuid::Uuid::new_v4().simple());
    engine
        .set_env_value(&p.id, "DEMO_API_KEY", Some(&secret))
        .unwrap();
    engine
        .set_env_value(&p.id, "DEMO_MODE", Some("fast"))
        .unwrap();
    let vars = engine.env_vars(&p.id).unwrap();
    assert!(vars.iter().all(|v| v.has_value));
    assert!(vars[0].value.is_none(), "secret values never go to the UI");

    let seen = Arc::new(Mutex::new(String::new()));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let sink: EventSink = {
        let seen = seen.clone();
        Arc::new(move |e: EngineEvent| {
            let mut s = seen.lock().unwrap();
            match &e {
                EngineEvent::Log { line, .. } => s.push_str(&format!("{line}\n")),
                EngineEvent::Console { data, .. } => s.push_str(data),
                EngineEvent::RunState { state, .. } => {
                    let _ = tx.send(*state);
                }
                _ => {}
            }
        })
    };
    engine.launch(&p.id, sink).await.unwrap();
    tokio::time::timeout(Duration::from_secs(20), async {
        while let Some(state) = rx.recv().await {
            if matches!(state, RunState::Stopped | RunState::Crashed) {
                break;
            }
        }
    })
    .await
    .expect("program finished");
    // Terminal output is read on its own thread and can trail the exit notice slightly.
    for _ in 0..50 {
        if seen.lock().unwrap().contains("len=") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let shown = seen.lock().unwrap().clone();
    assert!(
        shown.contains(&format!("len={}", secret.len())),
        "the program received the key: {shown}"
    );
    assert!(shown.contains("key=[hidden]"), "{shown}");
    assert!(shown.contains("mode=fast"));
    assert!(!shown.contains(&secret));

    // Nothing the engine wrote anywhere contains the key: logs, state database, metadata.
    let hits = files_containing(home.path(), secret.as_bytes());
    assert!(hits.is_empty(), "secret written to {hits:?}");
    assert!(!engine.recent_log(&p.id, 100).join("\n").contains(&secret));

    // Removing the project deletes its keychain entry.
    assert_eq!(store.keys().len(), 1);
    engine.uninstall(&p.id).await.unwrap();
    assert!(store.keys().is_empty());
}

#[tokio::test]
async fn no_keychain_means_a_clear_message_not_a_file() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("launcher.json"),
        r#"{"version":1,"type":"terminal","run":{"command":["python","a.py"]},"env":[{"name":"X_API_KEY"}]}"#,
    )
    .unwrap();
    let home = tempfile::tempdir().unwrap();
    let engine = engine_with(home.path(), None, Arc::new(MemoryStore::unavailable()));
    let p = engine.add(&project.path().to_string_lossy()).await.unwrap();
    engine
        .review(&p.id, null_sink(), CancellationToken::new())
        .await
        .unwrap();
    let err = engine
        .set_env_value(&p.id, "X_API_KEY", Some("value-that-must-not-land-on-disk"))
        .unwrap_err();
    assert_eq!(err.code, ProblemCode::KeychainUnavailable);
    assert!(engine.settings().unwrap().keychain_problem.is_some());
    assert!(files_containing(home.path(), b"value-that-must-not-land-on-disk").is_empty());
}
