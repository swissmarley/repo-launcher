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
