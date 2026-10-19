use super::{FileTree, Framework};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonInfo {
    /// Requirements file to install, if any (root `requirements.txt` or a production variant).
    pub requirements: Option<String>,
    pub has_uv_lock: bool,
    /// Has a build backend (`[build-system]`, `setup.py`, `setup.cfg`), so `pip install .` works.
    pub installable: bool,
    /// `[project].dependencies` from a pyproject without a build backend.
    pub pyproject_dependencies: Vec<String>,
    pub requires_python: Option<String>,
    /// From `.python-version` or `runtime.txt`.
    pub pinned_version: Option<String>,
    /// Normalized names from every source we read.
    pub dependencies: BTreeSet<String>,
    pub scripts: Vec<String>,
    pub frameworks: Vec<Framework>,
    /// Best first.
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EntryKind {
    Streamlit,
    Gradio,
    Flask {
        app: String,
    },
    FastApi {
        app: String,
    },
    Django,
    Script,
    /// A script that opens its own window (tkinter, pygame, Qt…).
    Gui,
    Module {
        module: String,
    },
    ConsoleScript {
        name: String,
    },
    /// Only notebooks to run: open them in JupyterLab.
    Notebooks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub file: String,
    #[serde(flatten)]
    pub kind: EntryKind,
    pub score: i32,
}

const MARKERS: &[&str] = &[
    "requirements.txt",
    "pyproject.toml",
    "setup.py",
    "setup.cfg",
    "Pipfile",
    "uv.lock",
    "environment.yml",
];
const PREFERRED_NAMES: &[&str] = &[
    "streamlit_app.py",
    "app.py",
    "main.py",
    "Home.py",
    "Hello.py",
    "demo.py",
    "run.py",
];
/// Modules whose import means the script opens a window rather than talking in a terminal.
const GUI_MODULES: &[&str] = &[
    "tkinter",
    "customtkinter",
    "ttkbootstrap",
    "pygame",
    "PyQt5",
    "PyQt6",
    "PySide2",
    "PySide6",
    "wx",
    "kivy",
    "pyglet",
    "arcade",
];
const IGNORED_DIRS: &[&str] = &[
    "tests",
    "test",
    "docs",
    "examples",
    "scripts",
    "pages",
    "benchmarks",
];

pub fn detect(tree: &FileTree) -> Option<PythonInfo> {
    let has_marker = MARKERS.iter().any(|m| tree.has(m));
    let root_py = tree.files().any(|f| !f.contains('/') && f.ends_with(".py"));
    let notebooks = has_notebooks(tree);
    if !has_marker && !root_py && !notebooks {
        return None;
    }

    let mut info = PythonInfo {
        has_uv_lock: tree.has("uv.lock"),
        installable: tree.has("setup.py") || tree.has("setup.cfg"),
        ..Default::default()
    };

    info.requirements = pick_requirements(tree);
    if let Some(req) = info.requirements.clone() {
        read_requirements(tree, &req, &mut info.dependencies, 0);
    }
    if let Some(text) = tree.read("pyproject.toml") {
        read_pyproject(&text, &mut info);
    }
    info.pinned_version = tree
        .read(".python-version")
        .and_then(|s| s.lines().next().map(|l| l.trim().to_owned()))
        .or_else(|| {
            tree.read("runtime.txt")
                .and_then(|s| s.trim().strip_prefix("python-").map(str::to_owned))
        })
        .filter(|v| !v.is_empty());

    let readme_hints = readme_hints(tree);
    info.entries = find_entries(tree, &info, &readme_hints);
    info.entries
        .sort_by(|a, b| b.score.cmp(&a.score).then(a.file.cmp(&b.file)));

    for (dep, fw) in [
        ("streamlit", Framework::Streamlit),
        ("gradio", Framework::Gradio),
        ("flask", Framework::Flask),
        ("fastapi", Framework::FastApi),
        ("django", Framework::Django),
    ] {
        let used = info
            .entries
            .iter()
            .any(|e| framework_of(&e.kind) == Some(fw));
        if info.dependencies.contains(dep) || used {
            info.frameworks.push(fw);
        }
    }
    if notebooks {
        info.frameworks.push(Framework::Jupyter);
    }
    Some(info)
}

/// Notebooks meant to be opened: not the test data of a library that reads notebooks.
fn has_notebooks(tree: &FileTree) -> bool {
    const TEST_DIRS: &[&str] = &[
        "test",
        "tests",
        "testing",
        "test_files",
        "test_data",
        "testdata",
        "fixtures",
        "__tests__",
    ];
    tree.files()
        .any(|f| f.ends_with(".ipynb") && !f.split('/').any(|part| TEST_DIRS.contains(&part)))
}

fn framework_of(kind: &EntryKind) -> Option<Framework> {
    match kind {
        EntryKind::Streamlit => Some(Framework::Streamlit),
        EntryKind::Gradio => Some(Framework::Gradio),
        EntryKind::Flask { .. } => Some(Framework::Flask),
        EntryKind::FastApi { .. } => Some(Framework::FastApi),
        EntryKind::Django => Some(Framework::Django),
        _ => None,
    }
}

fn pick_requirements(tree: &FileTree) -> Option<String> {
    [
        "requirements.txt",
        "requirements/prod.txt",
        "requirements/production.txt",
        "requirements/base.txt",
        "requirements/main.txt",
    ]
    .into_iter()
    .find(|p| tree.has(p))
    .map(str::to_owned)
}

fn read_requirements(tree: &FileTree, rel: &str, out: &mut BTreeSet<String>, depth: u8) {
    let Some(text) = tree.read(rel) else { return };
    let dir = rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    for line in text.lines() {
        let line = line.split(" #").next().unwrap_or("").trim();
        if let Some(inc) = line
            .strip_prefix("-r ")
            .or_else(|| line.strip_prefix("--requirement "))
        {
            if depth < 3 {
                let inc = inc.trim();
                let path = if dir.is_empty() {
                    inc.to_owned()
                } else {
                    format!("{dir}/{inc}")
                };
                read_requirements(tree, &path, out, depth + 1);
            }
            continue;
        }
        if let Some(name) = requirement_name(line) {
            out.insert(name);
        }
    }
}

/// Extracts the normalized distribution name from a PEP 508 line.
pub fn requirement_name(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
        return None;
    }
    if let Some((name, _)) = line.split_once(" @ ") {
        return Some(normalize(name.trim()));
    }
    let name: String = line
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect();
    (!name.is_empty() && !line.contains("://")).then(|| normalize(&name))
}

/// PEP 503 normalization.
pub fn normalize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_sep = false;
    for c in name.chars() {
        if matches!(c, '-' | '_' | '.') {
            if !last_sep {
                out.push('-');
            }
            last_sep = true;
        } else {
            out.push(c.to_ascii_lowercase());
            last_sep = false;
        }
    }
    out
}

fn read_pyproject(text: &str, info: &mut PythonInfo) {
    let Ok(doc) = text.parse::<toml::Table>() else {
        return;
    };
    let project = doc.get("project").and_then(|p| p.as_table());
    let poetry = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.as_table());
    if doc.contains_key("build-system") {
        info.installable = true;
    }
    if let Some(p) = project {
        let deps: Vec<String> = p
            .get("dependencies")
            .and_then(|d| d.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        info.dependencies
            .extend(deps.iter().filter_map(|d| requirement_name(d)));
        if !info.installable {
            info.pyproject_dependencies = deps;
        }
        info.requires_python = p
            .get("requires-python")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        if let Some(scripts) = p.get("scripts").and_then(|s| s.as_table()) {
            info.scripts.extend(scripts.keys().cloned());
        }
    }
    if let Some(p) = poetry {
        if let Some(deps) = p.get("dependencies").and_then(|d| d.as_table()) {
            for (name, spec) in deps {
                if name == "python" {
                    if info.requires_python.is_none() {
                        info.requires_python = spec.as_str().map(poetry_to_pep440);
                    }
                } else {
                    info.dependencies.insert(normalize(name));
                }
            }
        }
        if let Some(scripts) = p.get("scripts").and_then(|s| s.as_table()) {
            info.scripts.extend(scripts.keys().cloned());
        }
    }
}

/// Converts the common Poetry forms (`^3.10`, `~3.11`, `>=3.9,<4`) to PEP 440.
fn poetry_to_pep440(spec: &str) -> String {
    let spec = spec.trim();
    if let Some(v) = spec.strip_prefix('^') {
        let major = v.split('.').next().unwrap_or("3");
        let next = major.parse::<u32>().map(|m| m + 1).unwrap_or(4);
        format!(">={v},<{next}")
    } else if let Some(v) = spec.strip_prefix('~').filter(|v| !v.starts_with('=')) {
        format!("~={v}")
    } else {
        spec.to_owned()
    }
}

/// Script names mentioned in run instructions, e.g. `streamlit run app.py`.
fn readme_hints(tree: &FileTree) -> Vec<String> {
    let Some(readme) = [
        "README.md",
        "README.rst",
        "README.txt",
        "README",
        "readme.md",
    ]
    .iter()
    .find_map(|r| tree.read(r)) else {
        return Vec::new();
    };
    let mut hints = Vec::new();
    for line in readme.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        for w in words.windows(2) {
            // Also inline code in a sentence: "Start it with `python app.py`."
            let quote = |c: char| matches!(c, '`' | '"' | '\'' | '$');
            let cmd_ok = matches!(
                w[0].trim_start_matches(quote),
                "run" | "python" | "python3" | "py"
            );
            let file = w[1]
                .trim_matches(quote)
                .trim_end_matches(['.', ',', ';', ':', ')'])
                .trim_end_matches(quote);
            if cmd_ok && file.ends_with(".py") {
                hints.push(file.trim_start_matches("./").to_owned());
            }
        }
    }
    hints
}

fn find_entries(tree: &FileTree, info: &PythonInfo, hints: &[String]) -> Vec<Entry> {
    let mut out = Vec::new();
    if tree.has("manage.py") {
        out.push(Entry {
            file: "manage.py".into(),
            kind: EntryKind::Django,
            score: 60,
        });
    }
    for f in tree.files() {
        let depth = f.matches('/').count();
        let first = f.split('/').next().unwrap_or("");
        if !f.ends_with(".py") || depth > 1 || f == "setup.py" || f == "manage.py" {
            continue;
        }
        if depth == 1 && IGNORED_DIRS.contains(&first) {
            continue;
        }
        let name = f.rsplit('/').next().unwrap_or(f);
        if name.starts_with("test_") || name == "conftest.py" {
            continue;
        }
        if name == "__main__.py" && depth == 1 {
            out.push(Entry {
                file: f.into(),
                kind: EntryKind::Module {
                    module: first.into(),
                },
                score: 30,
            });
            continue;
        }
        let Some(src) = tree.read(f) else { continue };
        let mut score = if depth == 0 { 10 } else { 0 };
        let preferred = PREFERRED_NAMES.contains(&name);
        let hinted = hints.iter().any(|h| h == f);
        if preferred {
            score += 20;
        }
        if hinted {
            score += 30;
        }

        let kind = if src.contains("import streamlit") || src.contains("from streamlit") {
            if src.contains("set_page_config") {
                score += 5;
            }
            Some(EntryKind::Streamlit)
        } else if (src.contains("import gradio") || src.contains("from gradio"))
            && src.contains(".launch(")
        {
            Some(EntryKind::Gradio)
        } else if let Some((var, in_factory)) = assigned_var(&src, "Flask(") {
            let app = if in_factory {
                module_path(f)
            } else {
                format!("{}:{var}", module_path(f))
            };
            Some(EntryKind::Flask { app })
        } else if let Some((var, _)) = assigned_var(&src, "FastAPI(") {
            Some(EntryKind::FastApi {
                app: format!("{}:{var}", module_path(f)),
            })
        } else if depth == 0
            && ((src.contains("__name__") && src.contains("__main__"))
                // Many scripts just run at the top level, without a `__main__` guard.
                || ((preferred || hinted) && runs_at_top_level(&src)))
        {
            Some(if imports_any(&src, GUI_MODULES) {
                EntryKind::Gui
            } else {
                EntryKind::Script
            })
        } else {
            None
        };
        if let Some(kind) = kind {
            let bonus = match kind {
                EntryKind::Script | EntryKind::Gui => 0,
                _ => 20,
            };
            out.push(Entry {
                file: f.into(),
                kind,
                score: score + bonus,
            });
        }
    }
    if out.is_empty() && info.scripts.is_empty() && has_notebooks(tree) {
        out.push(Entry {
            file: String::new(),
            kind: EntryKind::Notebooks,
            score: 5,
        });
    }
    for name in &info.scripts {
        out.push(Entry {
            file: String::new(),
            kind: EntryKind::ConsoleScript { name: name.clone() },
            score: 25,
        });
    }
    out
}

/// Whether the module does something when run: a call at the top level, such as `main()` or
/// `root.mainloop()`, not only definitions and imports.
fn runs_at_top_level(src: &str) -> bool {
    const NOT_CALLS: &[&str] = &[
        "def ", "class ", "import ", "from ", "if ", "elif ", "else", "for ", "while ", "with ",
        "try", "except", "finally", "return", "async ", "@", "#",
    ];
    src.lines().any(|line| {
        line.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && !NOT_CALLS.iter().any(|k| line.starts_with(k))
            && line
                .split_once('(')
                .is_some_and(|(head, _)| !head.contains('='))
    })
}

/// Whether the source imports any of `modules` (`import x`, `import x.y`, `from x import …`).
fn imports_any(src: &str, modules: &[&str]) -> bool {
    src.lines().any(|line| {
        let line = line.trim_start();
        let names = if let Some(rest) = line.strip_prefix("import ") {
            rest
        } else if let Some(rest) = line.strip_prefix("from ") {
            rest.split_whitespace().next().unwrap_or("")
        } else {
            return false;
        };
        names.split(',').any(|n| {
            let top = n.trim().split([' ', '.']).next().unwrap_or("");
            modules.contains(&top)
        })
    })
}

/// Finds `name = Flask(` style assignments. Returns the name and whether it was inside a
/// function (an app factory such as `create_app()`), in which case the module alone is the
/// right target: Flask looks for the factory itself.
fn assigned_var(src: &str, ctor: &str) -> Option<(String, bool)> {
    src.lines().find_map(|line| {
        let (lhs, rhs) = line.split_once('=')?;
        let indented = line.starts_with([' ', '\t']);
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        let valid = !lhs.is_empty() && lhs.chars().all(|c| c.is_alphanumeric() || c == '_');
        (valid && (rhs.starts_with(ctor) || rhs.contains(&format!(".{ctor}"))))
            .then(|| (lhs.to_owned(), indented))
    })
}

/// `pkg/app.py` → `pkg.app`; a package's `__init__.py` is the package itself.
fn module_path(file: &str) -> String {
    file.trim_end_matches(".py")
        .trim_end_matches("/__init__")
        .replace('/', ".")
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
    fn names_are_normalized() {
        assert_eq!(
            requirement_name("Flask_Cors>=3.0 ; python_version>'3'"),
            Some("flask-cors".into())
        );
        assert_eq!(requirement_name("torch==2.4.0+cpu"), Some("torch".into()));
        assert_eq!(
            requirement_name("mypkg @ https://x/y.whl"),
            Some("mypkg".into())
        );
        assert_eq!(requirement_name("-e ."), None);
        assert_eq!(requirement_name("# comment"), None);
        assert_eq!(requirement_name("git+https://github.com/a/b"), None);
    }

    #[test]
    fn streamlit_app_with_readme_hint() {
        let (_d, t) = tree(&[
            (
                "requirements.txt",
                "streamlit==1.40\npandas\n-r extra.txt\n",
            ),
            ("extra.txt", "altair\n"),
            ("utils.py", "import streamlit as st\n"),
            ("dashboard.py", "import streamlit as st\nst.title('x')\n"),
            ("README.md", "Run it with `streamlit run dashboard.py`\n"),
            ("pages/1_Other.py", "import streamlit as st\n"),
        ]);
        let info = detect(&t).unwrap();
        assert_eq!(info.requirements.as_deref(), Some("requirements.txt"));
        assert!(info.dependencies.contains("altair"));
        assert_eq!(info.frameworks, vec![Framework::Streamlit]);
        assert_eq!(info.entries[0].file, "dashboard.py");
        assert!(info.entries.iter().all(|e| !e.file.starts_with("pages/")));
    }

    #[test]
    fn scripts_without_a_main_guard_and_windowed_apps() {
        // A tkinter app that just runs: no `if __name__ == "__main__"`.
        let (_d, t) = tree(&[
            (
                "app.py",
                "import tkinter as tk\nroot = tk.Tk()\nroot.title('x')\nroot.mainloop()\n",
            ),
            ("requirements.txt", "cryptography\n"),
        ]);
        let info = detect(&t).unwrap();
        assert_eq!(info.entries[0].kind, EntryKind::Gui);
        assert_eq!(info.entries[0].file, "app.py");

        // A plain script named like an entry point, and one the README says to run.
        let (_d, t) = tree(&[
            ("main.py", "def main():\n    print('hi')\n\nmain()\n"),
            ("tool.py", "import sys\nprint(sys.argv)\n"),
            ("README.md", "Run it with `python tool.py`.\n"),
        ]);
        let info = detect(&t).unwrap();
        let files: Vec<_> = info.entries.iter().map(|e| e.file.as_str()).collect();
        assert_eq!(files, ["tool.py", "main.py"]);
        assert!(info.entries.iter().all(|e| e.kind == EntryKind::Script));

        // Helpers that only define things, or other unguarded files, aren't entry points.
        let (_d, t) = tree(&[
            (
                "main.py",
                "import os\nX = os.getcwd()\n\ndef run():\n    pass\n",
            ),
            ("helpers.py", "print('side effect')\n"),
            ("requirements.txt", "requests\n"),
        ]);
        assert!(detect(&t).unwrap().entries.is_empty());

        // A guarded pygame game is a windowed app too.
        let (_d, t) = tree(&[(
            "game.py",
            "import pygame, sys\n\nif __name__ == '__main__':\n    pygame.init()\n",
        )]);
        assert_eq!(detect(&t).unwrap().entries[0].kind, EntryKind::Gui);
    }

    #[test]
    fn gradio_flask_fastapi() {
        let (_d, t) = tree(&[
            (
                "app.py",
                "import gradio as gr\ndemo = gr.Interface(fn=f)\ndemo.launch()\n",
            ),
            (
                "web/server.py",
                "from flask import Flask\nserver = Flask(__name__)\n",
            ),
            ("api.py", "from fastapi import FastAPI\napi = FastAPI()\n"),
        ]);
        let info = detect(&t).unwrap();
        assert_eq!(info.entries[0].kind, EntryKind::Gradio);
        assert!(info.entries.iter().any(|e| e.kind
            == EntryKind::Flask {
                app: "web.server:server".into()
            }));
        assert!(info.entries.iter().any(|e| e.kind
            == EntryKind::FastApi {
                app: "api:api".into()
            }));
    }

    #[test]
    fn pyproject_cli_and_poetry() {
        let (_d, t) = tree(&[(
            "pyproject.toml",
            r#"
[build-system]
requires = ["poetry-core"]
[tool.poetry.dependencies]
python = "^3.10"
Rich = "*"
[tool.poetry.scripts]
hello = "hello.cli:main"
"#,
        )]);
        let info = detect(&t).unwrap();
        assert!(info.installable);
        assert_eq!(info.requires_python.as_deref(), Some(">=3.10,<4"));
        assert!(info.dependencies.contains("rich"));
        assert_eq!(
            info.entries[0].kind,
            EntryKind::ConsoleScript {
                name: "hello".into()
            }
        );
    }

    #[test]
    fn pyproject_without_build_backend_installs_deps_only() {
        let (_d, t) = tree(&[(
            "pyproject.toml",
            "[project]\nname='x'\nrequires-python='>=3.11'\ndependencies=['streamlit>=1.30','httpx']\n",
        )]);
        let info = detect(&t).unwrap();
        assert!(!info.installable);
        assert_eq!(
            info.pyproject_dependencies,
            vec!["streamlit>=1.30", "httpx"]
        );
        assert_eq!(info.requires_python.as_deref(), Some(">=3.11"));
    }

    #[test]
    fn test_data_notebooks_dont_make_a_notebook_project() {
        // A library that converts notebooks keeps one in its tests (microsoft/markitdown).
        let (_d, t) = tree(&[
            ("README.md", "# A converter\n"),
            (
                "packages/conv/tests/test_files/sample.ipynb",
                r#"{"cells": []}"#,
            ),
        ]);
        assert!(detect(&t).is_none());

        let (_d, t) = tree(&[("notebooks/01-intro.ipynb", r#"{"cells": []}"#)]);
        assert_eq!(detect(&t).unwrap().entries[0].kind, EntryKind::Notebooks);
    }

    #[test]
    fn not_python() {
        let (_d, t) = tree(&[("package.json", "{}"), ("src/index.py", "")]);
        assert!(detect(&t).is_none());
    }
}
