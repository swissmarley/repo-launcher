use super::shell_words;
use super::{
    AppKind, NodeSpec, PackageManager, Recipe, RecipeSource, RunSpec, Step, StepLabel, WebSpec,
};
use crate::detect::node::{NodeFramework, NodeInfo};
use crate::problem::{Problem, ProblemCode};
use crate::runtime::pins;
use std::collections::BTreeMap;

const READY_TIMEOUT_SECS: u64 = 300;

pub fn recipe(info: &NodeInfo) -> Result<Recipe, Problem> {
    let pm = info.package_manager.unwrap_or(PackageManager::Npm);
    let node = NodeSpec {
        version: pins::node_version_for(info.node_major).to_owned(),
        package_manager: pm,
        package_manager_version: info.package_manager_version.clone(),
    };
    let app_dir = if info.app_dir.is_empty() {
        ".".to_owned()
    } else {
        info.app_dir.clone()
    };
    let mut install = vec![install_step(pm, info)];

    let (kind, run, web) = if let Some(r) = electron(info, pm, &app_dir) {
        r
    } else if let Some(r) = web_app(info, pm, &app_dir) {
        r
    } else if let Some((_, file)) = info.bins.first() {
        // Command-line tool. Some publish their entry point only after a build.
        let file = file.trim_start_matches("./");
        if info.scripts.contains_key("build")
            && (file.starts_with("dist/") || file.starts_with("build/") || file.starts_with("lib/"))
        {
            let mut build = Step::new(StepLabel::BuildProject, pm_run(pm, "build"));
            build.cwd = app_dir.clone();
            install.push(build);
        }
        (
            AppKind::Terminal,
            run_in(&app_dir, vec!["node".into(), file.into()]),
            None,
        )
    } else if info.scripts.contains_key("start") {
        // Unknown kind of app: run it and watch its output for a web address.
        (
            AppKind::Terminal,
            run_in(&app_dir, pm_run(pm, "start")),
            None,
        )
    } else if let Some(main) = &info.main {
        (
            AppKind::Terminal,
            run_in(&app_dir, vec!["node".into(), main.clone()]),
            None,
        )
    } else {
        return Err(Problem::new(ProblemCode::NoEntryPoint));
    };

    Ok(Recipe {
        kind,
        source: RecipeSource::Heuristic,
        python: None,
        node: Some(node),
        install,
        run,
        web,
        env_vars: Vec::new(),
        system_tools: Vec::new(),
    })
}

type Plan = (AppKind, RunSpec, Option<WebSpec>);

fn electron(info: &NodeInfo, pm: PackageManager, app_dir: &str) -> Option<Plan> {
    if !info.has(NodeFramework::Electron) {
        return None;
    }
    let script = ["start", "dev"]
        .into_iter()
        .find(|s| info.scripts.contains_key(*s))?;
    Some((AppKind::Gui, run_in(app_dir, pm_run(pm, script)), None))
}

/// Flags that make a framework's dev server use our port and stay on this computer.
fn framework_flags(bin: &str) -> Option<(&'static [&'static str], u16)> {
    Some(match bin {
        "next" => (&["-p", "{port}", "-H", "{host}"], 3000),
        "vite" => (
            &["--port", "{port}", "--strictPort", "--host", "{host}"],
            5173,
        ),
        "astro" => (&["--port", "{port}", "--host", "{host}"], 4321),
        "nuxt" | "nuxi" => (&["--port", "{port}", "--host", "{host}"], 3000),
        "gatsby" => (&["-p", "{port}", "-H", "{host}"], 8000),
        "remix" | "react-router" => (&["--port", "{port}", "--host", "{host}"], 5173),
        _ => return None,
    })
}

fn web_app(info: &NodeInfo, pm: PackageManager, app_dir: &str) -> Option<Plan> {
    let webish = [
        NodeFramework::Next,
        NodeFramework::Vite,
        NodeFramework::Astro,
        NodeFramework::Nuxt,
        NodeFramework::SvelteKit,
        NodeFramework::Remix,
        NodeFramework::Gatsby,
        NodeFramework::CreateReactApp,
        NodeFramework::Express,
    ];
    if !webish.iter().any(|fw| info.has(*fw)) {
        return None;
    }
    let script_name = ["dev", "develop", "start", "serve"]
        .into_iter()
        .find(|s| info.scripts.contains_key(*s))?;
    let script = &info.scripts[script_name];
    let web = |port: u16, discover: bool| {
        Some(WebSpec {
            preferred_port: port,
            path: "/".into(),
            ready_timeout_secs: READY_TIMEOUT_SECS,
            discover,
        })
    };

    if let Some(simple) = shell_words::split(script) {
        let bin = simple.argv[0].as_str();
        let mut env: BTreeMap<String, String> = simple.env.into_iter().collect();
        if let Some((flags, port)) = framework_flags(bin) {
            // Run the framework binary directly so our flags reach it unchanged, whatever the
            // package manager's argument-forwarding rules are.
            let mut argv = simple.argv.clone();
            argv.extend(flags.iter().map(|f| f.to_string()));
            let mut run = run_in(app_dir, argv);
            run.env.append(&mut env);
            return Some((AppKind::Web, run, web(port, false)));
        }
        if bin == "react-scripts" {
            let mut run = run_in(app_dir, simple.argv.clone());
            env.insert("PORT".into(), "{port}".into());
            env.insert("HOST".into(), "{host}".into());
            run.env = env;
            return Some((AppKind::Web, run, web(3000, false)));
        }
    }

    // Anything else: run the script as written, offer a port, and watch for the real address.
    let mut run = run_in(app_dir, pm_run(pm, script_name));
    run.env.insert("PORT".into(), "{port}".into());
    Some((AppKind::Web, run, web(3000, true)))
}

pub(super) fn install_step(pm: PackageManager, info: &NodeInfo) -> Step {
    let locked = info.lockfile.is_some();
    let berry = info
        .package_manager_version
        .as_deref()
        .is_some_and(|v| !v.starts_with('1'));
    let words = |s: &str| s.split(' ').map(str::to_owned).collect::<Vec<_>>();
    let (argv, fallbacks): (&str, &[&str]) = match (pm, locked) {
        (PackageManager::Npm, true) => {
            ("npm ci", &["npm install", "npm install --legacy-peer-deps"])
        }
        (PackageManager::Npm, false) => ("npm install", &["npm install --legacy-peer-deps"]),
        (PackageManager::Pnpm, true) => (
            "pnpm install --frozen-lockfile",
            &["pnpm install --no-frozen-lockfile"],
        ),
        (PackageManager::Pnpm, false) => ("pnpm install", &[]),
        (PackageManager::Yarn, true) if berry => ("yarn install --immutable", &["yarn install"]),
        (PackageManager::Yarn, true) => ("yarn install --frozen-lockfile", &["yarn install"]),
        (PackageManager::Yarn, false) => ("yarn install", &[]),
        (PackageManager::Bun, true) => ("bun install --frozen-lockfile", &["bun install"]),
        (PackageManager::Bun, false) => ("bun install", &[]),
    };
    let from = info
        .lockfile
        .clone()
        .unwrap_or_else(|| "package.json".into());
    let mut step = Step::new(StepLabel::InstallHelpers { from }, words(argv));
    step.fallbacks = fallbacks.iter().map(|f| words(f)).collect();
    if berry {
        step.env
            .insert("YARN_ENABLE_IMMUTABLE_INSTALLS".into(), "false".into());
    }
    step
}

fn pm_run(pm: PackageManager, script: &str) -> Vec<String> {
    let pm = match pm {
        PackageManager::Npm => "npm",
        PackageManager::Pnpm => "pnpm",
        PackageManager::Yarn => "yarn",
        PackageManager::Bun => "bun",
    };
    vec![pm.into(), "run".into(), script.into()]
}

fn run_in(dir: &str, argv: Vec<String>) -> RunSpec {
    RunSpec {
        argv,
        cwd: dir.to_owned(),
        env: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(scripts: &[(&str, &str)], deps: &[&str]) -> NodeInfo {
        let mut i = NodeInfo {
            scripts: scripts
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            lockfile: Some("package-lock.json".into()),
            package_manager: Some(PackageManager::Npm),
            ..Default::default()
        };
        let fws = [
            ("next", NodeFramework::Next),
            ("vite", NodeFramework::Vite),
            ("react-scripts", NodeFramework::CreateReactApp),
            ("express", NodeFramework::Express),
            ("electron", NodeFramework::Electron),
        ];
        i.frameworks = fws
            .iter()
            .filter(|(d, _)| deps.contains(d))
            .map(|(_, f)| *f)
            .collect();
        i
    }

    #[test]
    fn vite_runs_directly_with_our_port() {
        let r = recipe(&info(&[("dev", "vite")], &["vite"])).unwrap();
        assert_eq!(r.kind, AppKind::Web);
        assert_eq!(
            r.run.argv,
            [
                "vite",
                "--port",
                "{port}",
                "--strictPort",
                "--host",
                "{host}"
            ]
        );
        assert_eq!(r.web.unwrap().preferred_port, 5173);
        assert_eq!(r.install[0].argv, ["npm", "ci"]);
        assert_eq!(r.install[0].fallbacks.len(), 2);
        assert_eq!(r.node.unwrap().version, "24.21.0");
    }

    #[test]
    fn next_keeps_script_flags_and_env() {
        let r = recipe(&info(
            &[(
                "dev",
                "NODE_OPTIONS=--max-old-space-size=4096 next dev --turbopack",
            )],
            &["next"],
        ))
        .unwrap();
        assert_eq!(r.run.argv[..3], ["next", "dev", "--turbopack"]);
        assert!(r.run.argv.contains(&"-H".to_string()));
        assert_eq!(
            r.run.env.get("NODE_OPTIONS").map(String::as_str),
            Some("--max-old-space-size=4096")
        );
    }

    #[test]
    fn complex_scripts_run_through_the_package_manager_and_discover_the_port() {
        let r = recipe(&info(
            &[("start", "node build.js && node server.js")],
            &["express"],
        ))
        .unwrap();
        assert_eq!(r.run.argv, ["npm", "run", "start"]);
        assert_eq!(r.run.env.get("PORT").map(String::as_str), Some("{port}"));
        assert!(r.web.unwrap().discover);
    }

    #[test]
    fn cli_with_build_step() {
        let mut i = info(&[("build", "tsc")], &[]);
        i.bins = vec![("tool".into(), "./dist/cli.js".into())];
        let r = recipe(&i).unwrap();
        assert_eq!(r.kind, AppKind::Terminal);
        assert_eq!(r.run.argv, ["node", "dist/cli.js"]);
        assert_eq!(r.install.last().unwrap().argv, ["npm", "run", "build"]);
    }

    #[test]
    fn electron_is_a_desktop_app() {
        let r = recipe(&info(&[("start", "electron .")], &["electron"])).unwrap();
        assert_eq!(r.kind, AppKind::Gui);
    }

    #[test]
    fn nothing_to_run() {
        let r = recipe(&info(&[("test", "jest")], &["lodash"]));
        assert_eq!(r.unwrap_err().code, ProblemCode::NoEntryPoint);
    }
}
