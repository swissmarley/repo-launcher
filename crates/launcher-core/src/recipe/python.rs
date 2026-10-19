use super::{AppKind, Recipe, RecipeSource, RunSpec, Step, StepLabel, WebSpec, pyversion};
use crate::detect::python::{EntryKind, PythonInfo};
use crate::problem::{Problem, ProblemCode};
use std::collections::BTreeMap;

const READY_TIMEOUT_SECS: u64 = 180;

pub fn recipe(py: &PythonInfo) -> Result<Recipe, Problem> {
    let entry = py
        .entries
        .first()
        .ok_or_else(|| Problem::new(ProblemCode::NoEntryPoint))?;
    let version = pyversion::choose(py.requires_python.as_deref(), py.pinned_version.as_deref());

    let mut install = setup_steps(&version);

    if py.has_uv_lock {
        let mut s = step(
            StepLabel::InstallHelpers {
                from: "uv.lock".into(),
            },
            &["uv", "sync", "--frozen", "--no-dev", "--python", &version],
        );
        s.env
            .insert("UV_PROJECT_ENVIRONMENT".into(), "{env}".into());
        install.push(s);
    } else {
        if let Some(req) = &py.requirements {
            install.push(step(
                StepLabel::InstallHelpers { from: req.clone() },
                &["uv", "pip", "install", "-r", req],
            ));
        }
        if py.installable {
            install.push(step(
                StepLabel::InstallHelpers {
                    from: "project".into(),
                },
                &["uv", "pip", "install", "-e", "."],
            ));
        } else if !py.pyproject_dependencies.is_empty() {
            let mut argv = vec!["uv".to_owned(), "pip".into(), "install".into()];
            argv.extend(py.pyproject_dependencies.iter().cloned());
            install.push(Step::new(
                StepLabel::InstallHelpers {
                    from: "pyproject.toml".into(),
                },
                argv,
            ));
        }
    }

    // Frameworks the entry point needs but the project forgot to declare.
    let needs: &[&str] = match entry.kind {
        EntryKind::Streamlit => &["streamlit"],
        EntryKind::Gradio => &["gradio"],
        EntryKind::Flask { .. } => &["flask"],
        EntryKind::FastApi { .. } => &["fastapi", "uvicorn"],
        EntryKind::Django => &["django"],
        EntryKind::Notebooks => &["jupyterlab"],
        _ => &[],
    };
    let missing: Vec<&str> = needs
        .iter()
        .copied()
        .filter(|n| !py.dependencies.contains(*n))
        .collect();
    if !missing.is_empty() {
        let mut argv = vec!["uv", "pip", "install"];
        argv.extend(&missing);
        install.push(step(
            StepLabel::InstallHelpers {
                from: "launcher".into(),
            },
            &argv,
        ));
    }

    let (kind, run, web) = run_for(&entry.kind, &entry.file, &mut install);
    Ok(Recipe {
        kind,
        source: RecipeSource::Heuristic,
        python: Some(version),
        node: None,
        go: false,
        env_vars: Vec::new(),
        system_tools: Vec::new(),
        install,
        run,
        web,
    })
}

fn run_for(
    kind: &EntryKind,
    file: &str,
    install: &mut Vec<Step>,
) -> (AppKind, RunSpec, Option<WebSpec>) {
    let web = |port: u16| {
        Some(WebSpec {
            preferred_port: port,
            path: "/".into(),
            ready_timeout_secs: READY_TIMEOUT_SECS,
            discover: false,
        })
    };
    match kind {
        EntryKind::Streamlit => (
            AppKind::Web,
            run(&[
                "python",
                "-m",
                "streamlit",
                "run",
                file,
                "--server.port",
                "{port}",
                "--server.address",
                "{host}",
                // Headless also skips Streamlit's first-run email prompt, which would hang.
                "--server.headless",
                "true",
                "--browser.gatherUsageStats",
                "false",
            ]),
            web(8501),
        ),
        EntryKind::Gradio => {
            let mut r = run(&["python", file]);
            r.env.insert("GRADIO_SERVER_PORT".into(), "{port}".into());
            r.env.insert("GRADIO_SERVER_NAME".into(), "{host}".into());
            r.env
                .insert("GRADIO_ANALYTICS_ENABLED".into(), "False".into());
            (AppKind::Web, r, web(7860))
        }
        EntryKind::Flask { app } => (
            AppKind::Web,
            run(&[
                "python", "-m", "flask", "--app", app, "run", "--port", "{port}", "--host",
                "{host}",
            ]),
            web(5000),
        ),
        EntryKind::FastApi { app } => (
            AppKind::Web,
            run(&[
                "python", "-m", "uvicorn", app, "--port", "{port}", "--host", "{host}",
            ]),
            web(8000),
        ),
        EntryKind::Django => {
            install.push(step(
                StepLabel::Custom {
                    name: "prepare-database".into(),
                },
                &["python", "manage.py", "migrate"],
            ));
            (
                AppKind::Web,
                run(&["python", "manage.py", "runserver", "{host}:{port}"]),
                web(8000),
            )
        }
        EntryKind::Script => (AppKind::Terminal, run(&["python", file]), None),
        EntryKind::Gui => (AppKind::Gui, run(&["python", file]), None),
        EntryKind::Module { module } => (AppKind::Terminal, run(&["python", "-m", module]), None),
        EntryKind::ConsoleScript { name } => (AppKind::Terminal, run(&[name]), None),
        EntryKind::Notebooks => {
            let r = run(&[
                "python",
                "-m",
                "jupyterlab",
                "--no-browser",
                "--ServerApp.port={port}",
                "--ServerApp.port_retries=0",
                "--ServerApp.ip={host}",
                // A fresh random token per launch; the address we open already carries it.
                "--IdentityProvider.token={token}",
            ]);
            let mut w = web(8888).expect("web spec");
            w.path = "/lab?token={token}".into();
            (AppKind::Web, r, Some(w))
        }
    }
}

/// Installs the Python version and creates the project's private environment.
pub(super) fn setup_steps(version: &str) -> Vec<Step> {
    vec![
        step(
            StepLabel::SetUpPython {
                version: version.to_owned(),
            },
            &["uv", "python", "install", "--no-bin", version],
        ),
        step(
            StepLabel::CreateEnvironment,
            &[
                "uv",
                "venv",
                "{env}",
                "--python",
                version,
                "--allow-existing",
                "--no-project",
            ],
        ),
    ]
}

fn step(label: StepLabel, argv: &[&str]) -> Step {
    Step::new(label, argv.iter().map(|s| s.to_string()).collect())
}

fn run(argv: &[&str]) -> RunSpec {
    RunSpec {
        argv: argv.iter().map(|s| s.to_string()).collect(),
        cwd: ".".into(),
        env: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::python::Entry;

    fn info(kind: EntryKind, deps: &[&str]) -> PythonInfo {
        PythonInfo {
            requirements: Some("requirements.txt".into()),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
            entries: vec![Entry {
                file: "app.py".into(),
                kind,
                score: 50,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn streamlit_recipe() {
        let r = recipe(&info(EntryKind::Streamlit, &["streamlit"])).unwrap();
        assert_eq!(r.kind, AppKind::Web);
        assert_eq!(r.python.as_deref(), Some("3.12"));
        assert_eq!(r.web.as_ref().unwrap().preferred_port, 8501);
        assert!(r.run.argv.contains(&"--server.headless".to_string()));
        let argvs: Vec<String> = r.install.iter().map(|s| s.argv.join(" ")).collect();
        assert_eq!(argvs[0], "uv python install --no-bin 3.12");
        assert_eq!(argvs[2], "uv pip install -r requirements.txt");
        assert_eq!(argvs.len(), 3, "nothing missing to add: {argvs:?}");
    }

    #[test]
    fn adds_missing_server_for_fastapi() {
        let r = recipe(&info(
            EntryKind::FastApi {
                app: "app:app".into(),
            },
            &["fastapi"],
        ))
        .unwrap();
        assert_eq!(
            r.install.last().unwrap().argv,
            ["uv", "pip", "install", "uvicorn"]
        );
        assert_eq!(r.run.argv[..4], ["python", "-m", "uvicorn", "app:app"]);
    }

    #[test]
    fn uv_lock_uses_sync() {
        let mut i = info(EntryKind::Script, &[]);
        i.has_uv_lock = true;
        let r = recipe(&i).unwrap();
        assert_eq!(r.kind, AppKind::Terminal);
        assert_eq!(r.install[2].argv[..3], ["uv", "sync", "--frozen"]);
        assert_eq!(
            r.install[2]
                .env
                .get("UV_PROJECT_ENVIRONMENT")
                .map(String::as_str),
            Some("{env}")
        );
    }

    #[test]
    fn no_entry_is_a_problem() {
        let r = recipe(&PythonInfo::default());
        assert_eq!(r.unwrap_err().code, ProblemCode::NoEntryPoint);
    }
}
