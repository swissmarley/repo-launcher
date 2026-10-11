//! Node project detection from `package.json` and lockfiles.

use super::FileTree;
use crate::recipe::PackageManager;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeFramework {
    Next,
    Vite,
    Astro,
    Nuxt,
    SvelteKit,
    Remix,
    Gatsby,
    CreateReactApp,
    Express,
    Electron,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub package_manager: Option<PackageManager>,
    pub package_manager_version: Option<String>,
    pub lockfile: Option<String>,
    /// Major version from `engines.node` (lower bound), if stated.
    pub node_major: Option<u32>,
    /// The folder (relative, "" for root) holding the app to run; differs from root in monorepos.
    pub app_dir: String,
    pub name: Option<String>,
    pub scripts: BTreeMap<String, String>,
    pub dependencies: BTreeSet<String>,
    /// `bin` entries: (command name, file).
    pub bins: Vec<(String, String)>,
    pub main: Option<String>,
    pub frameworks: Vec<NodeFramework>,
    pub workspaces: bool,
}

impl NodeInfo {
    pub fn has(&self, fw: NodeFramework) -> bool {
        self.frameworks.contains(&fw)
    }
}

pub fn detect(tree: &FileTree) -> Option<NodeInfo> {
    let root: Value = serde_json::from_str(&tree.read("package.json")?).ok()?;
    let mut info = NodeInfo::default();

    if let Some(pm) = root.get("packageManager").and_then(Value::as_str) {
        let (name, version) = pm.split_once('@').unwrap_or((pm, ""));
        // "pnpm@9.12.0+sha512.abc" → version "9.12.0"
        let version = version.split('+').next().unwrap_or("").trim();
        info.package_manager = parse_pm(name);
        info.package_manager_version = (!version.is_empty()).then(|| version.to_owned());
    }
    for (file, pm) in [
        ("pnpm-lock.yaml", PackageManager::Pnpm),
        ("yarn.lock", PackageManager::Yarn),
        ("bun.lock", PackageManager::Bun),
        ("bun.lockb", PackageManager::Bun),
        ("package-lock.json", PackageManager::Npm),
        ("npm-shrinkwrap.json", PackageManager::Npm),
    ] {
        if tree.has(file) {
            info.lockfile = Some(file.to_owned());
            info.package_manager.get_or_insert(pm);
            break;
        }
    }
    if info.package_manager == Some(PackageManager::Pnpm) && info.package_manager_version.is_none()
    {
        info.package_manager_version = pnpm_for_lockfile(tree.read("pnpm-lock.yaml").as_deref());
    }
    if info.package_manager == Some(PackageManager::Yarn) && info.package_manager_version.is_none()
    {
        // Yarn 2+ lockfiles start with a __metadata block.
        let berry = tree
            .read("yarn.lock")
            .is_some_and(|l| l.contains("__metadata:"))
            || tree.has(".yarnrc.yml");
        info.package_manager_version = Some(if berry { "4".into() } else { "1.22.22".into() });
    }

    info.node_major = root
        .pointer("/engines/node")
        .and_then(Value::as_str)
        .and_then(node_major)
        .or_else(|| {
            tree.read(".nvmrc")
                .or_else(|| tree.read(".node-version"))
                .and_then(|v| node_major(&v))
        });
    info.workspaces = root.get("workspaces").is_some() || tree.has("pnpm-workspace.yaml");

    // In a monorepo whose root has nothing to run, pick the app package.
    let mut pkg = root.clone();
    if info.workspaces
        && !has_runnable_script(&root)
        && let Some((dir, sub)) = pick_workspace_app(tree)
    {
        info.app_dir = dir;
        pkg = sub;
    }
    read_package(&pkg, &mut info);
    let app_prefix = if info.app_dir.is_empty() {
        String::new()
    } else {
        format!("{}/", info.app_dir)
    };
    let has_next_config = tree.files().any(|f| {
        let rest = f.strip_prefix(&app_prefix).unwrap_or(f);
        !rest.contains('/') && rest.starts_with("next.config.")
    });
    if has_next_config {
        push(&mut info.frameworks, NodeFramework::Next);
    }
    Some(info)
}

fn push(v: &mut Vec<NodeFramework>, fw: NodeFramework) {
    if !v.contains(&fw) {
        v.push(fw);
    }
}

fn read_package(pkg: &Value, info: &mut NodeInfo) {
    info.name = pkg.get("name").and_then(Value::as_str).map(str::to_owned);
    if let Some(scripts) = pkg.get("scripts").and_then(Value::as_object) {
        info.scripts = scripts
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
            .collect();
    }
    for key in ["dependencies", "devDependencies"] {
        if let Some(deps) = pkg.get(key).and_then(Value::as_object) {
            info.dependencies.extend(deps.keys().cloned());
        }
    }
    info.main = pkg.get("main").and_then(Value::as_str).map(str::to_owned);
    match pkg.get("bin") {
        Some(Value::String(file)) => {
            let name = info.name.clone().unwrap_or_else(|| "cli".into());
            let name = name.rsplit('/').next().unwrap_or(&name).to_owned();
            info.bins.push((name, file.clone()));
        }
        Some(Value::Object(map)) => {
            info.bins.extend(
                map.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned()))),
            );
        }
        _ => {}
    }
    let has = |d: &str| info.dependencies.contains(d);
    let mut fws = Vec::new();
    if has("next") {
        fws.push(NodeFramework::Next);
    }
    if has("@sveltejs/kit") {
        fws.push(NodeFramework::SvelteKit);
    }
    if has("astro") {
        fws.push(NodeFramework::Astro);
    }
    if has("nuxt") || has("nuxt3") {
        fws.push(NodeFramework::Nuxt);
    }
    if has("@remix-run/dev") || has("@react-router/dev") {
        fws.push(NodeFramework::Remix);
    }
    if has("gatsby") {
        fws.push(NodeFramework::Gatsby);
    }
    if has("react-scripts") {
        fws.push(NodeFramework::CreateReactApp);
    }
    if has("vite") {
        fws.push(NodeFramework::Vite);
    }
    if has("electron") {
        fws.push(NodeFramework::Electron);
    }
    if has("express") || has("fastify") || has("koa") || has("hono") {
        fws.push(NodeFramework::Express);
    }
    for fw in fws {
        push(&mut info.frameworks, fw);
    }
}

fn has_runnable_script(pkg: &Value) -> bool {
    pkg.get("scripts")
        .and_then(Value::as_object)
        .is_some_and(|s| s.contains_key("dev") || s.contains_key("start"))
}

/// Prefers `apps/web`-style folders, then any workspace package with a dev script.
fn pick_workspace_app(tree: &FileTree) -> Option<(String, Value)> {
    let mut candidates: Vec<(i32, String, Value)> = tree
        .files()
        .filter(|f| f.ends_with("/package.json") && f.matches('/').count() == 2)
        .filter_map(|f| {
            let dir = f.trim_end_matches("/package.json").to_owned();
            let pkg: Value = serde_json::from_str(&tree.read(f)?).ok()?;
            if !has_runnable_script(&pkg) {
                return None;
            }
            let mut score = 0;
            if dir.starts_with("apps/") || dir.starts_with("examples/") {
                score += 10;
            }
            let leaf = dir.rsplit('/').next().unwrap_or("");
            if matches!(leaf, "web" | "app" | "frontend" | "site" | "www" | "client") {
                score += 10;
            }
            if pkg.pointer("/scripts/dev").is_some() {
                score += 5;
            }
            Some((score, dir, pkg))
        })
        .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    candidates.into_iter().next().map(|(_, d, p)| (d, p))
}

fn parse_pm(name: &str) -> Option<PackageManager> {
    match name {
        "npm" => Some(PackageManager::Npm),
        "pnpm" => Some(PackageManager::Pnpm),
        "yarn" => Some(PackageManager::Yarn),
        "bun" => Some(PackageManager::Bun),
        _ => None,
    }
}

/// Picks a pnpm major that reads the lockfile format.
fn pnpm_for_lockfile(lock: Option<&str>) -> Option<String> {
    let version = lock?
        .lines()
        .find_map(|l| l.strip_prefix("lockfileVersion:"))?
        .trim()
        .trim_matches('\'');
    let major: u32 = version.split('.').next()?.parse().ok()?;
    Some(
        match major {
            9.. => "10",
            6..=8 => "8",
            _ => "7",
        }
        .into(),
    )
}

/// The lowest major a range allows: ">=18", "^20.11", "20.x", "v22.1.0", "lts/*" (None).
pub fn node_major(range: &str) -> Option<u32> {
    let first = range.split("||").next()?.trim();
    let digits: String = first
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
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
    fn node_major_ranges() {
        assert_eq!(node_major(">=18"), Some(18));
        assert_eq!(node_major("^20.11.0"), Some(20));
        assert_eq!(node_major("20.x"), Some(20));
        assert_eq!(node_major("v22.1.0\n"), Some(22));
        assert_eq!(node_major("18 || 20"), Some(18));
        assert_eq!(node_major("lts/*"), None);
    }

    #[test]
    fn vite_app_with_pnpm() {
        let (_d, t) = tree(&[
            (
                "package.json",
                r#"{"name":"app","packageManager":"pnpm@9.12.0+sha512.x","scripts":{"dev":"vite"},"devDependencies":{"vite":"^6"},"engines":{"node":">=20"}}"#,
            ),
            ("pnpm-lock.yaml", "lockfileVersion: '9.0'\n"),
        ]);
        let info = detect(&t).unwrap();
        assert_eq!(info.package_manager, Some(PackageManager::Pnpm));
        assert_eq!(info.package_manager_version.as_deref(), Some("9.12.0"));
        assert_eq!(info.node_major, Some(20));
        assert!(info.has(NodeFramework::Vite));
        assert_eq!(info.app_dir, "");
    }

    #[test]
    fn lockfile_picks_manager_and_version() {
        let (_d, t) = tree(&[
            ("package.json", "{}"),
            ("yarn.lock", "__metadata:\n  version: 8\n"),
        ]);
        let info = detect(&t).unwrap();
        assert_eq!(info.package_manager, Some(PackageManager::Yarn));
        assert_eq!(info.package_manager_version.as_deref(), Some("4"));
        let (_d, t) = tree(&[
            ("package.json", "{}"),
            ("pnpm-lock.yaml", "lockfileVersion: '6.0'\n"),
        ]);
        assert_eq!(
            detect(&t).unwrap().package_manager_version.as_deref(),
            Some("8")
        );
    }

    #[test]
    fn monorepo_picks_web_app() {
        let (_d, t) = tree(&[
            (
                "package.json",
                r#"{"private":true,"workspaces":["apps/*","packages/*"],"scripts":{"lint":"x"}}"#,
            ),
            (
                "apps/docs/package.json",
                r#"{"name":"docs","scripts":{"start":"x"}}"#,
            ),
            (
                "apps/web/package.json",
                r#"{"name":"web","scripts":{"dev":"next dev"},"dependencies":{"next":"15"}}"#,
            ),
            ("packages/ui/package.json", r#"{"name":"ui"}"#),
        ]);
        let info = detect(&t).unwrap();
        assert_eq!(info.app_dir, "apps/web");
        assert!(info.has(NodeFramework::Next));
    }

    #[test]
    fn cli_bins() {
        let (_d, t) = tree(&[(
            "package.json",
            r#"{"name":"@scope/cowsay","bin":"./cli.js"}"#,
        )]);
        let info = detect(&t).unwrap();
        assert_eq!(
            info.bins,
            vec![("cowsay".to_string(), "./cli.js".to_string())]
        );
        assert_eq!(info.package_manager, None);
    }
}
