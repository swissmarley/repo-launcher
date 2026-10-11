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

/// Sent to the console window when a run starts: clear the screen (xterm.js keeps what was on
/// it in the scrollback) and put the cursor top-left, which is where the pseudo-terminal
/// believes it is (see [`CURSOR_QUERY`]).
pub const SESSION_START: &str = "\x1b[2J\x1b[H";

/// Windows' pseudo-console opens every session by asking where the cursor is ("ESC[6n"), and
/// shows nothing at all until it gets an answer. The engine answers this first query itself,
/// "top-left", so programs run the same with or without a console window (headless tests,
/// a window that opens late), and removes it from the output so the window's terminal doesn't
/// answer a second time. Later queries from the program itself reach the terminal as usual.
const CURSOR_QUERY: &[u8] = b"\x1b[6n";
const CURSOR_AT_ORIGIN: &str = "\x1b[1;1R";

pub struct Console {
    /// `None` once the terminal is closed (Windows, after the program exits).
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    scrollback: Mutex<VecDeque<u8>>,
}

impl Console {
    pub fn write(&self, data: &str) -> std::io::Result<()> {
        let mut w = self
            .writer
            .lock()
            .map_err(|_| std::io::Error::other("console poisoned"))?;
        match w.as_mut() {
            Some(w) => {
                w.write_all(data.as_bytes())?;
                w.flush()
            }
            None => Ok(()),
        }
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        if let Ok(m) = self.master.lock()
            && let Some(m) = m.as_ref()
        {
            let _ = m.resize(PtySize {
                rows: rows.max(2),
                cols: cols.max(10),
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    /// Closes the pseudo-terminal. Windows only delivers the last output, and then the end of
    /// it, when the pseudo-console is closed; this may block until that output is read.
    fn close(&self) {
        if let Ok(mut w) = self.writer.lock() {
            w.take();
        }
        if let Ok(mut m) = self.master.lock() {
            m.take();
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
    console: Arc<Console>,
}

impl PtyChild {
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Waits for the last output to be read, so it is delivered before the exit is reported.
    /// On Windows the output only ends when the pseudo-console closes, so it is closed here
    /// (the program has exited). Bounded, in case something the program started keeps the
    /// terminal open.
    pub async fn drain(&mut self) {
        if cfg!(windows) {
            let console = self.console.clone();
            let _ = tokio::time::timeout(
                Duration::from_secs(2),
                tokio::task::spawn_blocking(move || console.close()),
            )
            .await;
        }
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
        master: Mutex::new(Some(pair.master)),
        writer: Mutex::new(Some(writer)),
        scrollback: Mutex::new(VecDeque::new()),
    });
    sink(EngineEvent::Console {
        project_id: project_id.to_owned(),
        data: SESSION_START.to_owned(),
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
        let mut handshake = cfg!(windows);
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            pending.extend_from_slice(&buf[..n]);
            if handshake {
                match answer_first_cursor_query(&mut pending) {
                    Handshake::Answered => {
                        let _ = c.write(CURSOR_AT_ORIGIN);
                        handshake = false;
                    }
                    Handshake::NeedMore => continue,
                    Handshake::None => handshake = false,
                }
            }
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
        console.clone(),
        PtyChild {
            pid,
            killer,
            exited,
            output_done: Some(output_done),
            console,
        },
    ))
}

#[derive(Debug, PartialEq, Eq)]
enum Handshake {
    /// The output started with the cursor query; it was removed and needs an answer.
    Answered,
    /// Too little output yet to tell.
    NeedMore,
    /// The output doesn't start with the query.
    None,
}

fn answer_first_cursor_query(pending: &mut Vec<u8>) -> Handshake {
    if pending.starts_with(CURSOR_QUERY) {
        pending.drain(..CURSOR_QUERY.len());
        Handshake::Answered
    } else if CURSOR_QUERY.starts_with(pending) {
        Handshake::NeedMore
    } else {
        Handshake::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_only_a_leading_cursor_query() {
        let mut out = b"\x1b[6n\x1b[?25hhello".to_vec();
        assert_eq!(answer_first_cursor_query(&mut out), Handshake::Answered);
        assert_eq!(out, b"\x1b[?25hhello");

        let mut partial = b"\x1b[".to_vec();
        assert_eq!(answer_first_cursor_query(&mut partial), Handshake::NeedMore);
        assert_eq!(partial, b"\x1b[");

        let mut plain = b"hello \x1b[6n".to_vec();
        assert_eq!(answer_first_cursor_query(&mut plain), Handshake::None);
        assert_eq!(plain, b"hello \x1b[6n");
    }

    /// A short-lived program's output reaches the log with no terminal window attached to
    /// answer anything (on Windows, it used to wait forever for a cursor-position reply).
    #[tokio::test(flavor = "multi_thread")]
    async fn short_program_output_arrives_without_a_window() {
        let dir = tempfile::tempdir().unwrap();
        let (program, args): (std::path::PathBuf, Vec<String>) = if cfg!(windows) {
            let root = std::env::var_os("SYSTEMROOT").unwrap_or_else(|| "C:\\Windows".into());
            (
                Path::new(&root).join("System32").join("cmd.exe"),
                vec!["/c".into(), "echo".into(), "hello-from-pty".into()],
            )
        } else {
            (
                "/bin/sh".into(),
                vec!["-c".into(), "echo hello-from-pty".into()],
            )
        };
        let env: Vec<(OsString, OsString)> = ["PATH", "SYSTEMROOT", "WINDIR"]
            .iter()
            .filter_map(|k| Some((OsString::from(k), std::env::var_os(k)?)))
            .collect();
        let events: Arc<Mutex<Vec<EngineEvent>>> = Arc::default();
        let seen = events.clone();
        let sink: EventSink = Arc::new(move |e| seen.lock().unwrap().push(e));
        let log = ProjectLog::create("t", &dir.path().join("run.log"), sink.clone());
        let spec = PtySpec {
            program: &program,
            args: &args,
            cwd: dir.path(),
            env: &env,
        };
        let (console, mut child) = spawn(spec, "t", log, sink).unwrap();
        let code = tokio::time::timeout(Duration::from_secs(20), child.wait())
            .await
            .expect("the program should finish");
        let started = std::time::Instant::now();
        child.drain().await;
        assert_eq!(code, Some(0));
        let lines: Vec<String> = events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                EngineEvent::Log { line, .. } => Some(line.clone()),
                _ => None,
            })
            .collect();
        assert!(
            lines.iter().any(|l| l.contains("hello-from-pty")),
            "{lines:?}"
        );
        assert!(
            started.elapsed() < Duration::from_millis(1900),
            "the end of the output should be seen promptly, not by timeout"
        );
        // The query is answered here, never passed on to a console window.
        assert!(!console.scrollback().contains("\x1b[6n"));
        assert!(matches!(
            &events.lock().unwrap()[0],
            EngineEvent::Console { data, .. } if data == SESSION_START
        ));
    }
}
