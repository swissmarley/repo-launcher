//! `launcher.json`: a project's own description of how to install and run it.
//! Reference: docs/LAUNCHER_JSON.md and schema/launcher.schema.json.

use super::{
    AppKind, EnvVarSpec, NodeSpec, PackageManager, Recipe, RecipeSource, RunSpec, Step, StepLabel,
    WebSpec, current_os, pyversion,
};
use crate::detect::DetectionReport;
use crate::problem::{Problem, ProblemCode};
use crate::runtime::pins;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub const FILE_NAME: &str = "launcher.json";
pub const SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    #[serde(rename = "$schema")]
    _schema: Option<String>,
    version: u64,
    #[serde(rename = "type")]
    kind: AppKind,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    requires: Requires,
    #[serde(default)]
    install: Vec<InstallStep>,
    run: Run,
    #[serde(default)]
    web: Option<Web>,
    #[serde(default)]
    env: Vec<EnvVarSpec>,
    #[serde(default)]
    platforms: Vec<String>,
    /// Already merged into the document by `apply_override`; kept so the field is allowed.
    #[serde(default, rename = "overrides")]
    _overrides: BTreeMap<String, Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Requires {
    python: Option<String>,
    node: Option<String>,
    package_manager: Option<String>,
    #[serde(default)]
    system: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstallStep {
    name: Option<String>,
    run: Option<Vec<String>>,
    python: Option<PythonStep>,
    node: Option<NodeStep>,
    cwd: Option<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    platforms: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PythonStep {
    #[serde(default)]
    requirements: Vec<String>,
    /// Install the project itself (`pip install -e .`).
    #[serde(default)]
    project: bool,
    #[serde(default)]
    packages: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NodeStep {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Run {
    command: Vec<String>,
    cwd: Option<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Web {
    port: Option<u16>,
    port_env: Option<String>,
    path: Option<String>,
    ready_timeout_seconds: Option<u64>,
    #[serde(default)]
    discover: bool,
}

/// Display fields a manifest may set (applied to the project card).
#[derive(Debug, Clone, Default)]
pub struct Display {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
}

fn invalid(detail: impl std::fmt::Display) -> Problem {
    Problem::new(ProblemCode::ManifestInvalid).detail(detail)
}

/// Parses `launcher.json` text. `report` fills in what the manifest leaves to detection (e.g. the
/// Node package manager from the lockfile).
pub fn parse(text: &str, report: &DetectionReport) -> Result<(Recipe, Display), Problem> {
    let mut doc: Value =
        serde_json::from_str(text).map_err(|e| invalid(format!("not valid JSON: {e}")))?;
    apply_override(&mut doc)?;
    let m: Manifest = serde_json::from_value(doc).map_err(invalid)?;
    if m.version != SCHEMA_VERSION {
        return Err(invalid(format!(
            "unsupported version {} (expected {SCHEMA_VERSION})",
            m.version
        )));
    }
    if !m.platforms.is_empty() && !m.platforms.iter().any(|p| p == current_os()) {
        return Err(Problem::new(ProblemCode::UnsupportedProject).param("platform", current_os()));
    }

    let uses = |tool: &[&str]| {
        let mut argvs = m
            .install
            .iter()
            .filter_map(|s| s.run.as_ref())
            .chain([&m.run.command]);
        argvs.any(|a| a.first().is_some_and(|c| tool.contains(&c.as_str())))
    };
    let needs_python = m.requires.python.is_some()
        || m.install.iter().any(|s| s.python.is_some())
        || uses(&["python", "python3", "pip", "uv"]);
    let needs_node = m.requires.node.is_some()
        || m.install.iter().any(|s| s.node.is_some())
        || uses(&["node", "npm", "npx", "pnpm", "yarn", "bun"]);

    let needs_go = uses(&["go"]);
    let python = needs_python.then(|| pyversion::choose(m.requires.python.as_deref(), None));
    let node = needs_node.then(|| node_spec(&m.requires, report));

    let mut install = Vec::new();
    if let Some(v) = &python {
        install.extend(super::python::setup_steps(v));
    }
    for (i, s) in m.install.iter().enumerate() {
        install.extend(convert_step(s, i, node.as_ref(), report)?);
    }

    for argv in install.iter().map(|s| &s.argv).chain([&m.run.command]) {
        validate_argv(argv)?;
    }

    let mut run = RunSpec {
        argv: m.run.command.clone(),
        cwd: m.run.cwd.clone().unwrap_or_else(|| ".".into()),
        env: m.run.env.clone(),
    };
    let web = match (m.kind, m.web) {
        (AppKind::Web, w) => {
            let w = w.unwrap_or_default();
            if let Some(var) = &w.port_env {
                run.env.insert(var.clone(), "{port}".into());
            }
            let injects = w.port_env.is_some() || run.argv.iter().any(|a| a.contains("{port}"));
            Some(WebSpec {
                preferred_port: w.port.unwrap_or(8000),
                path: w.path.unwrap_or_else(|| "/".into()),
                ready_timeout_secs: w.ready_timeout_seconds.unwrap_or(300),
                discover: w.discover || !injects,
            })
        }
        _ => None,
    };

    let recipe = Recipe {
        kind: m.kind,
        source: RecipeSource::Manifest,
        python,
        python_fallbacks: Vec::new(),
        node,
        go: needs_go,
        install,
        run,
        web,
        env_vars: m.env,
        system_tools: m.requires.system,
    };
    Ok((
        recipe,
        Display {
            name: m.name,
            description: m.description,
            icon: m.icon,
        },
    ))
}

/// Deep-merges `overrides.<this OS>` into the document.
fn apply_override(doc: &mut Value) -> Result<(), Problem> {
    let Some(ov) = doc
        .get("overrides")
        .and_then(|o| o.get(current_os()))
        .cloned()
    else {
        return Ok(());
    };
    if !ov.is_object() {
        return Err(invalid(format!(
            "overrides.{} must be an object",
            current_os()
        )));
    }
    merge(doc, ov);
    Ok(())
}

fn merge(base: &mut Value, patch: Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                merge(b.entry(k).or_insert(Value::Null), v);
            }
        }
        (b, p) => *b = p,
    }
}

fn node_spec(req: &Requires, report: &DetectionReport) -> NodeSpec {
    let detected = report.node.as_ref();
    let (pm, pm_version) = match req
        .package_manager
        .as_deref()
        .map(|p| p.split_once('@').unwrap_or((p, "")))
    {
        Some((name, v)) => (
            match name {
                "pnpm" => PackageManager::Pnpm,
                "yarn" => PackageManager::Yarn,
                "bun" => PackageManager::Bun,
                _ => PackageManager::Npm,
            },
            (!v.is_empty()).then(|| v.to_owned()),
        ),
        None => (
            detected
                .and_then(|n| n.package_manager)
                .unwrap_or(PackageManager::Npm),
            detected.and_then(|n| n.package_manager_version.clone()),
        ),
    };
    let major = req
        .node
        .as_deref()
        .and_then(crate::detect::node::node_major)
        .or_else(|| detected.and_then(|n| n.node_major));
    NodeSpec {
        version: pins::node_version_for(major).to_owned(),
        package_manager: pm,
        package_manager_version: pm_version,
    }
}

fn convert_step(
    s: &InstallStep,
    index: usize,
    node: Option<&NodeSpec>,
    report: &DetectionReport,
) -> Result<Vec<Step>, Problem> {
    let kinds = [s.run.is_some(), s.python.is_some(), s.node.is_some()]
        .iter()
        .filter(|k| **k)
        .count();
    if kinds != 1 {
        return Err(invalid(format!(
            "install[{index}] needs exactly one of \"run\", \"python\" or \"node\""
        )));
    }
    let label = |fallback: StepLabel| {
        s.name
            .clone()
            .map(|name| StepLabel::Custom { name })
            .unwrap_or(fallback)
    };
    let finish = |mut step: Step| {
        step.cwd = s.cwd.clone().unwrap_or_else(|| ".".into());
        step.env.extend(s.env.clone());
        step.platforms = s.platforms.clone();
        step
    };
    let mut out = Vec::new();
    if let Some(argv) = &s.run {
        out.push(finish(Step::new(
            label(StepLabel::Custom {
                name: format!("step {}", index + 1),
            }),
            argv.clone(),
        )));
    }
    if let Some(py) = &s.python {
        for req in &py.requirements {
            let argv = ["uv", "pip", "install", "-r", req]
                .map(String::from)
                .to_vec();
            out.push(finish(Step::new(
                label(StepLabel::InstallHelpers { from: req.clone() }),
                argv,
            )));
        }
        if !py.packages.is_empty() {
            let mut argv: Vec<String> = ["uv", "pip", "install"].map(String::from).to_vec();
            argv.extend(py.packages.iter().cloned());
            out.push(finish(Step::new(
                label(StepLabel::InstallHelpers {
                    from: "launcher.json".into(),
                }),
                argv,
            )));
        }
        if py.project {
            let argv = ["uv", "pip", "install", "-e", "."]
                .map(String::from)
                .to_vec();
            out.push(finish(Step::new(
                label(StepLabel::InstallHelpers {
                    from: "project".into(),
                }),
                argv,
            )));
        }
    }
    if s.node.is_some() {
        let pm = node
            .map(|n| n.package_manager)
            .unwrap_or(PackageManager::Npm);
        let default_info = crate::detect::node::NodeInfo::default();
        let info = report.node.as_ref().unwrap_or(&default_info);
        out.push(finish(super::node::install_step(pm, info)));
    }
    Ok(out)
}

/// Commands must be plain programs: no privilege escalation, no inline shell code, no
/// absolute paths outside the project (DECISIONS D-008).
fn validate_argv(argv: &[String]) -> Result<(), Problem> {
    let Some(first) = argv.first() else {
        return Err(invalid("empty command"));
    };
    let prog = first
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(first)
        .to_ascii_lowercase();
    let prog = prog.trim_end_matches(".exe");
    if matches!(prog, "sudo" | "su" | "doas" | "runas" | "pkexec" | "gsudo") {
        return Err(invalid(format!(
            "\"{first}\" asks for administrator rights, which Repo Launcher never uses"
        )));
    }
    let absolute =
        first.starts_with('/') || first.starts_with('\\') || first.chars().nth(1) == Some(':');
    if absolute {
        return Err(invalid(format!("\"{first}\" points outside the project")));
    }
    let shells = ["sh", "bash", "zsh", "fish", "cmd", "powershell", "pwsh"];
    let inline = argv.iter().skip(1).any(|a| {
        matches!(
            a.to_ascii_lowercase().as_str(),
            "-c" | "/c" | "-command" | "-encodedcommand"
        )
    });
    if shells.contains(&prog) && inline {
        return Err(invalid(format!(
            "\"{first}\" runs inline shell code; put it in a script file in the repo instead"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::DetectionReport;

    fn report() -> DetectionReport {
        DetectionReport {
            ecosystems: vec![],
            has_launcher_json: true,
            python: None,
            node: None,
            go: None,
            compose_file: None,
            env_hints: vec![],
        }
    }

    #[test]
    fn full_manifest() {
        let text = r#"{
          "$schema": "https://example/launcher.schema.json",
          "version": 1, "type": "web", "name": "Photo Restorer",
          "requires": { "python": ">=3.10,<3.12" },
          "install": [
            { "python": { "requirements": ["requirements.txt"] } },
            { "name": "Download the model", "run": ["python", "scripts/fetch.py"], "platforms": ["macos", "linux", "windows"] }
          ],
          "run": { "command": ["python", "app.py", "--port", "{port}"] },
          "web": { "port": 7860 },
          "env": [{ "name": "OPENAI_API_KEY", "required": true, "helpUrl": "https://platform.openai.com/api-keys" }]
        }"#;
        let (r, d) = parse(text, &report()).unwrap();
        assert_eq!(d.name.as_deref(), Some("Photo Restorer"));
        assert_eq!(r.source, RecipeSource::Manifest);
        assert_eq!(r.python.as_deref(), Some("3.11"));
        assert_eq!(
            r.install.len(),
            4,
            "python setup (2) + requirements + custom"
        );
        assert_eq!(
            r.install[3].label,
            StepLabel::Custom {
                name: "Download the model".into()
            }
        );
        let web = r.web.unwrap();
        assert_eq!((web.preferred_port, web.discover), (7860, false));
        assert!(
            r.env_vars[0].secret,
            "env vars are secret unless said otherwise"
        );
    }

    #[test]
    fn port_env_and_discovery() {
        let text = r#"{"version":1,"type":"web","run":{"command":["node","server.js"]},"web":{"portEnv":"PORT"}}"#;
        let (r, _) = parse(text, &report()).unwrap();
        assert_eq!(r.run.env.get("PORT").map(String::as_str), Some("{port}"));
        assert!(!r.web.as_ref().unwrap().discover);
        assert!(r.node.is_some(), "node detected from the run command");

        let text = r#"{"version":1,"type":"web","run":{"command":["node","server.js"]}}"#;
        let (r, _) = parse(text, &report()).unwrap();
        assert!(
            r.web.unwrap().discover,
            "no way to inject a port, so discover it"
        );
    }

    #[test]
    fn per_os_overrides() {
        let os = current_os();
        let text = format!(
            r#"{{"version":1,"type":"terminal","run":{{"command":["python","a.py"]}},"overrides":{{"{os}":{{"run":{{"command":["python","b.py"]}}}}}}}}"#
        );
        let (r, _) = parse(&text, &report()).unwrap();
        assert_eq!(r.run.argv, ["python", "b.py"]);
    }

    #[test]
    fn rejects_mistakes_and_unsafe_commands() {
        let cases = [
            r#"{"version":1,"type":"web","run":{"command":["x"]},"typo":1}"#,
            r#"{"version":2,"type":"web","run":{"command":["x"]}}"#,
            r#"{"version":1,"type":"web","run":{"command":["sudo","x"]}}"#,
            r#"{"version":1,"type":"web","run":{"command":["/usr/bin/python3","x"]}}"#,
            r#"{"version":1,"type":"web","run":{"command":["bash","-c","curl x | sh"]}}"#,
            r#"{"version":1,"type":"web","install":[{"run":["x"],"python":{}}],"run":{"command":["x"]}}"#,
            r#"not json"#,
        ];
        for c in cases {
            let err = parse(c, &report()).unwrap_err();
            assert_eq!(err.code, ProblemCode::ManifestInvalid, "{c}");
        }
        let ok = r#"{"version":1,"type":"terminal","run":{"command":["bash","./start.sh"]}}"#;
        assert!(parse(ok, &report()).is_ok());
    }
}
