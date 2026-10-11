//! `rl`: a developer/test front-end for the Repo Launcher engine.
//!
//! Set `RL_HOME` to use a separate data folder (tests do this).

mod corpus;

use launcher_core::events::{EngineEvent, RunState};
use launcher_core::{AppPaths, Engine, EventSink, Problem};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const USAGE: &str = "usage:
  rl add <github-link | ./local/folder>
  rl list
  rl detect <folder>
  rl review <id>            download and show what installing would do
  rl install <id> [--yes]   --yes: agree to the plan shown by review
  rl run <id> [--smoke]     --smoke: start, check it responds, stop
  rl keys <id>              list the settings a project asks for
  rl set-key <id> <NAME>    read a value from stdin into the keychain (empty clears it)
  rl uninstall <id>
  rl shortcut <id> [--exe <app>]   create (or repair) shortcuts that open it with the app
  rl unshortcut <id>
  rl remove-shortcuts       delete every shortcut Repo Launcher made
  rl runs <id>              the newest run: its address and whether it is still running
  rl github status|login|logout|repos
  rl config github-app <client-id> [app-slug]
  rl forget-secrets         delete every keychain entry Repo Launcher made
  rl corpus-sync <corpus.toml> <out-dir>   (development: refresh test snapshots)";

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args).await {
        Ok(code) => code,
        Err(p) => {
            eprintln!("problem: {p}");
            if let Ok(json) = serde_json::to_string_pretty(&p) {
                eprintln!("{json}");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run(args: &[String]) -> Result<ExitCode, Problem> {
    let arg = |i: usize| args.get(i).map(String::as_str);
    if let (Some("corpus-sync"), Some(list), Some(out)) = (arg(0), arg(1), arg(2)) {
        corpus::sync(std::path::Path::new(list), std::path::Path::new(out)).await?;
        return Ok(ExitCode::SUCCESS);
    }
    if arg(0) == Some("detect") {
        let dir = arg(1).ok_or_else(usage)?;
        let tree = launcher_core::detect::FileTree::scan(std::path::Path::new(dir))?;
        let report = launcher_core::detect::detect(&tree);
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
        match launcher_core::recipe::resolve(&tree, &report).map(|(r, _)| r) {
            Ok(r) => println!("{}", serde_json::to_string_pretty(&r).unwrap_or_default()),
            Err(p) => println!("no recipe: {p}"),
        }
        return Ok(ExitCode::SUCCESS);
    }

    let paths = match std::env::var_os("RL_HOME") {
        Some(home) => AppPaths::at(home),
        None => AppPaths::for_current_user().ok_or_else(|| Problem::internal("no data folder"))?,
    };
    let engine = Engine::open(paths)?;

    match (arg(0), arg(1)) {
        (Some("add"), Some(input)) => {
            let p = engine.add(input).await?;
            println!("{}  {}  ({})", p.id, p.name, p.source);
        }
        (Some("list"), None) => {
            for p in engine.list()? {
                println!("{}  {:<28} {:?}  {}", p.id, p.name, p.status, p.source);
            }
        }
        (Some("review"), Some(id)) => {
            let report = engine
                .review(id, printer(), CancellationToken::new())
                .await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_default()
            );
        }
        (Some("install"), Some(id)) => {
            let cancel = CancellationToken::new();
            let c = cancel.clone();
            tokio::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    c.cancel();
                }
            });
            let report = engine.review(id, printer(), cancel.clone()).await?;
            if !report.consented {
                for s in &report.signals {
                    eprintln!("[{:?}] {:?} {:?}", s.level, s.code, s.params);
                }
                for c in &report.commands {
                    eprintln!("  will run: {c}");
                }
                if !args.iter().any(|a| a == "--yes") {
                    eprintln!("review the plan above, then run again with --yes to agree");
                    return Ok(ExitCode::FAILURE);
                }
                engine.consent(id, &report.fingerprint, false)?;
            }
            let p = engine.install(id, printer(), cancel).await?;
            println!("installed: {} ({:?})", p.name, p.kind);
        }
        (Some("keys"), Some(id)) => {
            for v in engine.env_vars(id)? {
                let state = match (&v.value, v.has_value) {
                    (Some(value), _) => value.clone(),
                    (None, true) => "(saved in keychain)".into(),
                    (None, false) => "(not set)".into(),
                };
                let need = if v.required { "required" } else { "optional" };
                println!(
                    "{:<28} {need:<9} {state}  {}",
                    v.name,
                    v.help_url.unwrap_or_default()
                );
            }
        }
        (Some("set-key"), Some(id)) => {
            let name = arg(2).ok_or_else(usage)?;
            let mut value = String::new();
            std::io::stdin().read_line(&mut value)?;
            let value = value.trim();
            engine.set_env_value(id, name, (!value.is_empty()).then_some(value))?;
            println!("saved {name}");
        }
        (Some("github"), Some(cmd)) => return github(&engine, cmd).await,
        (Some("config"), Some("github-app")) => {
            let client_id = arg(2).ok_or_else(usage)?;
            engine.set_github_app(Some(client_id), arg(3))?;
            println!("GitHub App client ID saved");
        }
        (Some("forget-secrets"), None) => {
            let n = engine.forget_all_secrets()?;
            println!("deleted {n} keychain entries");
        }
        (Some("run"), Some(id)) => {
            return run_project(&engine, id, arg(2) == Some("--smoke")).await;
        }
        (Some("shortcut"), Some(id)) => {
            let exe = match args.iter().position(|a| a == "--exe") {
                Some(i) => std::path::PathBuf::from(arg(i + 1).ok_or_else(usage)?),
                // During development the app is built next to `rl`.
                None => std::env::current_exe()?
                    .with_file_name(format!("repo-launcher{}", std::env::consts::EXE_SUFFIX)),
            };
            let launcher = launcher_core::shortcuts::Launcher {
                exe: std::path::absolute(exe)?,
                ..Default::default()
            };
            for s in engine.create_shortcuts(id, &launcher)? {
                println!("{:?}  {}", s.kind, s.path);
            }
        }
        (Some("unshortcut"), Some(id)) => {
            engine.remove_shortcuts(id)?;
            println!("removed the shortcuts of {id}");
        }
        (Some("remove-shortcuts"), None) => {
            let n = engine.remove_all_shortcuts()?;
            println!("removed {n} shortcuts");
        }
        (Some("runs"), Some(id)) => match engine.latest_run(id)? {
            Some((url, open)) => println!(
                "{} {}",
                if open { "running" } else { "ended" },
                url.unwrap_or_default()
            ),
            None => println!("never run"),
        },
        (Some("uninstall"), Some(id)) => {
            engine.uninstall(id).await?;
            println!("removed {id}");
        }
        _ => return Err(usage()),
    }
    Ok(ExitCode::SUCCESS)
}

async fn github(engine: &Engine, cmd: &str) -> Result<ExitCode, Problem> {
    match cmd {
        "status" => match engine.github_account() {
            Some(a) => println!("connected as @{}", a.login),
            None => println!("not connected"),
        },
        "login" => {
            let code = engine.github_begin_sign_in().await?;
            println!(
                "Open {} and enter the code {}",
                code.verification_uri, code.user_code
            );
            let account = engine
                .github_finish_sign_in(CancellationToken::new())
                .await?;
            println!("connected as @{}", account.login);
        }
        "logout" => {
            let url = engine.github_sign_out()?;
            println!("signed out; to revoke access entirely, visit {url}");
        }
        "repos" => {
            for r in engine.github_repositories().await? {
                let vis = if r.private { "private" } else { "public" };
                println!("{:<40} {vis}", r.full_name);
            }
        }
        _ => return Err(usage()),
    }
    Ok(ExitCode::SUCCESS)
}

async fn run_project(engine: &Engine, id: &str, smoke: bool) -> Result<ExitCode, Problem> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(RunState, Option<i32>)>();
    let print = printer();
    let sink: EventSink = Arc::new(move |e| {
        if let EngineEvent::RunState {
            state, exit_code, ..
        } = &e
        {
            let _ = tx.send((*state, *exit_code));
        }
        print(e);
    });
    let info = engine.launch(id, sink).await?;
    if let Some(url) = &info.url {
        println!("ready at {url}");
        if smoke {
            // Launch already waited for readiness; prove it with one more request, then stop.
            let ok = accepts_connections(url);
            engine.stop(id).await;
            println!("smoke: {}", if ok { "ok" } else { "no response" });
            return Ok(if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            });
        }
    }
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                engine.stop(id).await;
                return Ok(ExitCode::SUCCESS);
            }
            msg = rx.recv() => match msg {
                Some((RunState::Stopped, _)) => return Ok(ExitCode::SUCCESS),
                Some((RunState::Crashed, code)) => {
                    eprintln!("exited with code {code:?}");
                    return Ok(ExitCode::FAILURE);
                }
                Some(_) => {}
                None => return Ok(ExitCode::SUCCESS),
            },
        }
    }
}

fn accepts_connections(url: &str) -> bool {
    let host = url
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or_default();
    host.parse().is_ok_and(|addr| {
        std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(3)).is_ok()
    })
}

fn printer() -> EventSink {
    Arc::new(|e| match e {
        EngineEvent::Progress {
            stage,
            fraction,
            label,
            ..
        } => {
            let label = label
                .map(|l| format!(" {}", serde_json::to_string(&l).unwrap_or_default()))
                .unwrap_or_default();
            eprintln!("[{:>3.0}%] {stage:?}{label}", fraction * 100.0);
        }
        EngineEvent::Log { line, .. } => eprintln!("  | {line}"),
        // Console output also arrives as plain log lines; the raw stream is for the GUI.
        EngineEvent::Console { .. } => {}
        EngineEvent::RunState {
            state,
            url,
            exit_code,
            ..
        } => {
            eprintln!(
                "[state] {state:?} {} {}",
                url.unwrap_or_default(),
                exit_code.map(|c| c.to_string()).unwrap_or_default()
            );
        }
    })
}

fn usage() -> Problem {
    eprintln!("{USAGE}");
    Problem::new(launcher_core::ProblemCode::InvalidLink).detail("bad arguments")
}
