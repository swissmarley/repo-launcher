//! Looks at a project's files and reports what it is. Pure: no network, no side effects.

pub mod envvars;
pub mod go;
pub mod node;
pub mod python;
mod tree;

pub use tree::FileTree;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Ecosystem {
    Python,
    Node,
    Rust,
    Go,
    Ruby,
    Php,
    Java,
    Docker,
    Conda,
    DotNet,
}

impl Ecosystem {
    /// The name people know it by, for messages ("It looks like a PHP project").
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::Node => "Node.js",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
            Self::Java => "Java",
            Self::Docker => "Docker",
            Self::Conda => "Conda",
            Self::DotNet => ".NET",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Framework {
    Streamlit,
    Gradio,
    Flask,
    FastApi,
    Django,
    Jupyter,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionReport {
    pub ecosystems: Vec<Ecosystem>,
    pub has_launcher_json: bool,
    pub python: Option<python::PythonInfo>,
    pub node: Option<node::NodeInfo>,
    pub go: Option<go::GoInfo>,
    /// A Docker Compose file at the root, if any.
    pub compose_file: Option<String>,
    /// Settings (API keys etc.) the code reads from its environment.
    #[serde(default)]
    pub env_hints: Vec<envvars::EnvHint>,
}

pub fn detect(tree: &FileTree) -> DetectionReport {
    let has = |name: &str| tree.has(name);
    let mut ecosystems = Vec::new();
    let python = python::detect(tree);
    if python.is_some() {
        ecosystems.push(Ecosystem::Python);
    }
    let node = node::detect(tree);
    if node.is_some() {
        ecosystems.push(Ecosystem::Node);
    }
    if has("Cargo.toml") {
        ecosystems.push(Ecosystem::Rust);
    }
    if has("go.mod") {
        ecosystems.push(Ecosystem::Go);
    }
    if has("Gemfile") {
        ecosystems.push(Ecosystem::Ruby);
    }
    if has("composer.json") {
        ecosystems.push(Ecosystem::Php);
    }
    if has("pom.xml") || has("build.gradle") || has("build.gradle.kts") {
        ecosystems.push(Ecosystem::Java);
    }
    if has("Dockerfile") || has("docker-compose.yml") || has("compose.yaml") {
        ecosystems.push(Ecosystem::Docker);
    }
    if has("environment.yml") || has("environment.yaml") {
        ecosystems.push(Ecosystem::Conda);
    }
    // C#, F# and VB projects: a solution or project file at the root or one folder down.
    if tree.files().any(|f| {
        f.matches('/').count() <= 1
            && [".sln", ".csproj", ".fsproj", ".vbproj"]
                .iter()
                .any(|ext| f.ends_with(ext))
    }) {
        ecosystems.push(Ecosystem::DotNet);
    }
    DetectionReport {
        ecosystems,
        has_launcher_json: has("launcher.json"),
        python,
        node,
        go: go::detect(tree),
        compose_file: [
            "compose.yaml",
            "compose.yml",
            "docker-compose.yml",
            "docker-compose.yaml",
        ]
        .into_iter()
        .find(|f| has(f))
        .map(str::to_owned),
        env_hints: envvars::scan(tree),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::ProblemCode;

    #[test]
    fn dotnet_projects_are_named_in_the_explanation() {
        let dir = tempfile::tempdir().unwrap();
        for file in [
            "Dashboard.sln",
            "Dashboard/Dashboard.csproj",
            "Dashboard/Program.cs",
        ] {
            let p = dir.path().join(file);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "").unwrap();
        }
        let tree = FileTree::scan(dir.path()).unwrap();
        let report = detect(&tree);
        assert_eq!(report.ecosystems, [Ecosystem::DotNet]);
        let problem = crate::recipe::resolve(&tree, &report).unwrap_err();
        assert_eq!(problem.code, ProblemCode::UnsupportedProject);
        assert_eq!(problem.params["ecosystem"], ".NET");
    }
}
