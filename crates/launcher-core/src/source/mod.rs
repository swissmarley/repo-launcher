pub mod archive;
pub mod github;
pub mod sync;
pub mod url;

use std::path::PathBuf;

/// Where a project's files come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSpec {
    GitHub(url::RepoRef),
    /// A local folder, used by tests and for trying a project you're developing.
    Local(PathBuf),
}

impl SourceSpec {
    /// Accepts anything [`url::parse`] does, plus a path to an existing local folder.
    pub fn parse(input: &str) -> Result<Self, crate::Problem> {
        let trimmed = input.trim();
        let path = PathBuf::from(trimmed);
        if (path.is_absolute() || trimmed.starts_with('.')) && path.is_dir() {
            return Ok(Self::Local(std::fs::canonicalize(path)?));
        }
        url::parse(trimmed).map(Self::GitHub)
    }
}

/// Recursively copies a local project, skipping folders that are rebuilt during install.
pub fn copy_local(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<usize> {
    const SKIP: &[&str] = &[
        ".git",
        "node_modules",
        ".venv",
        "venv",
        "__pycache__",
        "target",
        "dist",
    ];
    std::fs::create_dir_all(to)?;
    let mut count = 0;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            if SKIP.iter().any(|s| name == *s) {
                continue;
            }
            count += copy_local(&entry.path(), &to.join(&name))?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), to.join(&name))?;
            count += 1;
        }
    }
    Ok(count)
}
