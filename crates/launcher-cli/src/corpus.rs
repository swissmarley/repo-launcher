//! `rl corpus-sync`: downloads each corpus repo once and stores a small snapshot of what
//! detection reads: the file listing, manifest files, and only the relevant lines of sources.
//! Tests then run offline against the snapshots.

use launcher_core::Problem;
use launcher_core::detect::FileTree;
use launcher_core::download::{self, Download};
use launcher_core::source::{SourceSpec, archive, github::GitHub};
use serde::Deserialize;
use std::path::Path;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Deserialize)]
pub struct Corpus {
    pub repo: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub url: String,
    pub commit: String,
}

/// Folder name for a corpus entry, e.g. `pallets__flask__examples-tutorial`.
pub fn slug(url: &str) -> String {
    let path = url.trim_start_matches("https://github.com/");
    let mut parts: Vec<&str> = path.split('/').collect();
    let (owner, repo) = (parts.remove(0), parts.remove(0));
    let mut s = format!("{owner}__{repo}");
    // Drop "tree/<ref>" and keep the subfolder.
    if parts.first() == Some(&"tree") && parts.len() > 2 {
        s.push_str("__");
        s.push_str(&parts[2..].join("-"));
    }
    s
}

const MANIFESTS: &[&str] = &[
    "package.json",
    "pyproject.toml",
    "setup.cfg",
    "Pipfile",
    ".python-version",
    "runtime.txt",
    ".nvmrc",
    ".node-version",
    "launcher.json",
    "environment.yml",
    "environment.yaml",
    "pnpm-workspace.yaml",
    ".yarnrc.yml",
    "go.mod",
    "Cargo.toml",
    "composer.json",
];
const HEAD_ONLY: &[&str] = &["pnpm-lock.yaml", "yarn.lock"];

/// What to keep of a file, if anything.
fn reduce(rel: &str, text: &str) -> Option<String> {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    if MANIFESTS.contains(&name)
        || (name.starts_with("requirements") && name.ends_with(".txt"))
        || rel.starts_with("requirements/")
    {
        return Some(text.to_owned());
    }
    if HEAD_ONLY.contains(&name) {
        return Some(text.lines().take(10).collect::<Vec<_>>().join("\n"));
    }
    let keep = |line: &str| -> bool {
        let l = line.trim();
        l.starts_with("import ")
            || l.starts_with("from ")
            || [
                "Flask(",
                "FastAPI(",
                ".launch(",
                "__name__",
                "__main__",
                "set_page_config",
            ]
            .iter()
            .any(|n| l.contains(n))
    };
    if name.ends_with(".py") && rel.matches('/').count() <= 1 {
        return Some(
            text.lines()
                .filter(|l| keep(l))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    if name.ends_with(".go") && rel.matches('/').count() <= 2 {
        return Some(
            text.lines()
                .filter(|l| l.starts_with("package "))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    if name.to_ascii_lowercase().starts_with("readme") && !rel.contains('/') {
        let cmd = |l: &str| {
            ["run ", "python", "npm ", "pnpm ", "yarn ", "streamlit"]
                .iter()
                .any(|n| l.contains(n))
        };
        return Some(
            text.lines()
                .filter(|l| cmd(l))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    None
}

pub async fn sync(list: &Path, out: &Path) -> Result<(), Problem> {
    let corpus: Corpus =
        toml::from_str(&std::fs::read_to_string(list)?).map_err(Problem::internal)?;
    let http = reqwest::Client::builder()
        .user_agent("RepoLauncher-corpus")
        .build()
        .map_err(Problem::internal)?;
    let github = GitHub::new(http.clone());
    let cancel = CancellationToken::new();

    for entry in &corpus.repo {
        let dest = out.join(slug(&entry.url));
        if dest.join("tree.txt").is_file() {
            eprintln!("have   {}", entry.url);
            continue;
        }
        eprintln!("fetch  {}", entry.url);
        let SourceSpec::GitHub(r) = SourceSpec::parse(&entry.url)? else {
            return Err(Problem::internal("corpus entries must be GitHub links"));
        };
        let subdir = match &r.tree {
            Some(_) => github.resolve_ref(&r, None).await?.subdir,
            None => None,
        };
        let tmp = tempfile::tempdir()?;
        let file = tmp.path().join("src.tar.gz");
        let url = github.archive_url(&r, &entry.commit);
        download::fetch(
            &http,
            Download {
                url: &url,
                dest: &file,
                sha256: None,
                bearer: None,
            },
            &cancel,
            |_, _| {},
        )
        .await?;
        let src = tmp.path().join("src");
        archive::extract_tar_gz(&file, &src, true)?;
        let root = match &subdir {
            Some(sub) => sub.split('/').fold(src.clone(), |p, part| p.join(part)),
            None => src.clone(),
        };
        let tree = FileTree::scan(&root)?;

        let _ = std::fs::remove_dir_all(&dest);
        std::fs::create_dir_all(dest.join("files"))?;
        let listing: Vec<&str> = tree.files().collect();
        std::fs::write(dest.join("tree.txt"), listing.join("\n") + "\n")?;
        for rel in listing {
            let Some(text) = tree.read(rel) else { continue };
            if let Some(kept) = reduce(rel, &text) {
                let target = rel
                    .split('/')
                    .fold(dest.join("files"), |p, part| p.join(part));
                std::fs::create_dir_all(target.parent().unwrap_or(&dest))?;
                std::fs::write(target, kept)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("https://github.com/psf/black"), "psf__black");
        assert_eq!(
            slug("https://github.com/pallets/flask/tree/main/examples/tutorial"),
            "pallets__flask__examples-tutorial"
        );
        assert_eq!(
            slug("https://github.com/nuxt/starter/tree/templates"),
            "nuxt__starter"
        );
    }

    #[test]
    fn keeps_only_what_detection_reads() {
        let py = "import os\nSECRET = 1\napp = Flask(__name__)\ndef f():\n    pass\n";
        assert_eq!(
            reduce("app.py", py).unwrap(),
            "import os\napp = Flask(__name__)"
        );
        assert!(reduce("deep/er/app.py", py).is_none());
        assert_eq!(
            reduce("pnpm-lock.yaml", &"x\n".repeat(50))
                .unwrap()
                .lines()
                .count(),
            10
        );
        assert!(reduce("src/index.ts", "x").is_none());
    }
}
