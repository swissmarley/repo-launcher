//! A recipe is the normalized "how to install and run this project". Heuristics produce one
//! today; `launcher.json` and AI proposals will produce the same structure.

pub mod manifest;
mod node;
mod python;
pub mod pyversion;
pub mod shell_words;

use crate::detect::{DetectionReport, Ecosystem};
use crate::problem::{Problem, ProblemCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppKind {
    Web,
    Terminal,
    Gui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecipeSource {
    Manifest,
    Heuristic,
}

/// Plain-language identity of a step; the UI turns it into words.
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StepLabel {
    SetUpPython { version: String },
    SetUpNode { version: String },
    CreateEnvironment,
    InstallHelpers { from: String },
    BuildProject,
    Custom { name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSpec {
    /// Exact pinned Node version (see `runtime::pins`).
    pub version: String,
    pub package_manager: PackageManager,
    pub package_manager_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub label: StepLabel,
    /// argv. The first element is resolved to a managed tool (`uv`, `python`, …).
    pub argv: Vec<String>,
    /// Relative to the project source folder.
    pub cwd: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Commands tried in order if `argv` fails, e.g. a non-frozen install when the lockfile is
    /// out of date.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<Vec<String>>,
    /// Only run on these OSes ("macos", "windows", "linux"); empty means everywhere.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<String>,
}

impl Step {
    pub fn new(label: StepLabel, argv: Vec<String>) -> Self {
        Self {
            label,
            argv,
            cwd: ".".into(),
            env: BTreeMap::new(),
            fallbacks: Vec::new(),
            platforms: Vec::new(),
        }
    }

    pub fn applies_here(&self) -> bool {
        self.platforms.is_empty() || self.platforms.iter().any(|p| p == current_os())
    }
}

pub fn current_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macos",
        "windows" => "windows",
        _ => "linux",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSpec {
    pub argv: Vec<String>,
    pub cwd: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSpec {
    pub preferred_port: u16,
    pub path: String,
    pub ready_timeout_secs: u64,
    /// The app picks its own port, so we can't inject one; we learn its URL from its output.
    #[serde(default)]
    pub discover: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub kind: AppKind,
    pub source: RecipeSource,
    pub python: Option<String>,
    #[serde(default)]
    pub node: Option<NodeSpec>,
    pub install: Vec<Step>,
    pub run: RunSpec,
    pub web: Option<WebSpec>,
    /// Settings the project needs from the user (API keys etc.). Values never live here.
    #[serde(default)]
    pub env_vars: Vec<EnvVarSpec>,
    /// Programs the project expects to already be on the computer (e.g. `ffmpeg`). Checked,
    /// never installed.
    #[serde(default)]
    pub system_tools: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarSpec {
    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default = "yes")]
    pub secret: bool,
    pub help_url: Option<String>,
}

fn yes() -> bool {
    true
}

impl Recipe {
    /// Stable hash used to decide whether consent must be asked again.
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let json = serde_json::to_vec(self).unwrap_or_default();
        hex::encode(Sha256::digest(json))
    }
}

/// The recipe for a project: its own `launcher.json` if it has one, otherwise heuristics.
pub fn resolve(
    tree: &crate::detect::FileTree,
    report: &DetectionReport,
) -> Result<(Recipe, Option<manifest::Display>), Problem> {
    match tree.read(manifest::FILE_NAME) {
        Some(text) => manifest::parse(&text, report).map(|(r, d)| (r, Some(d))),
        None => from_detection(report).map(|r| (r, None)),
    }
}

/// Builds a recipe from heuristics. When a repo has both Python and Node, the one with a
/// clear way to start wins; web apps beat command-line tools.
pub fn from_detection(report: &DetectionReport) -> Result<Recipe, Problem> {
    let py = report.python.as_ref().map(python::recipe);
    let js = report.node.as_ref().map(node::recipe);
    let score = |r: &Option<Result<Recipe, Problem>>| match r {
        Some(Ok(r)) if r.kind == AppKind::Web => 3,
        Some(Ok(_)) => 2,
        Some(Err(_)) => 1,
        None => 0,
    };
    let (py_score, js_score) = (score(&py), score(&js));
    let best = if py_score == 0 && js_score == 0 {
        None
    } else if js_score > py_score {
        js
    } else {
        py
    };
    match best {
        Some(Ok(r)) => Ok(r),
        Some(Err(p)) if p.code != ProblemCode::NoEntryPoint => Err(p),
        _ => Err(unsupported(report)),
    }
}

fn unsupported(report: &DetectionReport) -> Problem {
    let supported = [Ecosystem::Python, Ecosystem::Node];
    match report.ecosystems.iter().find(|e| !supported.contains(e)) {
        Some(e) => {
            Problem::new(ProblemCode::UnsupportedProject).param("ecosystem", format!("{e:?}"))
        }
        None => Problem::new(ProblemCode::NoEntryPoint),
    }
}
