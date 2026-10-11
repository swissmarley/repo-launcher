//! Optional AI-suggested recipes (off by default). When Repo Launcher can't tell how to start a
//! project, the user may ask Claude, with their own Anthropic key, to write a `launcher.json`.
//!
//! - The message is built deterministically from the project's files, shown to the user in full
//!   ([`Preview`]), and sent only if it is unchanged (`digest`).
//! - `.env` files and key-like files are never included; all text is scrubbed.
//! - The answer is constrained to a JSON schema, then parsed by the same validator as an
//!   author's own `launcher.json`. It never runs without the safety gate, where it is red.

use crate::detect::FileTree;
use crate::problem::{FixAction, Problem, ProblemCode, Result};
use crate::redact;
use crate::secrets::Secret;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const MODEL: &str = "claude-opus-5-5";
const API_VERSION: &str = "2023-06-01";
/// Server-side fallback when a safety classifier declines (routed by Anthropic per category).
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
const MAX_TOKENS: u32 = 16_000;

const MAX_LISTED_FILES: usize = 400;
const README_CHARS: usize = 12_000;
const FILE_CHARS: usize = 6_000;
const TOTAL_CHARS: usize = 60_000;

/// Files that describe how a project is built and started, in the order they are included.
const MANIFESTS: &[&str] = &[
    "package.json",
    "pyproject.toml",
    "setup.py",
    "setup.cfg",
    "requirements.txt",
    "Pipfile",
    "environment.yml",
    "environment.yaml",
    "go.mod",
    "Cargo.toml",
    "Makefile",
    "justfile",
    "Procfile",
    "Dockerfile",
    "compose.yaml",
    "compose.yml",
    "docker-compose.yml",
    "docker-compose.yaml",
    "app.json",
    "main.py",
    "app.py",
    "server.py",
    "run.py",
    "index.js",
    "server.js",
    "main.go",
];

const SYSTEM: &str = "You help a desktop app called Repo Launcher start open-source projects for people \
who are not programmers. Repo Launcher could not work out how to start the project described in \
the user's message. Decide whether it is a runnable application and, if it is, write a \
launcher.json (version 1) describing how to install and run it.

Rules for the launcher.json:
- Commands are argument lists, never shell strings. Never use sudo, su, doas, runas, pkexec or \
gsudo. Never use absolute paths. Never run inline shell code (sh -c, bash -c, cmd /c, \
powershell -Command); a script already in the repository may be run, e.g. [\"bash\", \"start.sh\"].
- Repo Launcher provides private copies of Python (with uv), Node.js (npm, pnpm, yarn, bun) and \
Go. The first command word python, pip or uv uses the private Python environment; node, npm, \
npx, pnpm, yarn and bun use the private Node.js. Do not add steps that install these runtimes.
- Install Python dependencies with an install step {\"python\": {\"requirements\": [\"requirements.txt\"]}} \
or {\"python\": {\"project\": true}}, and Node dependencies with {\"node\": {}}. Use \"run\" steps \
only for other required setup such as a build.
- type is \"web\" for anything that serves a page in a browser, \"terminal\" for command-line \
programs, \"gui\" for desktop windows.
- For web apps, pass the port with the {port} placeholder (and the host with {host}) when the \
program accepts them, or set web.portEnv when it reads the port from an environment variable. \
Otherwise set web.discover to true.
- List settings the program needs from the user (API keys and the like) in env, with \
required true only when it cannot start without them. Use secret false only for values that \
are not sensitive.
- Prefer the simplest way to start the program that the README documents.
- If it is a library, a collection of files, or anything without a program to start, set \
runnable to false and omit launcher.
- explanation: one or two plain sentences for the user about what you found.";

/// Exactly what will be sent to Anthropic, shown to the user before sending.
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub model: String,
    pub system: String,
    pub message: String,
    /// Files whose contents are included (the rest are listed by name only).
    pub included_files: Vec<String>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub listed_files: usize,
    /// Identifies this exact message; sending requires it to be unchanged.
    pub digest: String,
}

/// What came back: a `launcher.json` text (not yet validated) and the model's explanation.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub launcher_json: Option<String>,
    pub explanation: String,
}

fn sensitive(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    name.starts_with(".env")
        || name.ends_with(".env")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.ends_with(".p12")
        || name.ends_with(".pfx")
        || name.starts_with("id_rsa")
        || name.starts_with("id_ed25519")
        || name.contains("secret")
        || name.contains("credential")
        || name == ".npmrc"
        || name == ".pypirc"
        || name == ".netrc"
}

fn excerpt(text: &str, limit: usize) -> (String, bool) {
    let scrubbed = redact::scrub(text);
    match scrubbed.char_indices().nth(limit) {
        Some((cut, _)) => (scrubbed[..cut].to_owned(), true),
        None => (scrubbed.into_owned(), false),
    }
}

/// Builds the message from the project's own files. Deterministic for the same files.
pub fn preview(project_label: &str, tree: &FileTree) -> Preview {
    let files: Vec<&str> = tree.files().filter(|f| !sensitive(f)).collect();
    let mut message = format!("Project: {project_label}\n\nFiles ({} total", files.len());
    if files.len() > MAX_LISTED_FILES {
        message.push_str(&format!(", first {MAX_LISTED_FILES} shown"));
    }
    message.push_str("):\n");
    for f in files.iter().take(MAX_LISTED_FILES) {
        message.push_str(f);
        message.push('\n');
    }

    let mut included = Vec::new();
    let mut budget = TOTAL_CHARS;
    let readme = files
        .iter()
        .find(|f| !f.contains('/') && f.to_ascii_lowercase().starts_with("readme"))
        .map(|f| f.to_string());
    let candidates = readme.into_iter().chain(
        MANIFESTS
            .iter()
            .filter(|m| files.contains(m))
            .map(|m| m.to_string()),
    );
    for path in candidates {
        let Some(text) = tree.read(&path) else {
            continue;
        };
        let limit = if path.to_ascii_lowercase().starts_with("readme") {
            README_CHARS
        } else {
            FILE_CHARS
        }
        .min(budget);
        if limit < 200 {
            break;
        }
        let (body, cut) = excerpt(&text, limit);
        budget -= body.chars().count();
        let note = if cut {
            format!(" (first {limit} characters)")
        } else {
            String::new()
        };
        message.push_str(&format!("\n--- {path}{note} ---\n{body}\n"));
        included.push(path);
    }

    let digest = hex::encode(Sha256::digest(format!("{MODEL}\n{SYSTEM}\n{message}")));
    Preview {
        model: MODEL.into(),
        system: SYSTEM.into(),
        message,
        included_files: included,
        listed_files: files.len(),
        digest,
    }
}

/// The answer's shape: launcher.json v1 restricted to what the validator accepts. Structured
/// outputs need `additionalProperties: false` on every object and no numeric/length limits.
pub fn schema() -> Value {
    let strings = json!({"type": "array", "items": {"type": "string"}});
    let platforms = json!({"type": "array", "items": {"type": "string", "enum": ["macos", "windows", "linux"]}});
    let step = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": {"type": "string"},
            "run": strings,
            "python": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "requirements": strings,
                    "project": {"type": "boolean"},
                    "packages": strings
                }
            },
            "node": {"type": "object", "additionalProperties": false, "properties": {}},
            "cwd": {"type": "string"},
            "platforms": platforms
        }
    });
    let launcher = json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["version", "type", "run"],
        "properties": {
            "version": {"type": "integer", "enum": [1]},
            "type": {"type": "string", "enum": ["web", "terminal", "gui"]},
            "name": {"type": "string"},
            "description": {"type": "string"},
            "requires": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "python": {"type": "string"},
                    "node": {"type": "string"},
                    "packageManager": {"type": "string"},
                    "system": strings
                }
            },
            "install": {"type": "array", "items": step},
            "run": {
                "type": "object",
                "additionalProperties": false,
                "required": ["command"],
                "properties": {"command": strings, "cwd": {"type": "string"}}
            },
            "web": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "port": {"type": "integer"},
                    "portEnv": {"type": "string"},
                    "path": {"type": "string"},
                    "discover": {"type": "boolean"}
                }
            },
            "env": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name"],
                    "properties": {
                        "name": {"type": "string"},
                        "label": {"type": "string"},
                        "description": {"type": "string"},
                        "required": {"type": "boolean"},
                        "secret": {"type": "boolean"},
                        "helpUrl": {"type": "string"}
                    }
                }
            },
            "platforms": platforms
        }
    });
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["runnable", "explanation"],
        "properties": {
            "runnable": {"type": "boolean"},
            "explanation": {"type": "string"},
            "launcher": launcher
        }
    })
}

fn failed(reason: &str, detail: impl std::fmt::Display) -> Problem {
    Problem::new(ProblemCode::AiFailed)
        .param("reason", reason)
        .fix(FixAction::Retry)
        .detail(detail)
}

/// Sends the previewed message with the user's key and returns the suggestion.
pub async fn suggest(
    http: &reqwest::Client,
    api_base: &str,
    key: &Secret,
    p: &Preview,
) -> Result<Suggestion> {
    let body = json!({
        "model": p.model,
        "max_tokens": MAX_TOKENS,
        "system": p.system,
        "messages": [{"role": "user", "content": p.message}],
        "output_config": {
            "effort": "medium",
            "format": {"type": "json_schema", "schema": schema()}
        },
        "fallbacks": "default"
    });
    let resp = http
        .post(format!("{}/v1/messages", api_base.trim_end_matches('/')))
        .header("x-api-key", key.expose())
        .header("anthropic-version", API_VERSION)
        .header("anthropic-beta", FALLBACK_BETA)
        .timeout(std::time::Duration::from_secs(300))
        .json(&body)
        .send()
        .await
        .map_err(|e| failed("network", e))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.map_err(|e| failed("network", e))?;
    let reply: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if status != 200 {
        let message = reply
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("no details");
        let reason = match status {
            401 | 403 => "key",
            429 => "busy",
            500..=599 => "unavailable",
            _ => "request",
        };
        return Err(failed(reason, format!("HTTP {status}: {message}")));
    }
    parse_reply(&reply)
}

/// Reads a Messages API reply: refusals and truncation first, then the JSON answer.
pub fn parse_reply(reply: &Value) -> Result<Suggestion> {
    match reply.get("stop_reason").and_then(Value::as_str) {
        Some("refusal") => {
            let category = reply
                .pointer("/stop_details/category")
                .and_then(Value::as_str)
                .unwrap_or("unspecified");
            return Err(failed("refusal", format!("declined ({category})")));
        }
        Some("max_tokens") => return Err(failed("tooLong", "answer was cut off")),
        _ => {}
    }
    let text: String = reply
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect();
    let answer: Value = serde_json::from_str(&text)
        .map_err(|e| Problem::new(ProblemCode::AiInvalidRecipe).detail(format!("not JSON: {e}")))?;
    let explanation = answer
        .get("explanation")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let runnable = answer.get("runnable").and_then(Value::as_bool) == Some(true);
    let launcher_json = match answer.get("launcher") {
        Some(l) if runnable => Some(serde_json::to_string_pretty(l).map_err(Problem::internal)?),
        _ => None,
    };
    Ok(Suggestion {
        launcher_json,
        explanation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(files: &[(&str, &str)]) -> (tempfile::TempDir, FileTree) {
        let dir = tempfile::tempdir().unwrap();
        for (name, body) in files {
            let p = dir.path().join(name);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        let t = FileTree::scan(dir.path()).unwrap();
        (dir, t)
    }

    #[test]
    fn preview_includes_readme_and_manifests_but_never_secrets() {
        let (_d, t) = tree(&[
            (
                "README.md",
                "Run with `python serve.py`. Token: ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            ),
            ("serve.py", "print('x')"),
            ("requirements.txt", "flask\n"),
            (
                ".env",
                "OPENAI_API_KEY=sk-real-key-should-never-leave-1234567890",
            ),
            (
                "config/credentials.json",
                "{\"password\": \"hunter2hunter2\"}",
            ),
        ]);
        let p = preview("acme/tool", &t);
        assert_eq!(p.included_files, ["README.md", "requirements.txt"]);
        assert!(p.message.contains("serve.py"));
        assert!(!p.message.contains(".env"));
        assert!(!p.message.contains("credentials"));
        assert!(!p.message.contains("sk-real-key"));
        assert!(
            !p.message.contains("ghp_abc"),
            "tokens in a README are scrubbed"
        );
        assert_eq!(p.digest, preview("acme/tool", &t).digest, "deterministic");
    }

    #[test]
    fn schema_objects_are_closed() {
        fn check(v: &Value) {
            if let Some(o) = v.as_object() {
                if o.get("type") == Some(&json!("object")) {
                    assert_eq!(o.get("additionalProperties"), Some(&json!(false)), "{v}");
                }
                o.values().for_each(check);
            } else if let Some(a) = v.as_array() {
                a.iter().for_each(check);
            }
        }
        check(&schema());
    }

    #[test]
    fn replies_are_read_carefully() {
        let ok = json!({
            "stop_reason": "end_turn",
            "content": [{"type": "thinking", "thinking": ""}, {"type": "text", "text":
                "{\"runnable\":true,\"explanation\":\"A Flask app.\",\"launcher\":{\"version\":1,\"type\":\"web\",\"run\":{\"command\":[\"python\",\"serve.py\"]}}}"}]
        });
        let s = parse_reply(&ok).unwrap();
        assert!(s.launcher_json.unwrap().contains("serve.py"));
        assert_eq!(s.explanation, "A Flask app.");

        let refusal =
            json!({"stop_reason": "refusal", "stop_details": {"category": "cyber"}, "content": []});
        let e = parse_reply(&refusal).unwrap_err();
        assert_eq!(
            (e.code, e.params["reason"].as_str()),
            (ProblemCode::AiFailed, "refusal")
        );

        let library = json!({"stop_reason": "end_turn", "content": [{"type": "text", "text": "{\"runnable\":false,\"explanation\":\"A library.\"}"}]});
        assert!(parse_reply(&library).unwrap().launcher_json.is_none());

        let junk =
            json!({"stop_reason": "end_turn", "content": [{"type": "text", "text": "nope"}]});
        assert_eq!(
            parse_reply(&junk).unwrap_err().code,
            ProblemCode::AiInvalidRecipe
        );
    }
}
