//! Go modules: finds the `main` package to build.

use super::FileTree;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoInfo {
    pub module: String,
    /// Package path to build, e.g. "." or "./cmd/tool".
    pub main_package: Option<String>,
    /// Name for the built program.
    pub binary: String,
}

pub fn detect(tree: &FileTree) -> Option<GoInfo> {
    let gomod = tree.read("go.mod")?;
    let module = gomod
        .lines()
        .find_map(|l| l.trim().strip_prefix("module "))
        .map(|m| m.trim().to_owned())?;
    let is_main = |f: &str| {
        tree.read(f)
            .is_some_and(|src| src.lines().any(|l| l.trim() == "package main"))
    };

    let root_main = tree
        .files()
        .any(|f| !f.contains('/') && f.ends_with(".go") && !f.ends_with("_test.go") && is_main(f));
    let last = module.rsplit('/').next().unwrap_or(&module).to_owned();
    let (main_package, binary) = if root_main {
        (Some(".".to_owned()), last)
    } else {
        // cmd/<name>/*.go, preferring the one named like the module.
        let mut cmds: Vec<String> = tree
            .files()
            .filter(|f| {
                f.starts_with("cmd/")
                    && f.matches('/').count() == 2
                    && f.ends_with(".go")
                    && is_main(f)
            })
            .filter_map(|f| f.split('/').nth(1).map(str::to_owned))
            .collect();
        cmds.sort();
        cmds.dedup();
        let pick = cmds.iter().find(|c| **c == last).or(cmds.first()).cloned();
        match pick {
            Some(c) => (Some(format!("./cmd/{c}")), c),
            None => (None, last),
        }
    };
    Some(GoInfo {
        module,
        main_package,
        binary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(files: &[(&str, &str)]) -> (tempfile::TempDir, FileTree) {
        let dir = tempfile::tempdir().unwrap();
        for (path, body) in files {
            let p = dir.path().join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        let t = FileTree::scan(dir.path()).unwrap();
        (dir, t)
    }

    #[test]
    fn root_main_package() {
        let (_d, t) = tree(&[
            ("go.mod", "module github.com/junegunn/fzf\n\ngo 1.23\n"),
            ("main.go", "package main\n"),
        ]);
        let g = detect(&t).unwrap();
        assert_eq!(
            (g.main_package.as_deref(), g.binary.as_str()),
            (Some("."), "fzf")
        );
    }

    #[test]
    fn cmd_folder() {
        let (_d, t) = tree(&[
            ("go.mod", "module example.com/tools\n"),
            ("lib.go", "package tools\n"),
            ("cmd/helper/main.go", "package main\n"),
            ("cmd/tools/main.go", "package main\n"),
        ]);
        let g = detect(&t).unwrap();
        assert_eq!(
            (g.main_package.as_deref(), g.binary.as_str()),
            (Some("./cmd/tools"), "tools")
        );
    }

    #[test]
    fn library_has_nothing_to_build() {
        let (_d, t) = tree(&[
            ("go.mod", "module example.com/lib\n"),
            ("lib.go", "package lib\n"),
        ]);
        assert_eq!(detect(&t).unwrap().main_package, None);
    }
}
