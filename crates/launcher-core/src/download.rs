//! Streaming downloads with SHA-256 verification, cancellation and HTTP Range resume.

use crate::problem::{FixAction, Problem, ProblemCode, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

pub struct Download<'a> {
    pub url: &'a str,
    pub dest: &'a Path,
    /// Lowercase hex. When set, a mismatch deletes the file and fails.
    pub sha256: Option<&'a str>,
    pub bearer: Option<&'a str>,
}

/// Downloads to `dest` (via `dest.part`) and returns the file's SHA-256.
pub async fn fetch(
    http: &reqwest::Client,
    d: Download<'_>,
    cancel: &CancellationToken,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<String> {
    if let Some(parent) = d.dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let part = part_path(d.dest);
    let mut hasher = Sha256::new();
    let mut have = hash_existing(&part, &mut hasher).await?;

    let mut req = http.get(d.url);
    if let Some(token) = d.bearer {
        req = req.bearer_auth(token);
    }
    if have > 0 {
        req = req.header("Range", format!("bytes={have}-"));
    }
    let resp = req.send().await?;
    let status = resp.status();
    let resumed = status.as_u16() == 206;
    if !status.is_success() {
        return Err(Problem::new(ProblemCode::DownloadFailed)
            .fix(FixAction::Retry)
            .detail(format!("GET {} -> {status}", d.url)));
    }
    if !resumed {
        have = 0;
        hasher = Sha256::new();
    }
    let total = resp.content_length().map(|len| len + have);

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&part)
        .await?;

    let mut stream = resp.bytes_stream();
    loop {
        let chunk = tokio::select! {
            _ = cancel.cancelled() => return Err(Problem::new(ProblemCode::Cancelled)),
            c = stream.next() => c,
        };
        let Some(chunk) = chunk else { break };
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        have += chunk.len() as u64;
        progress(have, total);
    }
    file.flush().await?;
    drop(file);

    let digest = hex::encode(hasher.finalize());
    if let Some(expected) = d.sha256
        && !digest.eq_ignore_ascii_case(expected)
    {
        let _ = tokio::fs::remove_file(&part).await;
        return Err(Problem::new(ProblemCode::DownloadCorrupt)
            .fix(FixAction::Retry)
            .detail(format!("{}: expected {expected}, got {digest}", d.url)));
    }
    tokio::fs::rename(&part, d.dest).await?;
    Ok(digest)
}

fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

async fn hash_existing(path: &Path, hasher: &mut Sha256) -> Result<u64> {
    let Ok(mut f) = tokio::fs::File::open(path).await else {
        return Ok(0);
    };
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = f.read(&mut buf).await?;
        if n == 0 {
            return Ok(total);
        }
        hasher.update(&buf[..n]);
        total += n as u64;
    }
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            return Ok(hex::encode(hasher.finalize()));
        }
        hasher.update(&buf[..n]);
    }
}
