//! Applies a freshly downloaded source tree onto a project's folder without losing the user's
//! work. A manifest records the hash of every file we installed:
//! - files we installed and the user didn't touch are replaced (or removed upstream),
//! - files the user changed are backed up first, then replaced,
//! - files the project or user created (outputs, models, `.env`, node_modules) are left alone.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub type Manifest = BTreeMap<String, String>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub written: usize,
    pub removed: usize,
    pub backed_up: Vec<String>,
}

pub fn load_manifest(path: &Path) -> Option<Manifest> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

pub fn save_manifest(path: &Path, m: &Manifest) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_vec(m).map_err(std::io::Error::other)?)
}

/// Hashes every file under `root` (relative, forward-slash paths).
pub fn manifest_of(root: &Path) -> std::io::Result<Manifest> {
    let mut out = Manifest::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Manifest) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_dir() {
            walk(root, &path, out)?;
        } else if kind.is_file() {
            out.insert(rel(root, &path), crate::download::sha256_file(&path)?);
        }
    }
    Ok(())
}

fn rel(root: &Path, path: &Path) -> String {
    let r = path.strip_prefix(root).unwrap_or(path);
    r.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn native(root: &Path, rel: &str) -> PathBuf {
    rel.split('/').fold(root.to_owned(), |p, part| p.join(part))
}

/// Moves `incoming` (a complete new source tree) into `dest`. `previous` is the manifest written
/// by the last sync; without one, `dest` is replaced outright.
pub fn apply(
    incoming: &Path,
    dest: &Path,
    previous: Option<&Manifest>,
    backups: &Path,
) -> std::io::Result<(Manifest, SyncReport)> {
    let next = manifest_of(incoming)?;
    let mut report = SyncReport::default();

    let Some(previous) = previous.filter(|_| dest.is_dir()) else {
        if dest.exists() {
            std::fs::remove_dir_all(dest)?;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(incoming, dest)?;
        report.written = next.len();
        return Ok((next, report));
    };

    // Back up user edits to files we're about to overwrite or remove.
    for (path, old_hash) in previous {
        let target = native(dest, path);
        let Ok(current) = crate::download::sha256_file(&target) else {
            continue;
        };
        let changes = next.get(path) != Some(&current);
        if current != *old_hash && changes {
            let backup = native(backups, path);
            if let Some(parent) = backup.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&target, &backup)?;
            report.backed_up.push(path.clone());
        }
        if !next.contains_key(path) {
            std::fs::remove_file(&target)?;
            report.removed += 1;
        }
    }
    for (path, hash) in &next {
        let target = native(dest, path);
        if crate::download::sha256_file(&target).ok().as_ref() == Some(hash) {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(native(incoming, path), &target)?;
        report.written += 1;
    }
    std::fs::remove_dir_all(incoming)?;
    Ok((next, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, files: &[(&str, &str)]) {
        for (p, body) in files {
            let path = native(root, p);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        }
    }

    fn read(root: &Path, p: &str) -> Option<String> {
        std::fs::read_to_string(native(root, p)).ok()
    }

    #[test]
    fn update_keeps_user_work() {
        let dir = tempfile::tempdir().unwrap();
        let (dest, backups) = (dir.path().join("src"), dir.path().join("backups"));

        let v1 = dir.path().join("v1");
        write(
            &v1,
            &[
                ("app.py", "v1"),
                ("config.yaml", "default"),
                ("old.txt", "x"),
                ("lib/util.py", "u1"),
            ],
        );
        let (m1, _) = apply(&v1, &dest, None, &backups).unwrap();

        // The user edits config and the project writes outputs.
        write(
            &dest,
            &[
                ("config.yaml", "mine"),
                ("outputs/result.png", "img"),
                (".env", "KEY=1"),
            ],
        );

        let v2 = dir.path().join("v2");
        write(
            &v2,
            &[
                ("app.py", "v2"),
                ("config.yaml", "new default"),
                ("lib/util.py", "u1"),
                ("new.py", "n"),
            ],
        );
        let (m2, report) = apply(&v2, &dest, Some(&m1), &backups).unwrap();

        assert_eq!(read(&dest, "app.py").as_deref(), Some("v2"));
        assert_eq!(read(&dest, "new.py").as_deref(), Some("n"));
        assert_eq!(read(&dest, "old.txt"), None, "removed upstream");
        assert_eq!(read(&dest, "config.yaml").as_deref(), Some("new default"));
        assert_eq!(
            read(&backups, "config.yaml").as_deref(),
            Some("mine"),
            "user edit backed up"
        );
        assert_eq!(
            read(&dest, "outputs/result.png").as_deref(),
            Some("img"),
            "project output untouched"
        );
        assert_eq!(read(&dest, ".env").as_deref(), Some("KEY=1"));
        assert_eq!(report.backed_up, vec!["config.yaml".to_string()]);
        assert_eq!(report.removed, 1);
        assert_eq!(
            report.written, 3,
            "app.py, config.yaml, new.py; util.py unchanged"
        );
        assert!(!m2.contains_key("outputs/result.png"));
    }
}
