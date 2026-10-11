//! Runs a recipe's install steps with progress, logging and cancellation.

use crate::diagnose;
use crate::env::project_env;
use crate::events::{EngineEvent, EventSink, LogStream, Stage};
use crate::paths::{AppPaths, ProjectPaths};
use crate::problem::{Problem, ProblemCode, Result};
use crate::proc;
use crate::recipe::{Recipe, Step, StepLabel};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const TAIL_LINES: usize = 80;

/// Appends lines to a log file and forwards them to the UI.
#[derive(Clone)]
pub struct ProjectLog {
    project_id: String,
    file: Arc<Mutex<Option<std::fs::File>>>,
    sink: EventSink,
}

impl ProjectLog {
    pub fn create(project_id: &str, path: &Path, sink: EventSink) -> Self {
        let file = path
            .parent()
            .and_then(|d| std::fs::create_dir_all(d).ok())
            .and_then(|_| {
                std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(path)
                    .ok()
            });
        Self {
            project_id: project_id.to_owned(),
            file: Arc::new(Mutex::new(file)),
            sink,
        }
    }

    pub fn line(&self, stream: LogStream, line: String) {
        if let Ok(mut f) = self.file.lock()
            && let Some(f) = f.as_mut()
        {
            let tag = match stream {
                LogStream::Stdout => "out",
                LogStream::Stderr => "err",
                LogStream::Engine => "launcher",
            };
            let _ = writeln!(f, "[{tag}] {line}");
        }
        (self.sink)(EngineEvent::Log {
            project_id: self.project_id.clone(),
            stream,
            line,
        });
    }
}

pub struct Context<'a> {
    pub app: &'a AppPaths,
    pub project: &'a ProjectPaths,
    /// The folder the recipe's relative paths start from (source root, or a subfolder of it).
    pub root: &'a Path,
    pub project_id: &'a str,
    pub sink: &'a EventSink,
    pub log: &'a ProjectLog,
    pub cancel: &'a CancellationToken,
    pub tools: &'a crate::runtime::Tools,
    /// Identifies the source being installed (commit id). Part of every step fingerprint, so an
    /// update re-runs the steps but an interrupted install of the same commit resumes.
    pub source_id: &'a str,
    /// Last emitted (stage, fraction), to drop progress events that change nothing visible.
    pub last_progress: Mutex<Option<(Stage, f32)>>,
}

impl Context<'_> {
    pub fn progress(&self, stage: Stage, fraction: f32, label: Option<StepLabel>) {
        if let Ok(mut last) = self.last_progress.lock() {
            let unchanged =
                matches!(*last, Some((s, f)) if s == stage && (fraction - f).abs() < 0.005);
            if unchanged && label.is_none() {
                return;
            }
            *last = Some((stage, fraction));
        }
        (self.sink)(EngineEvent::Progress {
            project_id: self.project_id.to_owned(),
            stage,
            fraction: fraction.clamp(0.0, 1.0),
            label,
        });
    }

    pub fn vars(&self) -> Vec<(&'static str, String)> {
        vec![
            ("{env}", self.project.env().to_string_lossy().into_owned()),
            ("{src}", self.root.to_string_lossy().into_owned()),
            ("{host}", "127.0.0.1".into()),
        ]
    }
}

pub fn expand(s: &str, vars: &[(&str, String)]) -> String {
    vars.iter()
        .fold(s.to_owned(), |acc, (k, v)| acc.replace(k, v))
}

fn stage_of(label: &StepLabel) -> Stage {
    match label {
        StepLabel::SetUpPython { .. }
        | StepLabel::SetUpNode { .. }
        | StepLabel::CreateEnvironment => Stage::Preparing,
        StepLabel::InstallHelpers { .. } | StepLabel::BuildProject | StepLabel::Custom { .. } => {
            Stage::InstallingHelpers
        }
    }
}

/// Fingerprints of steps that already succeeded, kept in the project's meta folder.
struct StepRecord {
    path: PathBuf,
    done: BTreeSet<String>,
}

impl StepRecord {
    fn load(project: &ProjectPaths) -> Self {
        let path = project.meta().join("steps.json");
        let done = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self { path, done }
    }

    fn mark(&mut self, fingerprint: String) {
        self.done.insert(fingerprint);
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec(&self.done) {
            let _ = std::fs::write(&self.path, json);
        }
    }
}

/// Forgets completed steps so the next install runs everything again (used by Repair).
pub fn forget_steps(project: &ProjectPaths) {
    let _ = std::fs::remove_file(project.meta().join("steps.json"));
}

fn fingerprint(step: &Step, source_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let json = serde_json::to_vec(step).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(source_id.as_bytes());
    h.update(json);
    hex::encode(h.finalize())
}

/// Install progress spans 0.30..0.95; downloading the source and runtimes covers the rest.
pub async fn run_steps(ctx: &Context<'_>, recipe: &Recipe) -> Result<()> {
    let steps: Vec<&Step> = recipe.install.iter().filter(|s| s.applies_here()).collect();
    let n = steps.len().max(1) as f32;
    let mut record = StepRecord::load(ctx.project);
    for (i, step) in steps.into_iter().enumerate() {
        let fraction = 0.30 + 0.65 * (i as f32 / n);
        ctx.progress(stage_of(&step.label), fraction, Some(step.label.clone()));
        let fp = fingerprint(step, ctx.source_id);
        if record.done.contains(&fp) {
            ctx.log.line(
                LogStream::Engine,
                format!("already done: {}", step.argv.join(" ")),
            );
            continue;
        }
        let mut result = run_command(ctx, step, &step.argv).await;
        for fallback in &step.fallbacks {
            match &result {
                Err(p) if p.code != ProblemCode::Cancelled => {
                    ctx.log.line(
                        LogStream::Engine,
                        "that didn't work; trying another way".into(),
                    );
                    result = run_command(ctx, step, fallback).await;
                }
                _ => break,
            }
        }
        result?;
        record.mark(fp);
    }
    Ok(())
}

async fn run_command(ctx: &Context<'_>, step: &Step, argv: &[String]) -> Result<()> {
    let vars = ctx.vars();
    let argv = proc::normalize_argv(argv.iter().map(|a| expand(a, &vars)).collect());
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| Problem::internal("empty step"))?;
    let program = proc::resolve_program(program, ctx.root, ctx.tools);
    let cwd: PathBuf = ctx.root.join(&step.cwd);
    let extra: BTreeMap<String, String> = step
        .env
        .iter()
        .map(|(k, v)| (k.clone(), expand(v, &vars)))
        .collect();
    let env = project_env(ctx.app, ctx.project, &ctx.tools.path, &extra);

    ctx.log
        .line(LogStream::Engine, format!("$ {}", argv.join(" ")));
    let mut child = proc::spawn(proc::Spec {
        program: &program,
        args,
        cwd: &cwd,
        env: &env,
    })
    .map_err(|e| {
        Problem::new(ProblemCode::InstallStepFailed).detail(format!("{}: {e}", program.display()))
    })?;

    let tail = Arc::new(Mutex::new(VecDeque::with_capacity(TAIL_LINES)));
    let pumps = spawn_pumps(&mut child, ctx.log, &tail, TAIL_LINES);

    let status = tokio::select! {
        s = child.wait() => s?,
        _ = ctx.cancel.cancelled() => {
            let _ = proc::stop_tree(&mut child, Duration::from_secs(3)).await;
            return Err(Problem::new(ProblemCode::Cancelled));
        }
    };
    // A lingering grandchild can hold the pipes open; don't wait on it forever.
    for p in pumps {
        let _ = tokio::time::timeout(Duration::from_secs(2), p).await;
    }
    if status.success() {
        return Ok(());
    }
    let tail: Vec<String> = tail
        .lock()
        .map(|t| t.iter().cloned().collect())
        .unwrap_or_default();
    Err(diagnose::install_failure(&tail, status.code()))
}

/// Forwards stdout/stderr to the log while keeping the last lines for diagnosis.
pub fn spawn_pumps(
    child: &mut Box<dyn process_wrap::tokio::ChildWrapper>,
    log: &ProjectLog,
    tail: &Arc<Mutex<VecDeque<String>>>,
    cap: usize,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();
    let make = |stream: LogStream| {
        let log = log.clone();
        let tail = tail.clone();
        move |line: String| {
            if let Ok(mut t) = tail.lock() {
                if t.len() >= cap {
                    t.pop_front();
                }
                t.push_back(line.clone());
            }
            log.line(stream, line);
        }
    };
    if let Some(out) = child.stdout().take() {
        let f = make(LogStream::Stdout);
        handles.push(tokio::spawn(proc::pump(out, f)));
    }
    if let Some(err) = child.stderr().take() {
        let f = make(LogStream::Stderr);
        handles.push(tokio::spawn(proc::pump(err, f)));
    }
    handles
}
