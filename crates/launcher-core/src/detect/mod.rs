//! Looks at a project's files and reports what it is. Pure: no network, no side effects.

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
}

pub fn detect(tree: &FileTree) -> DetectionReport {
    let has = |name: &str| tree.has(name);
    let mut ecosystems = Vec::new();
    let python = python::detect(tree);
    if python.is_some() {
        ecosystems.push(Ecosystem::Python);
    }
    if has("package.json") {
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
    DetectionReport {
        ecosystems,
        has_launcher_json: has("launcher.json"),
        python,
    }
}
