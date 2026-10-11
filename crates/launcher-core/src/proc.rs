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

/// Resolves a recipe's first argv element to a concrete program.
pub fn resolve_program(name: &str, src: &Path, venv_bin: &Path, uv: &Path) -> PathBuf {
    match name {
        "uv" => return uv.to_owned(),
        "python" | "python3" => return venv_bin.join(crate::runtime::exe("python")),
        _ => {}
    }
    if name.starts_with("./") || name.starts_with(".\\") {
        return src.join(&name[2..]);
    }
    for candidate in [crate::runtime::exe(name), name.to_owned()] {
        let p = venv_bin.join(&candidate);
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

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
