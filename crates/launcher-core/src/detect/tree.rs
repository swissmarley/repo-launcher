use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 4;
const MAX_FILES: usize = 20_000;
const MAX_READ_BYTES: u64 = 512 * 1024;
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".venv",
    "venv",
    "env",
    "__pycache__",
    "target",
    "dist",
    "build",
    ".next",
    ".tox",
    ".mypy_cache",
    "site-packages",
];

/// A bounded view of a project's files with forward-slash relative paths.
#[derive(Debug, Clone)]
pub struct FileTree {
    root: PathBuf,
    files: BTreeSet<String>,
}

impl FileTree {
    pub fn scan(root: &Path) -> std::io::Result<Self> {
        let mut files = BTreeSet::new();
        walk(root, root, 0, &mut files)?;
        Ok(Self {
            root: root.to_owned(),
            files,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn has(&self, rel: &str) -> bool {
        self.files.contains(rel)
    }

    pub fn files(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(String::as_str)
    }

    /// Reads a text file, ignoring anything too large or not UTF-8.
    pub fn read(&self, rel: &str) -> Option<String> {
        if !self.has(rel) {
            return None;
        }
        let path = self.root.join(rel);
        if std::fs::metadata(&path).ok()?.len() > MAX_READ_BYTES {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        String::from_utf8(bytes).ok()
    }
}

fn walk(root: &Path, dir: &Path, depth: usize, out: &mut BTreeSet<String>) -> std::io::Result<()> {
    if depth > MAX_DEPTH || out.len() >= MAX_FILES {
        return Ok(());
    }
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            if !SKIP_DIRS.contains(&name) {
                walk(root, &path, depth + 1, out)?;
            }
        } else if kind.is_file()
            && let Ok(rel) = path.strip_prefix(root)
        {
            let rel: Vec<_> = rel
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect();
            out.insert(rel.join("/"));
        }
    }
    Ok(())
}
