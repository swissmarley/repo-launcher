use crate::recipe::StepLabel;
use serde::Serialize;
use std::sync::Arc;

/// The user-facing install stages. Engine steps map onto these.
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Downloading,
    Preparing,
    InstallingHelpers,
    Ready,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LogStream {
    Stdout,
    Stderr,
    Engine,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RunState {
    Starting,
    Running,
    Stopped,
    Crashed,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EngineEvent {
    #[serde(rename_all = "camelCase")]
    Progress {
        project_id: String,
        stage: Stage,
        fraction: f32,
        label: Option<StepLabel>,
    },
    #[serde(rename_all = "camelCase")]
    Log {
        project_id: String,
        stream: LogStream,
        line: String,
    },
    #[serde(rename_all = "camelCase")]
    RunState {
        project_id: String,
        state: RunState,
        url: Option<String>,
        exit_code: Option<i32>,
    },
}

pub type EventSink = Arc<dyn Fn(EngineEvent) + Send + Sync>;

pub fn null_sink() -> EventSink {
    Arc::new(|_| {})
}
