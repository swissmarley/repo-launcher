//! Turns the tail of a failed command's output into a specific, fixable problem.

use crate::problem::{FixAction, Problem, ProblemCode};

const COMPILER: &[&str] = &[
    "Microsoft Visual C++ 14.0 or greater is required",
    "xcrun: error: invalid active developer path",
    "error: command 'gcc' failed",
    "error: command 'cc' failed",
    "unable to execute 'gcc'",
    "error: linker `cc` not found",
    "No such file or directory: 'cc'",
    "gcc: command not found",
    "cc: command not found",
];

const OFFLINE: &[&str] = &[
    "dns error",
    "failed to lookup address",
    "Could not connect",
    "Connection refused",
    "Network is unreachable",
    "tcp connect error",
    "error sending request",
];

pub fn install_failure(tail: &[String], exit_code: Option<i32>) -> Problem {
    let p = diagnose(tail)
        .unwrap_or_else(|| Problem::new(ProblemCode::InstallStepFailed).fix(FixAction::Retry));
    p.detail(detail(tail, exit_code))
}

pub fn launch_failure(tail: &[String], exit_code: Option<i32>) -> Problem {
    let p = diagnose(tail).unwrap_or_else(|| {
        let mut p = Problem::new(ProblemCode::ExitedEarly).fix(FixAction::Repair);
        if let Some(module) = tail.iter().rev().find_map(|l| missing_module(l)) {
            p = p.param("module", module);
        }
        p
    });
    p.detail(detail(tail, exit_code))
}

fn diagnose(tail: &[String]) -> Option<Problem> {
    let hit = |needles: &[&str]| tail.iter().any(|l| needles.iter().any(|n| l.contains(n)));
    if hit(COMPILER) {
        let fix = if cfg!(target_os = "macos") {
            FixAction::InstallMacCommandLineTools
        } else if cfg!(windows) {
            FixAction::OpenUrl {
                url: "https://visualstudio.microsoft.com/visual-cpp-build-tools/".into(),
            }
        } else {
            FixAction::Retry
        };
        return Some(Problem::new(ProblemCode::NeedsCompiler).fix(fix));
    }
    if hit(OFFLINE) {
        return Some(Problem::new(ProblemCode::Offline).fix(FixAction::CheckConnection));
    }
    None
}

fn missing_module(line: &str) -> Option<String> {
    let rest = line.split("No module named ").nth(1)?;
    Some(
        rest.trim()
            .trim_matches(|c| c == '\'' || c == '"')
            .to_owned(),
    )
}

fn detail(tail: &[String], exit_code: Option<i32>) -> String {
    let code = exit_code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "signal".into());
    format!("exit code {code}\n{}", tail.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &[&str]) -> Vec<String> {
        s.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn recognizes_compiler_and_network() {
        let p = install_failure(
            &lines(&[
                "Building wheel",
                "error: command 'gcc' failed: No such file",
            ]),
            Some(1),
        );
        assert_eq!(p.code, ProblemCode::NeedsCompiler);
        let p = install_failure(
            &lines(&["error: Request failed", "  Caused by: dns error: failed"]),
            Some(2),
        );
        assert_eq!(p.code, ProblemCode::Offline);
        let p = install_failure(&lines(&["something else"]), Some(1));
        assert_eq!(p.code, ProblemCode::InstallStepFailed);
        assert!(p.detail.unwrap().contains("exit code 1"));
    }

    #[test]
    fn names_missing_module() {
        let p = launch_failure(
            &lines(&["Traceback", "ModuleNotFoundError: No module named 'cv2'"]),
            Some(1),
        );
        assert_eq!(p.code, ProblemCode::ExitedEarly);
        assert_eq!(p.params.get("module").map(String::as_str), Some("cv2"));
    }
}
