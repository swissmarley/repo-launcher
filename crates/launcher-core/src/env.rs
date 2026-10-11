//! Builds the environment for project processes from an allowlist, so the user's other secrets
//! (cloud credentials, tokens, other projects' keys) never reach a child process.

use crate::paths::{AppPaths, ProjectPaths};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

/// Variables passed through unchanged when present. Matching is case-insensitive on Windows.
const ALLOW: &[&str] = &[
    // identity, locale, temp, terminal
    "HOME",
    "USER",
    "LOGNAME",
    "LANG",
    "LANGUAGE",
    "TZ",
    "TMPDIR",
    "TEMP",
    "TMP",
    "TERM",
    "COLORTERM",
    // desktop session (GUI apps, opening browsers, keyrings)
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "XDG_RUNTIME_DIR",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
    "DBUS_SESSION_BUS_ADDRESS",
    "__CF_USER_TEXT_ENCODING",
    // network configuration some users depend on
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "no_proxy",
    "SSL_CERT_FILE",
    // Windows essentials
    "SYSTEMROOT",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "SYSTEMDRIVE",
    "USERPROFILE",
    "USERNAME",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "PROGRAMFILES(X86)",
    "PROGRAMW6432",
    "COMMONPROGRAMFILES",
    "COMPUTERNAME",
    "HOMEDRIVE",
    "HOMEPATH",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "PUBLIC",
];

fn allowed(key: &str) -> bool {
    if key.starts_with("LC_") {
        return true;
    }
    if cfg!(windows) {
        ALLOW.iter().any(|a| a.eq_ignore_ascii_case(key))
    } else {
        ALLOW.contains(&key)
    }
}

pub fn system_path() -> Vec<PathBuf> {
    if cfg!(windows) {
        let root = std::env::var_os("SYSTEMROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| r"C:\Windows".into());
        vec![
            root.join("System32"),
            root.clone(),
            root.join("System32").join("Wbem"),
            root.join("System32").join("WindowsPowerShell").join("v1.0"),
        ]
    } else {
        ["/usr/bin", "/bin", "/usr/sbin", "/sbin"]
            .iter()
            .map(PathBuf::from)
            .collect()
    }
}

/// The allowlisted part of the Launcher's own environment.
pub fn base_env() -> BTreeMap<OsString, OsString> {
    std::env::vars_os()
        .filter(|(k, _)| k.to_str().is_some_and(allowed))
        .collect()
}

/// The environment every engine-run command gets for one project. `tool_path` comes first on
/// PATH (managed runtimes, the project's own environment), then the OS essentials.
pub fn project_env(
    app: &AppPaths,
    project: &ProjectPaths,
    tool_path: &[PathBuf],
    extra: &BTreeMap<String, String>,
) -> Vec<(OsString, OsString)> {
    let mut env = base_env();
    let path = tool_path.iter().cloned().chain(system_path());
    if let Ok(joined) = std::env::join_paths(path) {
        env.insert("PATH".into(), joined);
    }

    let home = project.home();
    let mut set = |k: &str, v: OsString| {
        env.insert(k.into(), v);
    };
    set("VIRTUAL_ENV", project.env().into());
    set("UV_CACHE_DIR", app.uv_cache().into());
    set("UV_PYTHON_INSTALL_DIR", app.python_installs().into());
    set(
        "UV_PYTHON_BIN_DIR",
        app.runtimes().join("python-bin").into(),
    );
    set("UV_TOOL_DIR", app.runtimes().join("uv-tools").into());
    set("UV_PYTHON_PREFERENCE", "only-managed".into());
    set("UV_NO_PROGRESS", "1".into());
    set("PYTHONUNBUFFERED", "1".into());
    set("PYTHONUTF8", "1".into());
    set("PIP_DISABLE_PIP_VERSION_CHECK", "1".into());
    // Caches that projects commonly fill go inside the project, so uninstall removes them.
    set("HF_HOME", home.join("huggingface").into());
    set("TORCH_HOME", home.join("torch").into());
    set("MPLCONFIGDIR", home.join("matplotlib").into());
    if cfg!(target_os = "linux") {
        set("XDG_CACHE_HOME", home.join("cache").into());
    }
    // Node ecosystem: caches in the app folder, no telemetry, no auto-opened browsers.
    let cache = app.cache();
    set("npm_config_cache", cache.join("npm").into());
    set("npm_config_store_dir", cache.join("pnpm-store").into());
    set("npm_config_update_notifier", "false".into());
    set("npm_config_fund", "false".into());
    set("npm_config_audit", "false".into());
    set("YARN_CACHE_FOLDER", cache.join("yarn").into());
    set("YARN_GLOBAL_FOLDER", cache.join("yarn-berry").into());
    set("YARN_ENABLE_TELEMETRY", "0".into());
    set("BUN_INSTALL_CACHE_DIR", cache.join("bun").into());
    set("NEXT_TELEMETRY_DISABLED", "1".into());
    set("ASTRO_TELEMETRY_DISABLED", "1".into());
    set("NUXT_TELEMETRY_DISABLED", "1".into());
    set("GATSBY_TELEMETRY_DISABLED", "1".into());
    set("BROWSER", "none".into());

    for (k, v) in extra {
        env.insert(k.into(), v.into());
    }
    env.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_secrets_and_keeps_essentials() {
        // SAFETY: tests in this module don't read these variables concurrently.
        unsafe {
            std::env::set_var("AWS_SECRET_ACCESS_KEY", "nope");
            std::env::set_var("GITHUB_TOKEN", "nope");
            std::env::set_var("LC_ALL", "en_US.UTF-8");
        }
        let app = AppPaths::at("/tmp/rl");
        let proj = app.project("abc");
        let extra = BTreeMap::from([("PORT".to_string(), "8501".to_string())]);
        let env: BTreeMap<String, String> = project_env(&app, &proj, &[proj.venv_bin()], &extra)
            .into_iter()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.to_string_lossy().into_owned(),
                )
            })
            .collect();
        assert!(!env.contains_key("AWS_SECRET_ACCESS_KEY"));
        assert!(!env.contains_key("GITHUB_TOKEN"));
        assert_eq!(env.get("LC_ALL").map(String::as_str), Some("en_US.UTF-8"));
        assert_eq!(env.get("PORT").map(String::as_str), Some("8501"));
        assert!(env["PATH"].contains("abc"));
        assert_eq!(env["UV_PYTHON_PREFERENCE"], "only-managed");
    }
}
