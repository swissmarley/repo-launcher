//! Detection and recipes over the real-repo corpus (fixtures/corpus.toml), offline, using the
//! snapshots in fixtures/corpus/. `RL_CORPUS_REPORT=1` prints what was decided for each repo.

use launcher_core::detect::{self, FileTree};
use launcher_core::recipe::{self, AppKind};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Corpus {
    repo: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    url: String,
    #[allow(dead_code)]
    commit: String,
    #[allow(dead_code)]
    note: Option<String>,
    kind: Option<String>,
    run: Option<Vec<String>>,
    problem: Option<String>,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn slug(url: &str) -> String {
    let path = url.trim_start_matches("https://github.com/");
    let mut parts: Vec<&str> = path.split('/').collect();
    let (owner, repo) = (parts.remove(0), parts.remove(0));
    let mut s = format!("{owner}__{repo}");
    if parts.first() == Some(&"tree") && parts.len() > 2 {
        s.push_str("__");
        s.push_str(&parts[2..].join("-"));
    }
    s
}

/// Windows builds get an `.exe` suffix; expectations are written without it.
fn os_neutral(argv: &[String]) -> Vec<String> {
    argv.iter()
        .map(|a| a.strip_suffix(".exe").unwrap_or(a).to_owned())
        .collect()
}

fn tree_of(dir: &Path) -> FileTree {
    let listing = std::fs::read_to_string(dir.join("tree.txt")).expect("snapshot listing");
    FileTree::from_parts(&dir.join("files"), listing.lines().map(str::to_owned))
}

#[test]
fn corpus_expectations() {
    let corpus: Corpus =
        toml::from_str(&std::fs::read_to_string(fixtures().join("corpus.toml")).unwrap()).unwrap();
    assert!(
        corpus.repo.len() >= 25,
        "the corpus should cover at least 25 repos"
    );
    let report = std::env::var_os("RL_CORPUS_REPORT").is_some();
    let mut failures = Vec::new();
    let (mut runnable, mut explained) = (0, 0);

    for e in &corpus.repo {
        let tree = tree_of(&fixtures().join("corpus").join(slug(&e.url)));
        let detected = detect::detect(&tree);
        let result = recipe::resolve(&tree, &detected);
        let got = match &result {
            Ok((r, _)) => format!("{:?} {:?}", r.kind, r.run.argv),
            Err(p) => format!("{:?}", p.code),
        };
        if report {
            println!(
                "{:<75} {got}",
                e.url.trim_start_matches("https://github.com/")
            );
            continue;
        }
        match (&result, &e.problem) {
            (Err(p), Some(want)) => {
                let code = serde_json::to_value(p.code).unwrap();
                if code.as_str() != Some(want) {
                    failures.push(format!("{}: expected {want}, got {got}", e.url));
                }
                explained += 1;
            }
            (Ok((r, _)), None) => {
                let kind = match r.kind {
                    AppKind::Web => "web",
                    AppKind::Terminal => "terminal",
                    AppKind::Gui => "gui",
                };
                let want_kind = e.kind.as_deref().unwrap_or("<missing>");
                let want_run = e.run.clone().unwrap_or_default();
                if kind != want_kind
                    || !os_neutral(&r.run.argv).starts_with(&want_run)
                    || want_run.is_empty()
                {
                    failures.push(format!(
                        "{}: expected {want_kind} {want_run:?}, got {got}",
                        e.url
                    ));
                }
                runnable += 1;
            }
            _ => failures.push(format!(
                "{}: expectation doesn't match outcome: got {got}",
                e.url
            )),
        }
    }
    if !report {
        assert!(
            failures.is_empty(),
            "{} corpus mismatches:\n{}",
            failures.len(),
            failures.join("\n")
        );
        println!("corpus: {runnable} runnable, {explained} explained");
    }
}
