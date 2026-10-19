//! A recipe is the normalized "how to install and run this project". Heuristics produce one
//! today; `launcher.json` and AI proposals will produce the same structure.

pub mod keys;
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
    /// Suggested by an AI model (opt-in); always shown as a red risk signal.
    Ai,
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
    /// Older Pythons to try in turn, newest first, if installing the project's packages fails
    /// on `python`: pinned old packages often have ready-made builds only for older Pythons.
    /// Empty when the project names its Python version.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub python_fallbacks: Vec<String>,
    #[serde(default)]
    pub node: Option<NodeSpec>,
    /// Needs the managed Go toolchain.
    #[serde(default)]
    pub go: bool,
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
    /// Stable hash of what will be installed and run, used to decide whether consent must be
    /// asked again. The list of settings the project asks for is left out: a new optional key
    /// doesn't change what runs on the computer. So are the fallback Pythons: they run the same
    /// steps with another of our own runtimes.
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut copy = self.clone();
        copy.env_vars.clear();
        copy.python_fallbacks.clear();
        let json = serde_json::to_vec(&copy).unwrap_or_default();
        hex::encode(Sha256::digest(json))
    }

    /// The same recipe on another Python version: the version in its `uv` steps is replaced.
    pub fn with_python(&self, version: &str) -> Recipe {
        let mut r = self.clone();
        if let Some(old) = &self.python {
            for step in r
                .install
                .iter_mut()
                .filter(|s| s.argv.first().is_some_and(|a| a == "uv"))
            {
                if let StepLabel::SetUpPython { version: v } = &mut step.label {
                    *v = version.to_owned();
                }
                for arg in step.argv.iter_mut().filter(|a| *a == old) {
                    *arg = version.to_owned();
                }
            }
            r.python = Some(version.to_owned());
        }
        r.python_fallbacks.clear();
        r
    }
}

/// The recipe for a project: its own `launcher.json` if it has one, otherwise heuristics.
pub fn resolve(
    tree: &crate::detect::FileTree,
    report: &DetectionReport,
) -> Result<(Recipe, Option<manifest::Display>), Problem> {
    let (mut recipe, display) = match tree.read(manifest::FILE_NAME) {
        Some(text) => manifest::parse(&text, report).map(|(r, d)| (r, Some(d)))?,
        None => (from_detection(report)?, None),
    };
    if recipe.env_vars.is_empty() {
        recipe.env_vars = env_from_hints(report);
    }
    Ok((recipe, display))
}

/// Settings found by heuristics. A launcher.json that lists its own settings replaces these.
pub fn env_from_hints(report: &DetectionReport) -> Vec<EnvVarSpec> {
    report
        .env_hints
        .iter()
        .map(|h| EnvVarSpec {
            name: h.name.clone(),
            label: None,
            description: None,
            required: h.required,
            secret: h.secret,
            help_url: None,
        })
        .collect()
}

/// Builds a recipe from heuristics. When a repo has both Python and Node, the one with a
/// clear way to start wins; web apps beat command-line tools.
pub fn from_detection(report: &DetectionReport) -> Result<Recipe, Problem> {
    // PHP/Ruby apps often ship a JavaScript asset pipeline (Laravel + Vite). Running only that
    // would show a blank asset server, so explain the real project type instead.
    let server_lang = report
        .ecosystems
        .iter()
        .any(|e| matches!(e, Ecosystem::Php | Ecosystem::Ruby));
    if server_lang && report.node.is_some() && report.python.is_none() && report.go.is_none() {
        return Err(unsupported(report));
    }
    let py = report.python.as_ref().map(python::recipe);
    let js = report.node.as_ref().map(node::recipe);
    let go = report.go.as_ref().map(go_recipe);
    let score = |r: &Option<Result<Recipe, Problem>>| match r {
        Some(Ok(r)) if r.kind == AppKind::Web => 3,
        Some(Ok(_)) => 2,
        Some(Err(_)) => 1,
        None => 0,
    };
    let mut ranked = [(score(&py), py), (score(&js), js), (score(&go), go)];
    // Stable: on ties Python, then Node, then Go keep their order.
    ranked.sort_by_key(|r| std::cmp::Reverse(r.0));
    let [(best_score, best), ..] = ranked;
    let best = if best_score == 0 { None } else { best };
    match best {
        Some(Ok(r)) => Ok(r),
        Some(Err(p)) if p.code != ProblemCode::NoEntryPoint => Err(p),
        _ => docker_compose(report)
            .or_else(|| static_site(report))
            .ok_or_else(|| unsupported(report)),
    }
}

/// A plain website (an `index.html` and nothing that starts or builds it), served as it is by
/// the managed Python's built-in web server. Not when a server-side language we don't run is
/// present: then `index.html` is a template (Jekyll, PHP), not the page.
fn static_site(report: &DetectionReport) -> Option<Recipe> {
    let plain = report
        .ecosystems
        .iter()
        .all(|e| matches!(e, Ecosystem::Python | Ecosystem::Node | Ecosystem::Docker));
    if !report.static_index || !plain {
        return None;
    }
    let version = pyversion::choose(None, None);
    Some(Recipe {
        kind: AppKind::Web,
        source: RecipeSource::Heuristic,
        python: Some(version.clone()),
        python_fallbacks: Vec::new(),
        node: None,
        go: false,
        install: python::setup_steps(&version),
        run: RunSpec {
            argv: ["python", "-m", "http.server", "{port}", "--bind", "{host}"]
                .map(String::from)
                .to_vec(),
            cwd: ".".into(),
            env: BTreeMap::new(),
        },
        web: Some(WebSpec {
            preferred_port: 8000,
            path: "/".into(),
            ready_timeout_secs: 60,
            discover: false,
        }),
        env_vars: Vec::new(),
        system_tools: Vec::new(),
    })
}

/// Builds a Go module's main package with the managed toolchain into the project's private
/// folder (static build, so no C compiler is needed), then runs it in the console.
fn go_recipe(info: &crate::detect::go::GoInfo) -> Result<Recipe, Problem> {
    let pkg = info
        .main_package
        .clone()
        .ok_or_else(|| Problem::new(ProblemCode::NoEntryPoint))?;
    let exe = if cfg!(windows) {
        format!("{}.exe", info.binary)
    } else {
        info.binary.clone()
    };
    let output = format!("{{env}}/bin/{exe}");
    let mut build = Step::new(
        StepLabel::BuildProject,
        ["go", "build", "-o", &output, &pkg]
            .map(String::from)
            .to_vec(),
    );
    build.env.insert("CGO_ENABLED".into(), "0".into());
    Ok(Recipe {
        kind: AppKind::Terminal,
        source: RecipeSource::Heuristic,
        python: None,
        python_fallbacks: Vec::new(),
        node: None,
        go: true,
        install: vec![build],
        run: RunSpec {
            argv: vec![output],
            cwd: ".".into(),
            env: BTreeMap::new(),
        },
        web: None,
        env_vars: Vec::new(),
        system_tools: Vec::new(),
    })
}

/// Projects described only by a Compose file run with Docker, if the user already has it
/// (we never install Docker). Output shows in the console; any web address it prints is offered.
fn docker_compose(report: &DetectionReport) -> Option<Recipe> {
    let file = report.compose_file.as_ref()?;
    let argv = ["docker", "compose", "-f", file, "up", "--build"]
        .map(String::from)
        .to_vec();
    Some(Recipe {
        kind: AppKind::Terminal,
        source: RecipeSource::Heuristic,
        python: None,
        python_fallbacks: Vec::new(),
        node: None,
        go: false,
        install: Vec::new(),
        run: RunSpec {
            argv,
            cwd: ".".into(),
            env: BTreeMap::new(),
        },
        web: None,
        env_vars: Vec::new(),
        system_tools: vec!["docker".into()],
    })
}

fn unsupported(report: &DetectionReport) -> Problem {
    let supported = [
        Ecosystem::Python,
        Ecosystem::Node,
        Ecosystem::Go,
        Ecosystem::Docker,
    ];
    match report.ecosystems.iter().find(|e| !supported.contains(e)) {
        Some(e) => {
            Problem::new(ProblemCode::UnsupportedProject).param("ecosystem", e.display_name())
        }
        None => Problem::new(ProblemCode::NoEntryPoint),
    }
}
