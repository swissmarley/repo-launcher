//! Every file the engine writes lives under one app-owned root.

use std::path::{Path, PathBuf};

const APP_DIR_NAME: &str = "app.repolauncher.desktop";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
}

impl AppPaths {
    /// The per-user data folder (Local, not Roaming, on Windows: runtimes are large).
    pub fn for_current_user() -> Option<Self> {
        let base = if cfg!(windows) {
            dirs::data_local_dir()?
        } else {
            dirs::data_dir()?
        };
        let name = if cfg!(target_os = "linux") {
            "repo-launcher"
        } else {
            APP_DIR_NAME
        };
        Some(Self::at(base.join(name)))
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn db(&self) -> PathBuf {
        self.root.join("state.db")
    }
    pub fn runtimes(&self) -> PathBuf {
        self.root.join("runtimes")
    }
    pub fn python_installs(&self) -> PathBuf {
        self.runtimes().join("python")
    }
    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }
    pub fn downloads(&self) -> PathBuf {
        self.cache().join("downloads")
    }
    pub fn uv_cache(&self) -> PathBuf {
        self.cache().join("uv")
    }
    /// Short folder names keep Windows paths well under the 260-character limit.
    pub fn project(&self, id: &str) -> ProjectPaths {
        ProjectPaths {
            root: self.root.join("p").join(id),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectPaths {
    pub root: PathBuf,
}

impl ProjectPaths {
    pub fn src(&self) -> PathBuf {
        self.root.join("src")
    }
    pub fn env(&self) -> PathBuf {
        self.root.join("env")
    }
    /// Redirected per-user caches (Hugging Face, Torch, XDG) so uninstall removes them.
    pub fn home(&self) -> PathBuf {
        self.root.join("home")
    }
    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }
    /// Engine bookkeeping: installed-file manifest, completed steps, backups.
    pub fn meta(&self) -> PathBuf {
        self.root.join("meta")
    }
    pub fn venv_bin(&self) -> PathBuf {
        venv_bin(&self.env())
    }
    pub fn venv_python(&self) -> PathBuf {
        let exe = if cfg!(windows) {
            "python.exe"
        } else {
            "python"
        };
        self.venv_bin().join(exe)
    }
}

pub fn venv_bin(venv: &Path) -> PathBuf {
    if cfg!(windows) {
        venv.join("Scripts")
    } else {
        venv.join("bin")
    }
}

/// Deletes a file or a folder tree, read-only entries included (Go makes its module cache
/// read-only). A path that doesn't exist is fine. Symbolic links are removed, never followed.
pub fn remove_tree(path: &Path) -> std::io::Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if !meta.is_dir() {
        make_writable(path, &meta);
        return std::fs::remove_file(path);
    }
    if std::fs::remove_dir_all(path).is_ok() {
        return Ok(());
    }
    make_tree_writable(path);
    std::fs::remove_dir_all(path)
}

fn make_tree_writable(dir: &Path) {
    if let Ok(meta) = std::fs::symlink_metadata(dir) {
        make_writable(dir, &meta);
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            make_tree_writable(&path);
        } else if !meta.is_symlink() {
            make_writable(&path, &meta);
        }
    }
}

fn make_writable(path: &Path, meta: &std::fs::Metadata) {
    let mut perms = meta.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(perms.mode() | 0o700);
    }
    #[cfg(windows)]
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    let _ = std::fs::set_permissions(path, perms);
}

#[cfg(test)]
mod tests {
    use super::remove_tree;

    #[test]
    fn removes_read_only_trees() {
        let root = tempfile::tempdir().unwrap();
        let tree = root.path().join("cache");
        let inner = tree.join("pkg").join("mod");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(inner.join("go.mod"), "module x").unwrap();
        let mut ro = std::fs::metadata(&inner).unwrap().permissions();
        ro.set_readonly(true);
        std::fs::set_permissions(inner.join("go.mod"), ro.clone()).unwrap();
        std::fs::set_permissions(&inner, ro).unwrap();
        remove_tree(&tree).unwrap();
        assert!(!tree.exists());
        remove_tree(&tree).unwrap();
    }
}
