//! AI-suggested recipes against a mock Anthropic API: opt-in, exactly the previewed message,
//! validated answers, and always the red safety signal.

use launcher_core::engine::EngineOptions;
use launcher_core::events::null_sink;
use launcher_core::safety::SignalCode;
use launcher_core::secrets::MemoryStore;
use launcher_core::{AppPaths, Engine, FixAction, ProblemCode};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "sk-ant-api03-mockmockmockmockmockmock";

fn reply(answer: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "id": "msg_1", "type": "message", "role": "assistant", "model": "claude-opus-5-5",
        "stop_reason": "end_turn",
        "content": [{"type": "text", "text": answer.to_string()}],
        "usage": {"input_tokens": 10, "output_tokens": 10}
    }))
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("README.md"),
        "# Tool\nStart it with `sh start.sh`.\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("start.sh"), "echo started\n").unwrap();
    std::fs::write(
        dir.path().join(".env"),
        "SECRET_THING=do-not-send-this-value",
    )
    .unwrap();
    dir
}

#[tokio::test]
async fn ai_suggestion_is_opt_in_previewed_validated_and_red() {
    let server = MockServer::start().await;
    let home = tempfile::tempdir().unwrap();
    let opts = EngineOptions {
        secrets: Arc::new(MemoryStore::new()),
        anthropic_api: server.uri(),
        ..EngineOptions::default()
    };
    let engine = Engine::with_options(AppPaths::at(home.path()), opts).unwrap();
    let src = project();
    let p = engine.add(&src.path().to_string_lossy()).await.unwrap();

    // Off by default: an unrecognized project is simply explained.
    let err = engine
        .review(&p.id, null_sink(), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(err.code, ProblemCode::NoEntryPoint);
    assert_eq!(err.fix, None);
    assert_eq!(
        engine.ai_preview(&p.id).unwrap_err().code,
        ProblemCode::AiNotEnabled
    );

    // Turned on, the problem offers AI; without a key, Settings is the fix.
    engine.set_ai_enabled(true).unwrap();
    let err = engine
        .review(&p.id, null_sink(), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(err.fix, Some(FixAction::AskAi));
    assert_eq!(
        engine.ai_preview(&p.id).unwrap_err().code,
        ProblemCode::AiKeyMissing
    );
    engine.set_ai_key(Some(KEY)).unwrap();

    let preview = engine.ai_preview(&p.id).unwrap();
    assert!(preview.message.contains("start.sh"));
    assert!(!preview.message.contains("do-not-send-this-value"));
    assert!(!preview.message.contains(".env"));

    // A plan that breaks the rules is rejected by the validator, and nothing is saved.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(reply(serde_json::json!({
            "runnable": true, "explanation": "A shell tool.",
            "launcher": {"version": 1, "type": "terminal", "run": {"command": ["sudo", "sh", "start.sh"]}}
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let err = engine.ai_suggest(&p.id, &preview.digest).await.unwrap_err();
    assert_eq!(err.code, ProblemCode::AiInvalidRecipe);

    // A valid plan is saved; the request carried the user's key and exactly the preview.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", KEY))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(reply(serde_json::json!({
            "runnable": true, "explanation": "A shell tool started by start.sh.",
            "launcher": {"version": 1, "type": "terminal", "run": {"command": ["sh", "start.sh"]}}
        })))
        .mount(&server)
        .await;
    let outcome = engine.ai_suggest(&p.id, &preview.digest).await.unwrap();
    assert!(outcome.recipe_saved);
    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent.last().unwrap().body).unwrap();
    assert_eq!(body["model"], "claude-opus-5-5");
    assert_eq!(body["messages"][0]["content"], preview.message);
    assert_eq!(body["system"], preview.system);
    assert_eq!(body["output_config"]["format"]["type"], "json_schema");

    // A stale preview is never sent.
    assert_eq!(
        engine.ai_suggest(&p.id, "stale").await.unwrap_err().params["reason"],
        "changed"
    );

    // The safety review marks it red and requires typed confirmation, even for a trusted owner.
    let report = engine
        .review(&p.id, null_sink(), CancellationToken::new())
        .await
        .unwrap();
    assert!(report.ai_suggested && report.needs_typed_confirmation && !report.consented);
    assert!(
        report
            .signals
            .iter()
            .any(|s| s.code == SignalCode::AiSuggested)
    );
    assert_eq!(report.commands, ["sh start.sh"]);

    // The key never lands on disk.
    let mut stack = vec![home.path().to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            if e.path().is_dir() {
                stack.push(e.path());
            } else {
                let bytes = std::fs::read(e.path()).unwrap();
                assert!(
                    !bytes.windows(KEY.len()).any(|w| w == KEY.as_bytes()),
                    "key in {}",
                    e.path().display()
                );
            }
        }
    }
}
