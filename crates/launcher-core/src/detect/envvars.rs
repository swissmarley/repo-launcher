//! Finds settings a project reads from its environment (API keys and the like), so the user
//! can be asked for them before launch. Sources: `.env.example`-style files, and reads such as
//! `os.getenv("X")` / `process.env.X` in the project's own code.

use super::FileTree;
use crate::recipe::keys;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::LazyLock;

const EXAMPLE_FILES: &[&str] = &[
    ".env.example",
    ".env.sample",
    ".env.template",
    ".env.dist",
    "example.env",
    "sample.env",
    "env.example",
];

const CODE_EXTS: &[&str] = &[".py", ".js", ".mjs", ".cjs", ".ts", ".tsx", ".jsx", ".mts"];
const MAX_CODE_FILES: usize = 400;

/// Variables that are about the runtime, not something to ask a person for.
const IGNORE: &[&str] = &[
    "PORT",
    "HOST",
    "HOSTNAME",
    "NODE_ENV",
    "ENV",
    "DEBUG",
    "PATH",
    "HOME",
    "USER",
    "PWD",
    "SHELL",
    "TERM",
    "LANG",
    "TZ",
    "TMPDIR",
    "TEMP",
    "CI",
    "PYTHONPATH",
    "VIRTUAL_ENV",
    "LOG_LEVEL",
    "LOGLEVEL",
    "VERBOSE",
    "ENVIRONMENT",
    "APP_ENV",
    "FLASK_ENV",
    "FLASK_APP",
    "FLASK_DEBUG",
    "DJANGO_SETTINGS_MODULE",
    "BASE_URL",
    "PUBLIC_URL",
    "BROWSER",
    "NODE_OPTIONS",
    "GRADIO_SERVER_PORT",
    "GRADIO_SERVER_NAME",
    "STREAMLIT_SERVER_PORT",
    "VERCEL",
    "VERCEL_URL",
    "NEXT_RUNTIME",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "XDG_DATA_HOME",
    "APPDATA",
    "LOCALAPPDATA",
    "USERPROFILE",
    "CUDA_VISIBLE_DEVICES",
    "WORKERS",
    "THREADS",
    "TIMEOUT",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvHint {
    pub name: String,
    /// The code fails without it (`os.environ["X"]`).
    pub required: bool,
    pub secret: bool,
}

static PY_GET: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"(?:os\.getenv|os\.environ\.get|environ\.get|getenv)\(\s*["']([A-Z][A-Z0-9_]{2,})["']"#,
    )
    .expect("pattern")
});
static PY_INDEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"environ\[\s*["']([A-Z][A-Z0-9_]{2,})["']\s*\]"#).expect("pattern")
});
static JS_ENV: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"(?:process\.env|import\.meta\.env|Deno\.env\.get\(|Bun\.env)(?:\.|\[\s*["']|\s*["'])([A-Z][A-Z0-9_]{2,})"#,
    )
    .expect("pattern")
});

pub fn looks_secret(name: &str) -> bool {
    const WORDS: &[&str] = &[
        "KEY",
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "CREDENTIAL",
        "AUTH",
    ];
    let upper = name.to_ascii_uppercase();
    WORDS.iter().any(|w| upper.contains(w))
}

fn worth_asking(name: &str) -> bool {
    !IGNORE.contains(&name)
        && !name.starts_with("npm_")
        && !name.starts_with("NEXT_PUBLIC_VERCEL")
        && (keys::lookup(name).is_some() || looks_secret(name))
}

pub fn scan(tree: &FileTree) -> Vec<EnvHint> {
    let mut found: BTreeMap<String, EnvHint> = BTreeMap::new();
    let mut add = |name: &str, required: bool, from_example: bool| {
        // Example files list settings deliberately, so their non-secret entries count too.
        if !(worth_asking(name) || (from_example && !IGNORE.contains(&name))) {
            return;
        }
        let e = found.entry(name.to_owned()).or_insert(EnvHint {
            name: name.to_owned(),
            required: false,
            secret: looks_secret(name) || keys::lookup(name).is_some(),
        });
        e.required |= required;
    };

    for file in EXAMPLE_FILES {
        if let Some(text) = tree.read(file) {
            for name in example_names(&text) {
                add(&name, false, true);
            }
        }
    }

    let code = tree
        .files()
        .filter(|f| CODE_EXTS.iter().any(|e| f.ends_with(e)))
        .filter(|f| !is_test_or_vendor(f))
        .take(MAX_CODE_FILES)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for file in code {
        let Some(text) = tree.read(&file) else {
            continue;
        };
        for c in PY_GET.captures_iter(&text) {
            add(&c[1], false, false);
        }
        for c in PY_INDEX.captures_iter(&text) {
            add(&c[1], true, false);
        }
        for c in JS_ENV.captures_iter(&text) {
            add(&c[1], false, false);
        }
    }
    found.into_values().collect()
}

fn is_test_or_vendor(path: &str) -> bool {
    path.split('/').any(|seg| {
        matches!(
            seg,
            "test" | "tests" | "__tests__" | "spec" | "e2e" | "vendor" | "third_party" | "examples"
        )
    }) || path.contains(".test.")
        || path.contains(".spec.")
        || path
            .rsplit('/')
            .next()
            .is_some_and(|f| f.starts_with("test_"))
}

/// Names from `NAME=value` lines (comments, `export` and blank lines allowed).
fn example_names(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.starts_with('#') {
                return None;
            }
            let l = l.strip_prefix("export ").unwrap_or(l);
            let (name, _) = l.split_once('=')?;
            let name = name.trim();
            let valid = name.len() >= 2
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
            valid.then(|| name.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(files: &[(&str, &str)]) -> (tempfile::TempDir, FileTree) {
        let dir = tempfile::tempdir().unwrap();
        for (name, body) in files {
            let p = dir.path().join(name);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        let t = FileTree::scan(dir.path()).unwrap();
        (dir, t)
    }

    #[test]
    fn finds_keys_in_code_and_example_files() {
        let (_d, t) = tree(&[
            (
                "app.py",
                "import os\nkey = os.getenv('OPENAI_API_KEY')\nport = os.getenv('PORT')\ndb = os.environ['DATABASE_PASSWORD']\n",
            ),
            (
                "web/index.ts",
                "const k = process.env.ANTHROPIC_API_KEY ?? ''\nprocess.env['NODE_ENV']",
            ),
            (
                ".env.example",
                "# keys\nHF_TOKEN=\nMODEL_NAME=gpt-4o\nexport SERPAPI_API_KEY=your-key\n",
            ),
            ("tests/test_app.py", "os.getenv('TEST_ONLY_SECRET')"),
        ]);
        let hints = scan(&t);
        let names: Vec<&str> = hints.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "ANTHROPIC_API_KEY",
                "DATABASE_PASSWORD",
                "HF_TOKEN",
                "MODEL_NAME",
                "OPENAI_API_KEY",
                "SERPAPI_API_KEY"
            ]
        );
        let get = |n: &str| hints.iter().find(|h| h.name == n).unwrap();
        assert!(get("DATABASE_PASSWORD").required);
        assert!(!get("OPENAI_API_KEY").required);
        assert!(
            !get("MODEL_NAME").secret,
            "plain settings from .env.example aren't secret"
        );
        assert!(get("HF_TOKEN").secret);
    }
}
