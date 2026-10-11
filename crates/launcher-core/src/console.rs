//! Terminal programs run in a real pseudo-terminal so interactive prompts, colors and progress
//! bars work in the built-in console window.

use crate::events::{EngineEvent, EventSink, LogStream};
use crate::install::ProjectLog;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

const SCROLLBACK_BYTES: usize = 256 * 1024;

pub struct Console {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    scrollback: Mutex<VecDeque<u8>>,
}

impl Console {
    pub fn write(&self, data: &str) -> std::io::Result<()> {
        let mut w = self
            .writer
            .lock()
            .map_err(|_| std::io::Error::other("console poisoned"))?;
        w.write_all(data.as_bytes())?;
        w.flush()
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        if let Ok(m) = self.master.lock() {
            let _ = m.resize(PtySize {
                rows: rows.max(2),
                cols: cols.max(10),
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    /// Everything printed so far (bounded), so a reopened console can replay it.
    pub fn scrollback(&self) -> String {
        let bytes: Vec<u8> = self
            .scrollback
            .lock()
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

pub struct PtyChild {
    pid: Option<u32>,
    killer: Box<dyn portable_pty::ChildKiller + Send + Sync>,
    exited: oneshot::Receiver<Option<i32>>,
    /// Fires when the output reader has delivered everything the program printed.
    output_done: Option<oneshot::Receiver<()>>,
}

impl PtyChild {
    /// Waits briefly for the last output to be read, so it is delivered before the exit is
    /// reported. Bounded: some terminals only signal end-of-output when the console closes.
    pub async fn drain(&mut self) {
        if let Some(rx) = self.output_done.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), rx).await;
        }
    }

    pub async fn wait(&mut self) -> Option<i32> {
        (&mut self.exited).await.ok().flatten()
    }

    /// Asks the program and everything it started to stop, then forces it after `grace`.
    pub async fn stop_tree(&mut self, grace: Duration) {
        #[cfg(unix)]
        if let Some(pid) = self.pid {
            // The pty child leads its own session and process group, so -pid reaches all of it.
            // SAFETY: plain signal delivery to a process group we created.
            unsafe { libc::kill(-(pid as i32), libc::SIGTERM) };
            if tokio::time::timeout(grace, &mut self.exited).await.is_ok() {
                return;
            }
            unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        }
        #[cfg(windows)]
        if let Some(pid) = self.pid {
            let _ = grace;
            let taskkill = std::env::var_os("SYSTEMROOT")
                .map(|r| Path::new(&r).join("System32").join("taskkill.exe"))
                .unwrap_or_else(|| "taskkill".into());
            let _ = tokio::process::Command::new(taskkill)
                .args(["/T", "/F", "/PID", &pid.to_string()])
                .output()
                .await;
        }
        let _ = self.killer.kill();
        let _ = tokio::time::timeout(Duration::from_secs(3), &mut self.exited).await;
    }
}

pub struct PtySpec<'a> {
    pub program: &'a Path,
    pub args: &'a [String],
    pub cwd: &'a Path,
    pub env: &'a [(OsString, OsString)],
}

pub fn spawn(
    spec: PtySpec<'_>,
    project_id: &str,
    log: ProjectLog,
    sink: EventSink,
) -> std::io::Result<(Arc<Console>, PtyChild)> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 30,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    // Batch files need cmd.exe to run inside a console.
    let is_batch = spec
        .program
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"));
    let mut cmd = if cfg!(windows) && is_batch {
        let mut c = CommandBuilder::new("cmd.exe");
        c.arg("/c");
        c.arg(spec.program);
        c
    } else {
        CommandBuilder::new(spec.program)
    };
    cmd.args(spec.args);
    cmd.cwd(spec.cwd);
    cmd.env_clear();
    for (k, v) in spec.env {
        cmd.env(k, v);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    drop(pair.slave);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let console = Arc::new(Console {
        master: Mutex::new(pair.master),
        writer: Mutex::new(writer),
        scrollback: Mutex::new(VecDeque::new()),
    });

    let pid = child.process_id();
    let killer = child.clone_killer();
    let (tx, exited) = oneshot::channel();
    std::thread::spawn(move || {
        let code = child.wait().ok().map(|s| s.exit_code() as i32);
        let _ = tx.send(code);
    });

    let (c, id) = (console.clone(), project_id.to_owned());
    let (done_tx, output_done) = oneshot::channel();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut pending = Vec::new(); // incomplete UTF-8 sequence or partial line
        let mut line = Vec::new();
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            pending.extend_from_slice(&buf[..n]);
            let valid = match std::str::from_utf8(&pending) {
                Ok(_) => pending.len(),
                Err(e) if e.error_len().is_none() => e.valid_up_to(),
                Err(_) => pending.len(),
            };
            let text = String::from_utf8_lossy(&pending[..valid]).into_owned();
            pending.drain(..valid);
            if !text.is_empty() {
                // Known secrets are hidden on screen too (best effort: a value split across
                // two reads can slip through here, but never into the log file below).
                let shown = crate::redact::scrub(&text).into_owned();
                if let Ok(mut sb) = c.scrollback.lock() {
                    sb.extend(shown.as_bytes());
                    let excess = sb.len().saturating_sub(SCROLLBACK_BYTES);
                    sb.drain(..excess);
                }
                sink(EngineEvent::Console {
                    project_id: id.clone(),
                    data: shown,
                });
            }
            // Also keep plain lines for the activity log and address discovery.
            for b in text.bytes() {
                if b == b'\n' || b == b'\r' {
                    if !line.is_empty() {
                        log.line(
                            LogStream::Stdout,
                            crate::proc::strip_ansi(&String::from_utf8_lossy(&line)),
                        );
                        line.clear();
                    }
                } else if line.len() < 16 * 1024 {
                    line.push(b);
                }
            }
        }
        if !line.is_empty() {
            log.line(
                LogStream::Stdout,
                crate::proc::strip_ansi(&String::from_utf8_lossy(&line)),
            );
        }
        let _ = done_tx.send(());
    });

    Ok((
        console,
        PtyChild {
            pid,
            killer,
            exited,
            output_done: Some(output_done),
        },
    ))
}
