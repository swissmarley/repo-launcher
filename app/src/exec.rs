//! "Open in my terminal": the system's own terminal runs this same binary as
//! `repo-launcher --exec <id>`, a headless mode with no window and no Tauri. It reads the
//! project's keys from the keychain itself and hands them only to the program it starts, so no
//! key is ever written into a script (DECISIONS D-023, SECURITY.md). The small script some
//! terminals need to be told what to run contains only this binary's path and the project id.

use crate::strings::t;
use launcher_core::{AppPaths, Engine, Problem, ProblemCode};
use std::io::{BufRead, Write};
use std::path::Path;

/// Runs `--exec <id>` if that's what this process was started for; returns its exit code.
pub fn run_if_requested() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|a| a == "--exec")?;
    let id = args.get(at + 1)?.clone();
    #[cfg(windows)]
    attach_console();
    let runtime = tokio::runtime::Runtime::new().ok()?;
    Some(runtime.block_on(exec(&id)))
}

async fn exec(id: &str) -> i32 {
    let engine = match AppPaths::for_current_user()
        .ok_or_else(|| Problem::internal("no per-user data folder"))
        .and_then(Engine::open)
    {
        Ok(e) => e,
        Err(p) => return fail(id, &p),
    };
    let name = engine
        .get(id)
        .map(|p| p.name)
        .unwrap_or_else(|_| id.to_owned());
    // The terminal window's title.
    print!("\x1b]0;{name}\x07");
    let _ = std::io::stdout().flush();
    let mut extra = Vec::new();
    loop {
        let cmd = match engine.exec_command(id, &extra).await {
            Ok(c) => c,
            Err(p) => return fail(&name, &p),
        };
        let spawned = tokio::process::Command::new(&cmd.program)
            .args(&cmd.args)
            .current_dir(&cmd.cwd)
            .env_clear()
            .envs(cmd.env)
            .spawn();
        let mut child = match spawned {
            Ok(c) => c,
            Err(e) => {
                let p = Problem::new(ProblemCode::LaunchFailed)
                    .detail(format!("{}: {e}", cmd.program.display()));
                return fail(&name, &p);
            }
        };
        // Ctrl-C reaches the program (it shares this terminal); we keep waiting for it, so we
        // can say how it ended and offer to run it again.
        let status = loop {
            tokio::select! {
                s = child.wait() => break s,
                _ = tokio::signal::ctrl_c() => {}
            }
        };
        let code = status.ok().and_then(|s| s.code());
        println!();
        match code {
            Some(0) => println!("{}", t("exec.finished", &[("name", &name)])),
            Some(c) => println!(
                "{}",
                t(
                    "exec.finishedCode",
                    &[("name", &name), ("code", &c.to_string())]
                )
            ),
            None => println!("{}", t("exec.finished", &[("name", &name)])),
        }
        print!("{} ", t("exec.again", &[]));
        let _ = std::io::stdout().flush();
        let line = read_line();
        if line.trim().is_empty() {
            return code.unwrap_or(0);
        }
        extra = split_args(&line);
    }
}

fn fail(name: &str, p: &Problem) -> i32 {
    let code = serde_json::to_value(p.code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    let title = t(&format!("problem.{code}.title"), &[]);
    eprintln!(
        "{}",
        t("exec.problem", &[("name", name), ("title", &title)])
    );
    if let Some(d) = &p.detail {
        eprintln!("({d})");
    }
    eprintln!("{}", t("exec.pressEnter", &[]));
    read_line();
    1
}

fn read_line() -> String {
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
    line
}

fn split_args(line: &str) -> Vec<String> {
    match launcher_core::recipe::shell_words::split(line.trim()) {
        Some(simple) if simple.env.is_empty() => simple.argv,
        _ => line.split_whitespace().map(str::to_owned).collect(),
    }
}

/// Release builds on Windows have no console of their own; give `--exec` one.
#[cfg(windows)]
fn attach_console() {
    use windows::Win32::System::Console::{AllocConsole, GetConsoleWindow};
    // SAFETY: plain console calls with no pointers.
    unsafe {
        if GetConsoleWindow().is_invalid() {
            let _ = AllocConsole();
        }
    }
}

/// Opens the system's terminal running `exe --exec <id>`.
pub fn open_in_terminal(exe: &Path, id: &str, scratch: &Path) -> Result<(), Problem> {
    let failed = |e: std::io::Error| Problem::new(ProblemCode::LaunchFailed).detail(e);
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::PermissionsExt;
        // Terminal runs `.command` files; this one holds only the app's path and the id.
        std::fs::create_dir_all(scratch).map_err(failed)?;
        let script = scratch.join("open-in-terminal.command");
        let quoted = exe.to_string_lossy().replace('\'', r"'\''");
        std::fs::write(&script, format!("#!/bin/sh\nexec '{quoted}' --exec {id}\n"))
            .map_err(failed)?;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700))
            .map_err(failed)?;
        std::process::Command::new("/usr/bin/open")
            .arg(&script)
            .spawn()
            .map(|_| ())
            .map_err(failed)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = scratch;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        std::process::Command::new(exe)
            .args(["--exec", id])
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .map(|_| ())
            .map_err(failed)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = scratch;
        // The desktop's preferred terminal first, then common ones, each with its own way of
        // being told what to run.
        let terminals: [(&str, &[&str]); 8] = [
            ("x-terminal-emulator", &["-e"]),
            ("gnome-terminal", &["--"]),
            ("konsole", &["-e"]),
            ("xfce4-terminal", &["-x"]),
            ("kitty", &[]),
            ("alacritty", &["-e"]),
            ("foot", &[]),
            ("xterm", &["-e"]),
        ];
        let (program, flags) = terminals
            .iter()
            .find_map(|(name, flags)| find_on_path(name).map(|p| (p, *flags)))
            .ok_or_else(|| Problem::new(ProblemCode::NoSystemTerminal))?;
        std::process::Command::new(program)
            .args(flags)
            .arg(exe)
            .args(["--exec", id])
            .spawn()
            .map(|_| ())
            .map_err(failed)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn find_on_path(name: &str) -> Option<std::path::PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .chain(["/usr/bin".into(), "/usr/local/bin".into()])
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}
