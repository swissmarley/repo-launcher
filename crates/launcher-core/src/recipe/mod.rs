//! A recipe is the normalized "how to install and run this project". Heuristics produce one
//! today; `launcher.json` and AI proposals will produce the same structure.

mod python;
pub mod pyversion;

use crate::detect::{DetectionReport, Ecosystem};
use crate::problem::{Problem, ProblemCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppKind {
    Web,
    Terminal,
    Gui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecipeSource {
    Manifest,
    Heuristic,
}

/// Plain-language identity of a step; the UI turns it into words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StepLabel {
    SetUpPython { version: String },
    CreateEnvironment,
    InstallHelpers { from: String },
    Custom { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub label: StepLabel,
    /// argv. The first element is resolved to a managed tool (`uv`, `python`, …).
    pub argv: Vec<String>,
    /// Relative to the project source folder.
    pub cwd: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSpec {
    pub argv: Vec<String>,
    pub cwd: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSpec {
    pub preferred_port: u16,
    pub path: String,
    pub ready_timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub kind: AppKind,
    pub source: RecipeSource,
    pub python: Option<String>,
    pub install: Vec<Step>,
    pub run: RunSpec,
    pub web: Option<WebSpec>,
}

impl Recipe {
    /// Stable hash used to decide whether consent must be asked again.
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let json = serde_json::to_vec(self).unwrap_or_default();
        hex::encode(Sha256::digest(json))
    }
}

pub fn from_detection(report: &DetectionReport) -> Result<Recipe, Problem> {
    if let Some(py) = &report.python {
        return python::recipe(py);
    }
    let what = report
        .ecosystems
        .first()
        .map(|e| format!("{e:?}").to_lowercase())
        .unwrap_or_else(|| "unknown".into());
    let code = if report.ecosystems.contains(&Ecosystem::Node) {
        // Node support lands in Phase 2.
        ProblemCode::UnsupportedProject
    } else if report.ecosystems.is_empty() {
        ProblemCode::NoEntryPoint
    } else {
        ProblemCode::UnsupportedProject
    };
    Err(Problem::new(code).param("ecosystem", what))
}
