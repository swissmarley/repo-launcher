//! User-facing failures. The engine never produces English sentences for the UI: it returns a
//! stable [`ProblemCode`] plus parameters, and the UI owns the wording.

use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProblemCode {
    InvalidLink,
    NotGithub,
    RepoNotFoundOrPrivate,
    RateLimited,
    Offline,
    DownloadFailed,
    DownloadCorrupt,
    UnsupportedProject,
    NoEntryPoint,
    ManifestInvalid,
    MissingSystemTool,
    InstallStepFailed,
    NeedsCompiler,
    NotInstalled,
    AlreadyRunning,
    LaunchFailed,
    ExitedEarly,
    StartTimeout,
    Cancelled,
    ProjectNotFound,
    /// No usable keychain (e.g. a Linux desktop without a Secret Service).
    KeychainUnavailable,
    /// Longer than the 2560 bytes Windows allows for one saved secret.
    SecretTooLarge,
    /// This build has no GitHub App client ID configured.
    GithubNotConfigured,
    /// The sign-in code expired before it was entered.
    SignInExpired,
    /// The user declined on GitHub.
    SignInDenied,
    /// The saved GitHub sign-in no longer works (expired or revoked).
    SignedOut,
    /// Signed in, but GitHub still says not found: missing, or not shared with Repo Launcher.
    RepoNotShared,
    /// The project needs settings (API keys) the user hasn't entered.
    MissingKeys,
    /// The install or run plan changed (or was never approved); the user must review it.
    ConsentRequired,
    AiNotEnabled,
    AiKeyMissing,
    /// The AI service refused or failed (bad key, quota, network).
    AiFailed,
    /// The AI suggestion didn't pass the launcher.json validator.
    AiInvalidRecipe,
    /// A shortcut file couldn't be created or removed.
    ShortcutFailed,
    /// No terminal program was found to open a project in ("open in my terminal", Linux).
    NoSystemTerminal,
    /// Starting at login couldn't be turned on or off.
    LoginItemFailed,
    /// Checking for, downloading or installing a new version of Repo Launcher failed.
    UpdateFailed,
    Internal,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FixAction {
    Retry,
    Install,
    Repair,
    CheckConnection,
    WaitAndRetry {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        seconds: u64,
    },
    InstallMacCommandLineTools,
    OpenUrl {
        url: String,
    },
    ConnectGithub,
    /// Let Repo Launcher see more repositories (GitHub App installation page).
    ChooseRepositories {
        url: String,
    },
    EnterKeys,
    ReviewSafety,
    AskAi,
    OpenSettings,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub code: ProblemCode,
    pub params: BTreeMap<String, String>,
    pub fix: Option<FixAction>,
    /// Technical detail for "Copy diagnostics". Never shown as the main message.
    pub detail: Option<String>,
}

impl Problem {
    pub fn new(code: ProblemCode) -> Self {
        Self {
            code,
            params: BTreeMap::new(),
            fix: None,
            detail: None,
        }
    }

    pub fn param(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.insert(key.to_owned(), value.into());
        self
    }

    pub fn fix(mut self, fix: FixAction) -> Self {
        self.fix = Some(fix);
        self
    }

    /// Technical detail; secrets and token-like strings are removed here, at the source.
    pub fn detail(mut self, detail: impl fmt::Display) -> Self {
        self.detail = Some(crate::redact::scrub_owned(detail.to_string()));
        self
    }

    pub fn internal(detail: impl fmt::Display) -> Self {
        Self::new(ProblemCode::Internal)
            .fix(FixAction::Retry)
            .detail(detail)
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.code)?;
        for (k, v) in &self.params {
            write!(f, " {k}={v}")?;
        }
        if let Some(d) = &self.detail {
            write!(f, ": {d}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Problem {}

impl From<std::io::Error> for Problem {
    fn from(e: std::io::Error) -> Self {
        Problem::internal(e)
    }
}

impl From<rusqlite::Error> for Problem {
    fn from(e: rusqlite::Error) -> Self {
        Problem::internal(e)
    }
}

impl From<reqwest::Error> for Problem {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() || e.is_timeout() {
            Problem::new(ProblemCode::Offline)
                .fix(FixAction::CheckConnection)
                .detail(e)
        } else {
            Problem::new(ProblemCode::DownloadFailed)
                .fix(FixAction::Retry)
                .detail(e)
        }
    }
}

pub type Result<T, E = Problem> = std::result::Result<T, E>;
