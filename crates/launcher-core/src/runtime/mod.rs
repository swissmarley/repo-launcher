//! App-owned runtimes. Versions and SHA-256 hashes are pinned here (see DECISIONS D-015);
//! nothing is installed system-wide.

use crate::AppPaths;
use crate::download::{self, Download};
use crate::problem::{Problem, Result};
use crate::source::archive;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

pub const UV_VERSION: &str = "0.13.0";

pub struct PinnedAsset {
    pub file: &'static str,
    pub sha256: &'static str,
}

/// uv release assets for the platforms we ship on (from the release's `sha256.sum`).
pub fn uv_asset() -> Option<PinnedAsset> {
    let (file, sha256) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => (
            "uv-aarch64-apple-darwin.tar.gz",
            "a9c1b29002cf3c83f07fa9cd8a887a3be0107d90e23189721221e7257db8e3d6",
        ),
        ("macos", "x86_64") => (
            "uv-x86_64-apple-darwin.tar.gz",
            "5f44dcbde809b632f47c36fadb241cb4d6f9af71d0c8f172b5d2026d3dde742c",
        ),
        ("windows", "x86_64") => (
            "uv-x86_64-pc-windows-msvc.zip",
            "088962f9e7b7bd9ea740c04c650b2a21c8928c345bd99ac24350dc924dba656c",
        ),
        ("windows", "aarch64") => (
            "uv-aarch64-pc-windows-msvc.zip",
            "cb54028b59aa87f11cf6dec293ce000420043a49a8aea522e822a671321f8932",
        ),
        ("linux", "x86_64") => (
            "uv-x86_64-unknown-linux-gnu.tar.gz",
            "1468ebd5a5541121837c5a2817b9972ba6090fa6caa3d142620850a47fb75154",
        ),
        ("linux", "aarch64") => (
            "uv-aarch64-unknown-linux-gnu.tar.gz",
            "3ccfb6af6e242433eb552f7d9676abd5412c6595c497e990d9c8cb7b5bd4d2c3",
        ),
        _ => return None,
    };
    Some(PinnedAsset { file, sha256 })
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
    let asset = uv_asset().ok_or_else(|| {
        Problem::new(crate::ProblemCode::UnsupportedProject).param(
            "platform",
            format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        )
    })?;
    let url = format!(
        "https://github.com/astral-sh/uv/releases/download/{UV_VERSION}/{}",
        asset.file
    );
    let archive_path = paths.downloads().join(asset.file);
    if !archive_path.is_file() || download::sha256_file(&archive_path)? != asset.sha256 {
        download::fetch(
            http,
            Download {
                url: &url,
                dest: &archive_path,
                sha256: Some(asset.sha256),
                bearer: None,
            },
            cancel,
            progress,
        )
        .await?;
    }

    // Extract to a temp name and rename, so a half-written binary is never mistaken for done.
    let tmp = target.with_extension("partial");
    let (a, t) = (archive_path.clone(), tmp.clone());
    tokio::task::spawn_blocking(move || archive::extract_single(&a, &exe("uv"), &t))
        .await
        .map_err(Problem::internal)??;
    tokio::fs::rename(&tmp, &target).await?;
    let _ = tokio::fs::remove_file(&archive_path).await;
    Ok(target)
}
