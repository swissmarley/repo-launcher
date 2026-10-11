//! Repo Launcher engine: turns a GitHub repo into an installed, launchable project.
//!
//! The engine has no UI dependency. The desktop app and the `rl` CLI are thin layers over
//! [`Engine`].

pub mod ai;
pub mod console;
pub mod detect;
pub mod diagnose;
pub mod download;
pub mod engine;
pub mod env;
pub mod events;
pub mod install;
pub mod paths;
pub mod problem;
pub mod proc;
pub mod procinfo;
pub mod recipe;
pub mod redact;
pub mod run;
pub mod runtime;
pub mod safety;
pub mod secrets;
pub mod shortcuts;
pub mod source;
pub mod store;

pub use engine::Engine;
pub use events::{EngineEvent, EventSink, Stage};
pub use paths::AppPaths;
pub use problem::{FixAction, Problem, ProblemCode};
pub use store::{Project, ProjectStatus};
