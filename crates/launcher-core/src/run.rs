//! Launching and supervising running projects.

use crate::console;
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
    /// The address that answered, once known.
    url: Arc<Mutex<Option<String>>>,
    console: Option<Arc<console::Console>>,
}

/// A running project process: piped (web/desktop apps) or in a pseudo-terminal (terminal apps).
enum Child {
    Piped(
        Box<dyn process_wrap::tokio::ChildWrapper>,
        Vec<tokio::task::JoinHandle<()>>,
    ),
    Pty(console::PtyChild),
}

impl Child {
    async fn wait(&mut self) -> Option<i32> {
        match self {
            Child::Piped(c, _) => c.wait().await.ok().and_then(|s| s.code()),
            Child::Pty(p) => p.wait().await,
        }
    }

    async fn stop_tree(&mut self) {
        match self {
            Child::Piped(c, _) => {
                let _ = proc::stop_tree(c, STOP_GRACE).await;
            }
            Child::Pty(p) => p.stop_tree(STOP_GRACE).await,
        }
    }

    /// Lets output readers finish, without waiting forever on a lingering grandchild.
    async fn drain(&mut self) {
        match self {
            Child::Piped(_, pumps) => {
                for p in pumps.drain(..) {
                    let _ = tokio::time::timeout(Duration::from_secs(2), p).await;
                }
            }
            Child::Pty(p) => p.drain().await,
        }
    }
}

type Urls = Arc<Mutex<Vec<String>>>;

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
    pub tools: &'a crate::runtime::Tools,
    /// Appended to the recipe's command, e.g. options typed into the console.
    pub extra_args: &'a [String],
    /// The user's settings for this project (API keys etc.), injected into this run only.
    pub user_env: &'a BTreeMap<String, String>,
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
            .and_then(|r| r.url.lock().ok().and_then(|u| u.clone()))
    }

    pub fn running_ids(&self) -> Vec<String> {
        self.map()
            .iter()
            .filter(|(_, r)| r.done.borrow().is_none())
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn console(&self, id: &str) -> Option<Arc<console::Console>> {
        self.map().get(id).and_then(|r| r.console.clone())
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
        vars.push(("{token}", uuid::Uuid::new_v4().simple().to_string()));
        let run = &req.recipe.run;
        let argv = proc::normalize_argv(
            run.argv
                .iter()
                .map(|a| expand(a, &vars))
                .chain(req.extra_args.iter().cloned())
                .collect(),
        );
        let (program, args) = argv
            .split_first()
            .ok_or_else(|| Problem::internal("empty run command"))?;
        let program: PathBuf = proc::resolve_program(program, req.root, req.tools);
        let mut extra: BTreeMap<String, String> = run
            .env
            .iter()
            .map(|(k, v)| (k.clone(), expand(v, &vars)))
            .collect();
        extra.extend(req.user_env.iter().map(|(k, v)| (k.clone(), v.clone())));
        let env = project_env(req.app, req.project, &req.tools.path, &extra);
        let cwd = req.root.join(&run.cwd);

        // Watch the app's own output for the address it says it's serving on.
        let seen: Urls = Arc::default();
        let history = Arc::new(Mutex::new(VecDeque::with_capacity(LOG_HISTORY)));
        let use_console = req.recipe.kind == AppKind::Terminal;
        let sniff = {
            let (seen, sink, hist) = (seen.clone(), req.sink.clone(), history.clone());
            let sink: EventSink = Arc::new(move |e: EngineEvent| {
                if let EngineEvent::Log { line, .. } = &e {
                    if let Ok(mut seen) = seen.lock() {
                        for u in local_urls(line) {
                            if !seen.contains(&u) {
                                seen.push(u);
                            }
                        }
                    }
                    // Piped output is recorded by its pumps; console output only passes here.
                    if use_console && let Ok(mut h) = hist.lock() {
                        if h.len() >= LOG_HISTORY {
                            h.pop_front();
                        }
                        h.push_back(line.clone());
                    }
                }
                sink(e)
            });
            sink
        };
        let log = ProjectLog::create(&id, &req.project.logs().join("run.log"), sniff);
        log.line(LogStream::Engine, format!("$ {}", argv.join(" ")));
        let launch_failed = |e: std::io::Error| {
            Problem::new(ProblemCode::LaunchFailed)
                .fix(FixAction::Repair)
                .detail(format!("{}: {e}", program.display()))
        };
        let (mut child, console) = if use_console {
            let spec = console::PtySpec {
                program: &program,
                args,
                cwd: &cwd,
                env: &env,
            };
            let (console, pty) =
                console::spawn(spec, &id, log.clone(), req.sink.clone()).map_err(launch_failed)?;
            (Child::Pty(pty), Some(console))
        } else {
            let mut c = proc::spawn(proc::Spec {
                program: &program,
                args,
                cwd: &cwd,
                env: &env,
            })
            .map_err(launch_failed)?;
            let pumps = spawn_pumps(&mut c, &log, &history, LOG_HISTORY);
            (Child::Piped(c, pumps), None)
        };

        let stop = CancellationToken::new();
        let (done_tx, done_rx) = watch::channel(None);
        let path = req
            .recipe
            .web
            .as_ref()
            .map(|w| expand(&w.path, &vars))
            .unwrap_or_else(|| "/".into());
        let offered = port.map(|p| format!("http://127.0.0.1:{p}{path}"));
        let url_slot: Arc<Mutex<Option<String>>> = Arc::default();

        self.map().insert(
            id.clone(),
            Running {
                stop: stop.clone(),
                done: done_rx.clone(),
                history: history.clone(),
                url: url_slot.clone(),
                console,
            },
        );
        emit(&req.sink, &id, RunState::Starting, None, None);

        // The monitor task owns the child until it exits or is asked to stop.
        let sink = req.sink.clone();
        let monitor_id = id.clone();
        tokio::spawn(async move {
            let (code, by_user) = tokio::select! {
                code = child.wait() => (code, false),
                _ = stop.cancelled() => {
                    child.stop_tree().await;
                    (None, true)
                }
            };
            child.drain().await;
            let state = if by_user || code == Some(0) {
                RunState::Stopped
            } else {
                RunState::Crashed
            };
            let _ = done_tx.send(Some(code));
            emit(&sink, &monitor_id, state, None, code);
        });

        let probe = Probe {
            offered: offered.clone(),
            seen,
            done: done_rx,
        };
        if req.recipe.kind == AppKind::Web {
            let timeout = Duration::from_secs(
                req.recipe
                    .web
                    .as_ref()
                    .map_or(180, |w| w.ready_timeout_secs),
            );
            let url = match probe.wait(timeout).await? {
                Some(url) => url,
                None => {
                    let tail = tail_of(&history);
                    let code = *probe.done.borrow();
                    return Err(match code {
                        Some(code) => diagnose::launch_failure(&tail, code),
                        None => Problem::new(ProblemCode::StartTimeout)
                            .param("id", &id)
                            .fix(FixAction::Retry),
                    });
                }
            };
            if let Ok(mut slot) = url_slot.lock() {
                *slot = Some(url.clone());
            }
            emit(&req.sink, &id, RunState::Running, Some(url.clone()), None);
            let port = url_port(&url).or(port);
            return Ok(LaunchInfo {
                url: Some(url),
                port,
            });
        }

        // Terminal and desktop apps are "running" right away. Some still start a web page;
        // if one answers, surface it so the user gets an Open button.
        emit(&req.sink, &id, RunState::Running, None, None);
        let (sink, id2) = (req.sink.clone(), id.clone());
        tokio::spawn(async move {
            if let Ok(Some(url)) = probe.wait(Duration::from_secs(600)).await {
                if let Ok(mut slot) = url_slot.lock() {
                    *slot = Some(url.clone());
                }
                emit(&sink, &id2, RunState::Running, Some(url), None);
            }
        });
        Ok(LaunchInfo {
            url: None,
            port: None,
        })
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

/// Waits for the app's address: the one we offered, or one it printed.
struct Probe {
    offered: Option<String>,
    seen: Urls,
    done: watch::Receiver<Option<Option<i32>>>,
}

impl Probe {
    /// `Ok(Some(url))` once an address answers, `Ok(None)` if the app exited or time ran out.
    async fn wait(&self, timeout: Duration) -> Result<Option<String>> {
        // Never route a localhost probe through a proxy.
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(Problem::internal)?;
        let start = Instant::now();
        let mut done = self.done.clone();
        let mut delay = Duration::from_millis(250);
        loop {
            if done.borrow().is_some() {
                return Ok(None);
            }
            let mut candidates: Vec<String> =
                self.seen.lock().map(|s| s.clone()).unwrap_or_default();
            // Prefer what the app printed: it may ignore the port we offered.
            candidates.extend(self.offered.clone());
            for url in candidates {
                // Any HTTP answer means the server is up. An error page (e.g. a database that
                // still needs setting up) tells the user more in the browser than a timeout would.
                if http.get(&url).send().await.is_ok() {
                    return Ok(Some(url));
                }
            }
            if start.elapsed() > timeout {
                return Ok(None);
            }
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = done.changed() => {}
            }
            delay = (delay * 2).min(Duration::from_secs(2));
        }
    }
}

fn tail_of(history: &Arc<Mutex<VecDeque<String>>>) -> Vec<String> {
    history
        .lock()
        .map(|h| h.iter().rev().take(60).rev().cloned().collect())
        .unwrap_or_default()
}

fn url_port(url: &str) -> Option<u16> {
    url::Url::parse(url).ok()?.port()
}

/// Local web addresses mentioned in a line of output, normalized for a browser.
pub fn local_urls(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, _) in line.match_indices("http") {
        let rest = &line[i..];
        let end = rest
            .find(|c: char| {
                c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | ']' | '`')
            })
            .unwrap_or(rest.len());
        let candidate = rest[..end].trim_end_matches(['.', ',', ';', ':']);
        let Ok(mut url) = url::Url::parse(candidate) else {
            continue;
        };
        if !matches!(url.scheme(), "http" | "https") {
            continue;
        }
        let host = url.host_str().unwrap_or_default().to_owned();
        let local = matches!(
            host.as_str(),
            "localhost" | "127.0.0.1" | "0.0.0.0" | "[::]" | "[::1]" | "::" | "::1"
        );
        if !local || url.port_or_known_default().is_none() {
            continue;
        }
        if host != "localhost" {
            let _ = url.set_host(Some("127.0.0.1"));
        }
        out.push(url.to_string());
    }
    out
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
    fn finds_local_urls_in_output() {
        assert_eq!(
            local_urls("  ➜  Local:   http://localhost:5173/"),
            ["http://localhost:5173/"]
        );
        assert_eq!(
            local_urls("Running on http://0.0.0.0:8000 (Press CTRL+C)"),
            ["http://127.0.0.1:8000/"]
        );
        assert_eq!(
            local_urls("open http://127.0.0.1:8888/lab?token=abc."),
            ["http://127.0.0.1:8888/lab?token=abc"]
        );
        assert!(local_urls("docs at https://example.com and http://192.168.1.4:3000").is_empty());
    }

    #[test]
    fn port_held_on_all_interfaces_is_not_free() {
        let taken = std::net::TcpListener::bind(("0.0.0.0", 0)).unwrap();
        let port = taken.local_addr().unwrap().port();
        assert!(!is_free(port));
    }
}
