//! Launching and supervising running projects.

use crate::diagnose;
use crate::env::project_env;
use crate::events::{EngineEvent, EventSink, LogStream, RunState};
use crate::install::{ProjectLog, expand, spawn_pumps};
use crate::paths::{AppPaths, ProjectPaths};
use crate::problem::{FixAction, Problem, ProblemCode, Result};
use crate::proc;
use crate::recipe::{AppKind, Recipe};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

const LOG_HISTORY: usize = 2000;
const STOP_GRACE: Duration = Duration::from_secs(5);

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchInfo {
    pub url: Option<String>,
    pub port: Option<u16>,
}

struct Running {
    stop: CancellationToken,
    done: watch::Receiver<Option<Option<i32>>>,
    history: Arc<Mutex<VecDeque<String>>>,
    url: Option<String>,
}

#[derive(Default, Clone)]
pub struct Supervisor {
    running: Arc<Mutex<HashMap<String, Running>>>,
}

pub struct LaunchRequest<'a> {
    pub app: &'a AppPaths,
    pub project: &'a ProjectPaths,
    pub root: &'a Path,
    pub project_id: &'a str,
    pub recipe: &'a Recipe,
    pub uv: &'a Path,
    pub sink: EventSink,
}

impl Supervisor {
    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<String, Running>> {
        self.running.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.map()
            .get(id)
            .is_some_and(|r| r.done.borrow().is_none())
    }

    pub fn url(&self, id: &str) -> Option<String> {
        self.map()
            .get(id)
            .filter(|r| r.done.borrow().is_none())
            .and_then(|r| r.url.clone())
    }

    pub fn running_ids(&self) -> Vec<String> {
        self.map()
            .iter()
            .filter(|(_, r)| r.done.borrow().is_none())
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn history(&self, id: &str) -> Vec<String> {
        self.map()
            .get(id)
            .and_then(|r| r.history.lock().ok().map(|h| h.iter().cloned().collect()))
            .unwrap_or_default()
    }

    pub async fn launch(&self, req: LaunchRequest<'_>) -> Result<LaunchInfo> {
        if self.is_running(req.project_id) {
            return Err(Problem::new(ProblemCode::AlreadyRunning));
        }
        let id = req.project_id.to_owned();
        let port = match &req.recipe.web {
            Some(web) => Some(pick_port(web.preferred_port)?),
            None => None,
        };
        let mut vars = vec![
            ("{env}", req.project.env().to_string_lossy().into_owned()),
            ("{src}", req.root.to_string_lossy().into_owned()),
            ("{host}", "127.0.0.1".to_owned()),
        ];
        if let Some(p) = port {
            vars.push(("{port}", p.to_string()));
        }
        let run = &req.recipe.run;
        let argv: Vec<String> = run.argv.iter().map(|a| expand(a, &vars)).collect();
        let (program, args) = argv
            .split_first()
            .ok_or_else(|| Problem::internal("empty run command"))?;
        let program: PathBuf =
            proc::resolve_program(program, req.root, &req.project.venv_bin(), req.uv);
        let extra: BTreeMap<String, String> = run
            .env
            .iter()
            .map(|(k, v)| (k.clone(), expand(v, &vars)))
            .collect();
        let env = project_env(req.app, req.project, &extra);
        let cwd = req.root.join(&run.cwd);

        let log = ProjectLog::create(&id, &req.project.logs().join("run.log"), req.sink.clone());
        log.line(LogStream::Engine, format!("$ {}", argv.join(" ")));
        let mut child = proc::spawn(proc::Spec {
            program: &program,
            args,
            cwd: &cwd,
            env: &env,
        })
        .map_err(|e| {
            Problem::new(ProblemCode::LaunchFailed)
                .fix(FixAction::Repair)
                .detail(format!("{}: {e}", program.display()))
        })?;

        let history = Arc::new(Mutex::new(VecDeque::with_capacity(LOG_HISTORY)));
        let pumps = spawn_pumps(&mut child, &log, &history, LOG_HISTORY);

        let stop = CancellationToken::new();
        let (done_tx, done_rx) = watch::channel(None);
        let url = port.map(|p| {
            let path = req
                .recipe
                .web
                .as_ref()
                .map(|w| w.path.as_str())
                .unwrap_or("/");
            format!("http://127.0.0.1:{p}{path}")
        });

        self.map().insert(
            id.clone(),
            Running {
                stop: stop.clone(),
                done: done_rx.clone(),
                history: history.clone(),
                url: url.clone(),
            },
        );
        emit(&req.sink, &id, RunState::Starting, None, None);

        // The monitor task owns the child until it exits or is asked to stop.
        let sink = req.sink.clone();
        let monitor_id = id.clone();
        tokio::spawn(async move {
            let (code, by_user) = tokio::select! {
                s = child.wait() => (s.ok().and_then(|s| s.code()), false),
                _ = stop.cancelled() => {
                    let _ = proc::stop_tree(&mut child, STOP_GRACE).await;
                    (None, true)
                }
            };
            for p in pumps {
                let _ = tokio::time::timeout(Duration::from_secs(2), p).await;
            }
            let state = if by_user || code == Some(0) {
                RunState::Stopped
            } else {
                RunState::Crashed
            };
            let _ = done_tx.send(Some(code));
            emit(&sink, &monitor_id, state, None, code);
        });

        match (req.recipe.kind, &url) {
            (AppKind::Web, Some(url)) => {
                let timeout = Duration::from_secs(
                    req.recipe
                        .web
                        .as_ref()
                        .map_or(180, |w| w.ready_timeout_secs),
                );
                self.wait_ready(&id, url, timeout, done_rx, &history)
                    .await?;
                emit(&req.sink, &id, RunState::Running, Some(url.clone()), None);
            }
            _ => emit(&req.sink, &id, RunState::Running, None, None),
        }
        Ok(LaunchInfo { url, port })
    }

    async fn wait_ready(
        &self,
        id: &str,
        url: &str,
        timeout: Duration,
        mut done: watch::Receiver<Option<Option<i32>>>,
        history: &Arc<Mutex<VecDeque<String>>>,
    ) -> Result<()> {
        // Never route a localhost probe through a proxy.
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(Problem::internal)?;
        let start = Instant::now();
        let mut delay = Duration::from_millis(250);
        loop {
            if let Some(code) = *done.borrow() {
                let tail: Vec<String> = history
                    .lock()
                    .map(|h| h.iter().rev().take(60).rev().cloned().collect())
                    .unwrap_or_default();
                return Err(diagnose::launch_failure(&tail, code));
            }
            if let Ok(resp) = http.get(url).send().await
                && resp.status().as_u16() < 500
            {
                return Ok(());
            }
            if start.elapsed() > timeout {
                return Err(Problem::new(ProblemCode::StartTimeout)
                    .param("id", id)
                    .fix(FixAction::Retry));
            }
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = done.changed() => {}
            }
            delay = (delay * 2).min(Duration::from_secs(2));
        }
    }

    /// Stops a project's whole process tree. Returns once it has exited.
    pub async fn stop(&self, id: &str) {
        let entry = self.map().get(id).map(|r| (r.stop.clone(), r.done.clone()));
        if let Some((stop, mut done)) = entry {
            stop.cancel();
            let _ = tokio::time::timeout(STOP_GRACE * 2, done.wait_for(Option::is_some)).await;
        }
    }

    pub async fn stop_all(&self) {
        for id in self.running_ids() {
            self.stop(&id).await;
        }
    }
}

fn emit(sink: &EventSink, id: &str, state: RunState, url: Option<String>, exit_code: Option<i32>) {
    sink(EngineEvent::RunState {
        project_id: id.to_owned(),
        state,
        url,
        exit_code,
    });
}

/// The preferred port if it's free, otherwise any free port.
pub fn pick_port(preferred: u16) -> Result<u16> {
    if is_free(preferred) {
        return Ok(preferred);
    }
    for _ in 0..20 {
        let port = std::net::TcpListener::bind(("127.0.0.1", 0))?
            .local_addr()?
            .port();
        if is_free(port) {
            return Ok(port);
        }
    }
    Err(Problem::internal("no free port"))
}

/// A localhost bind can succeed while another program holds the same port on all interfaces
/// (macOS AirPlay Receiver owns *:5000), and then our readiness probe would talk to the wrong
/// server. Requiring both binds to succeed avoids that.
fn is_free(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
        && std::net::TcpListener::bind(("0.0.0.0", port)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_another_port_when_taken() {
        let taken = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = taken.local_addr().unwrap().port();
        let picked = pick_port(port).unwrap();
        assert_ne!(picked, port);
    }

    #[test]
    fn port_held_on_all_interfaces_is_not_free() {
        let taken = std::net::TcpListener::bind(("0.0.0.0", 0)).unwrap();
        let port = taken.local_addr().unwrap().port();
        assert!(!is_free(port));
    }
}
