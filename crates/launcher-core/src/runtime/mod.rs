//! App-owned runtimes (uv/Python, Node, package managers, bun). Nothing is installed system-wide.

pub mod pins;

use crate::AppPaths;
use crate::download::{self, Download};
use crate::problem::{Problem, ProblemCode, Result};
use crate::recipe::{NodeSpec, PackageManager};
use crate::source::archive;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

pub use pins::UV_VERSION;

/// The managed programs a project's commands can use. `path` is searched in order and becomes
/// the front of the child's PATH.
#[derive(Debug, Clone, Default)]
pub struct Tools {
    pub uv: PathBuf,
    pub python: PathBuf,
    pub path: Vec<PathBuf>,
}

pub fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

pub fn uv_path(paths: &AppPaths) -> PathBuf {
    paths.runtimes().join("uv").join(UV_VERSION).join(exe("uv"))
}

pub fn node_dir(paths: &AppPaths, version: &str) -> PathBuf {
    paths.runtimes().join("node").join(version)
}

/// Where `node` and `npm` live inside a Node distribution.
pub fn node_bin(node_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        node_dir.to_owned()
    } else {
        node_dir.join("bin")
    }
}

pub fn bun_dir(paths: &AppPaths) -> PathBuf {
    paths.runtimes().join("bun").join(pins::BUN_VERSION)
}

pub fn go_dir(paths: &AppPaths) -> PathBuf {
    paths.runtimes().join("go").join(pins::GO_VERSION)
}

/// Returns the folder of the managed Go toolchain, downloading it on first use.
pub async fn ensure_go(
    paths: &AppPaths,
    http: &reqwest::Client,
    cancel: &CancellationToken,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf> {
    let dir = go_dir(paths);
    if dir.join("bin").join(exe("go")).is_file() {
        return Ok(dir);
    }
    let asset = pins::go().ok_or_else(unsupported_platform)?;
    let url = format!("https://go.dev/dl/{}", asset.file);
    let archive_path = fetch_asset(paths, http, &url, &asset, cancel, progress).await?;
    let staging = dir.with_extension("partial");
    let _ = tokio::fs::remove_dir_all(&staging).await;
    let (a, s) = (archive_path.clone(), staging.clone());
    blocking(move || archive::extract_any(&a, &s, true).map(|_| ())).await?;
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::rename(&staging, &dir).await?;
    let _ = tokio::fs::remove_file(&archive_path).await;
    Ok(dir)
}

/// npm-installed tools put their launchers in `<prefix>/bin` (POSIX) or `<prefix>` (Windows).
pub fn prefix_bin(prefix: &Path) -> PathBuf {
    if cfg!(windows) {
        prefix.to_owned()
    } else {
        prefix.join("bin")
    }
}

/// The npm package and version that provide a project's package manager, if one is needed.
pub fn package_manager_package(spec: &NodeSpec) -> Option<(&'static str, String)> {
    let requested = spec.package_manager_version.clone();
    match spec.package_manager {
        PackageManager::Npm | PackageManager::Bun => None,
        PackageManager::Pnpm => Some((
            "pnpm",
            requested.unwrap_or_else(|| pins::PNPM_DEFAULT.into()),
        )),
        PackageManager::Yarn => {
            let v = requested.unwrap_or_else(|| pins::YARN_CLASSIC_DEFAULT.into());
            if v.starts_with("1") {
                Some(("yarn", v))
            } else {
                Some(("@yarnpkg/cli-dist", v))
            }
        }
    }
}

pub fn package_manager_prefix(paths: &AppPaths, package: &str, version: &str) -> PathBuf {
    let safe: String = format!("{package}@{version}")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '@' {
                c
            } else {
                '_'
            }
        })
        .collect();
    paths.runtimes().join("pm").join(safe)
}

fn unsupported_platform() -> Problem {
    Problem::new(ProblemCode::UnsupportedProject).param(
        "platform",
        format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
    )
}

/// Downloads a pinned asset into the download cache (reusing a verified copy if present).
async fn fetch_asset(
    paths: &AppPaths,
    http: &reqwest::Client,
    url: &str,
    asset: &pins::Asset,
    cancel: &CancellationToken,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf> {
    let path = paths.downloads().join(&asset.file);
    if path.is_file() && download::sha256_file(&path)? == asset.sha256 {
        return Ok(path);
    }
    download::fetch(
        http,
        Download {
            url,
            dest: &path,
            sha256: Some(asset.sha256),
            bearer: None,
        },
        cancel,
        progress,
    )
    .await?;
    Ok(path)
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> std::io::Result<T> + Send + 'static,
) -> Result<T> {
    Ok(tokio::task::spawn_blocking(f)
        .await
        .map_err(Problem::internal)??)
}

/// Returns the managed uv binary, downloading and verifying it on first use.
pub async fn ensure_uv(
    paths: &AppPaths,
    http: &reqwest::Client,
    cancel: &CancellationToken,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf> {
    let target = uv_path(paths);
    if target.is_file() {
        return Ok(target);
    }
    let asset = pins::uv().ok_or_else(unsupported_platform)?;
    let url = format!(
        "https://github.com/astral-sh/uv/releases/download/{UV_VERSION}/{}",
        asset.file
    );
    let archive_path = fetch_asset(paths, http, &url, &asset, cancel, progress).await?;
    // Extract under a temp name and rename, so a half-written binary is never mistaken for done.
    let tmp = target.with_extension("partial");
    let (a, t) = (archive_path.clone(), tmp.clone());
    blocking(move || archive::extract_single(&a, &exe("uv"), &t)).await?;
    tokio::fs::rename(&tmp, &target).await?;
    let _ = tokio::fs::remove_file(&archive_path).await;
    Ok(target)
}

/// Returns the folder of a managed Node distribution, downloading it on first use.
pub async fn ensure_node(
    paths: &AppPaths,
    http: &reqwest::Client,
    version: &str,
    cancel: &CancellationToken,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf> {
    let dir = node_dir(paths, version);
    if node_bin(&dir).join(exe("node")).is_file() {
        return Ok(dir);
    }
    let asset = pins::node(version).ok_or_else(unsupported_platform)?;
    let url = format!("https://nodejs.org/dist/v{version}/{}", asset.file);
    let archive_path = fetch_asset(paths, http, &url, &asset, cancel, progress).await?;
    let staging = dir.with_extension("partial");
    let _ = tokio::fs::remove_dir_all(&staging).await;
    let (a, s) = (archive_path.clone(), staging.clone());
    blocking(move || archive::extract_any(&a, &s, true).map(|_| ())).await?;
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::rename(&staging, &dir).await?;
    let _ = tokio::fs::remove_file(&archive_path).await;
    Ok(dir)
}

/// Returns the folder holding the managed `bun` binary.
pub async fn ensure_bun(
    paths: &AppPaths,
    http: &reqwest::Client,
    cancel: &CancellationToken,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf> {
    let dir = bun_dir(paths);
    let target = dir.join(exe("bun"));
    if target.is_file() {
        return Ok(dir);
    }
    let asset = pins::bun().ok_or_else(unsupported_platform)?;
    let url = format!(
        "https://github.com/oven-sh/bun/releases/download/bun-v{}/{}",
        pins::BUN_VERSION,
        asset.file
    );
    let archive_path = fetch_asset(paths, http, &url, &asset, cancel, progress).await?;
    let tmp = target.with_extension("partial");
    let (a, t) = (archive_path.clone(), tmp.clone());
    blocking(move || archive::extract_single(&a, &exe("bun"), &t)).await?;
    tokio::fs::rename(&tmp, &target).await?;
    let _ = tokio::fs::remove_file(&archive_path).await;
    Ok(dir)
}

/// Installs a package manager (pnpm, yarn) with the managed npm. npm verifies each package
/// against the registry's integrity hash. Returns the folder holding its launchers.
pub async fn ensure_package_manager(
    paths: &AppPaths,
    node_dir: &Path,
    package: &str,
    version: &str,
    cancel: &CancellationToken,
) -> Result<PathBuf> {
    let prefix = package_manager_prefix(paths, package, version);
    let marker = prefix.join(".installed");
    if marker.is_file() {
        return Ok(prefix_bin(&prefix));
    }
    let npm = node_bin(node_dir).join(if cfg!(windows) { "npm.cmd" } else { "npm" });
    let mut env = crate::env::base_env();
    let path = std::env::join_paths(
        [node_bin(node_dir)]
            .into_iter()
            .chain(crate::env::system_path()),
    )
    .map_err(Problem::internal)?;
    env.insert("PATH".into(), path);
    env.insert("npm_config_cache".into(), paths.cache().join("npm").into());
    env.insert("npm_config_update_notifier".into(), "false".into());
    let args: Vec<String> = [
        "install",
        "--global",
        "--no-audit",
        "--no-fund",
        "--prefix",
        &prefix.to_string_lossy(),
        &format!("{package}@{version}"),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let env: Vec<_> = env.into_iter().collect();
    let mut child = crate::proc::spawn(crate::proc::Spec {
        program: &npm,
        args: &args,
        cwd: &paths.root,
        env: &env,
    })?;
    // Drain both pipes while npm runs so a full pipe can't stall it.
    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let mut pumps = Vec::new();
    for pipe in [
        child
            .stdout()
            .take()
            .map(|p| Box::new(p) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
        child
            .stderr()
            .take()
            .map(|p| Box::new(p) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let out = output.clone();
        pumps.push(tokio::spawn(crate::proc::pump(pipe, move |l| {
            if let Ok(mut o) = out.lock() {
                o.push(l);
            }
        })));
    }
    let status = tokio::select! {
        s = child.wait() => s?,
        _ = cancel.cancelled() => {
            let _ = crate::proc::stop_tree(&mut child, std::time::Duration::from_secs(3)).await;
            return Err(Problem::new(ProblemCode::Cancelled));
        }
    };
    for p in pumps {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), p).await;
    }
    if !status.success() {
        let lines = output.lock().map(|o| o.clone()).unwrap_or_default();
        let tail: Vec<String> = lines[lines.len().saturating_sub(40)..].to_vec();
        return Err(crate::diagnose::install_failure(&tail, status.code()));
    }
    tokio::fs::write(&marker, b"").await?;
    Ok(prefix_bin(&prefix))
}
