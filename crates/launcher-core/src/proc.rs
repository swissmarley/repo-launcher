//! Spawning project processes as a killable tree: a process group on POSIX, a Job Object on
//! Windows (children die even if the Launcher itself crashes).

use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};

pub struct Spec<'a> {
    pub program: &'a Path,
    pub args: &'a [String],
    pub cwd: &'a Path,
    pub env: &'a [(OsString, OsString)],
}

pub fn spawn(spec: Spec<'_>) -> std::io::Result<Box<dyn ChildWrapper>> {
    let mut cmd = CommandWrap::with_new(spec.program, |c| {
        c.args(spec.args)
            .current_dir(spec.cwd)
            .env_clear()
            .envs(spec.env.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });
    #[cfg(unix)]
    cmd.wrap(process_wrap::tokio::ProcessGroup::leader());
    #[cfg(windows)]
    {
        use windows::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};
        cmd.wrap(process_wrap::tokio::CreationFlags(
            CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP,
        ));
        cmd.wrap(process_wrap::tokio::JobObject);
    }
    cmd.wrap(KillOnDrop);
    cmd.spawn()
}

/// Asks the whole tree to stop, then force-kills it after `grace`.
pub async fn stop_tree(child: &mut Box<dyn ChildWrapper>, grace: Duration) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        const SIGTERM: i32 = 15;
        if child.signal(SIGTERM).is_ok()
            && let Ok(res) = tokio::time::timeout(grace, child.wait()).await
        {
            return res.map(|_| ());
        }
    }
    #[cfg(windows)]
    let _ = grace;
    Box::into_pin(child.kill()).await
}

/// Reads a stream and calls `on_line` per line. Splits on `\n` and on bare `\r` (progress bars),
/// strips ANSI escape codes, and decodes invalid UTF-8 lossily.
pub async fn pump<R: AsyncRead + Unpin>(mut reader: R, mut on_line: impl FnMut(String)) {
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        let n = match reader.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for &b in &chunk[..n] {
            if b == b'\n' || b == b'\r' {
                if !buf.is_empty() {
                    on_line(clean(&buf));
                    buf.clear();
                }
            } else if buf.len() < 16 * 1024 {
                buf.push(b);
            }
        }
    }
    if !buf.is_empty() {
        on_line(clean(&buf));
    }
}

fn clean(bytes: &[u8]) -> String {
    strip_ansi(&String::from_utf8_lossy(bytes))
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    // CSI: parameters then one final byte in @..~
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC: until BEL or ESC \
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {
                    chars.next();
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Rewrites commands that need a different form in our environments: venvs made by uv have no
/// `pip`, so `pip …` becomes `uv pip …`.
pub fn normalize_argv(mut argv: Vec<String>) -> Vec<String> {
    if matches!(argv.first().map(String::as_str), Some("pip" | "pip3")) {
        argv[0] = "pip".into();
        argv.insert(0, "uv".into());
    }
    argv
}

/// Resolves a recipe's first argv element to a concrete program: managed tools first, then
/// project-local scripts, then anything on the tool path (venv, node_modules/.bin, runtimes).
pub fn resolve_program(name: &str, root: &Path, tools: &crate::runtime::Tools) -> PathBuf {
    match name {
        "uv" => return tools.uv.clone(),
        "python" | "python3" => return tools.python.clone(),
        _ => {}
    }
    if let Some(rel) = name.strip_prefix("./").or_else(|| name.strip_prefix(".\\")) {
        return root.join(rel);
    }
    let candidates: Vec<String> = if cfg!(windows) {
        ["exe", "cmd", "bat"]
            .iter()
            .map(|e| format!("{name}.{e}"))
            .chain([name.to_owned()])
            .collect()
    } else {
        vec![name.to_owned()]
    };
    for dir in &tools.path {
        for c in &candidates {
            let p = dir.join(c);
            if p.is_file() {
                return p;
            }
        }
    }
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pip_goes_through_uv() {
        let argv = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            normalize_argv(argv(&["pip", "install", "x"])),
            argv(&["uv", "pip", "install", "x"])
        );
        assert_eq!(
            normalize_argv(argv(&["pip3", "list"])),
            argv(&["uv", "pip", "list"])
        );
        assert_eq!(
            normalize_argv(argv(&["python", "x.py"])),
            argv(&["python", "x.py"])
        );
    }

    #[test]
    fn strips_ansi() {
        assert_eq!(strip_ansi("\u{1b}[32mok\u{1b}[0m done"), "ok done");
        assert_eq!(
            strip_ansi("\u{1b}]8;;http://x\u{7}link\u{1b}]8;;\u{7}"),
            "link"
        );
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[tokio::test]
    async fn pump_splits_lines_and_progress() {
        let input: &[u8] = b"one\r\ntwo\rthree\n\xffbad\n";
        let mut lines = Vec::new();
        pump(input, |l| lines.push(l)).await;
        assert_eq!(lines, vec!["one", "two", "three", "\u{fffd}bad"]);
    }
}
