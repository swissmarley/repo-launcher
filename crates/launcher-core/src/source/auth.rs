//! "Connect GitHub" through a GitHub App's device flow. No client secret ships with the app:
//! the device flow and refreshing device-flow tokens need only the public client ID.
//! Tokens are user-to-server tokens limited to what the user shares with the app
//! (read-only Contents and Metadata), valid 8 hours and refreshed automatically.

use crate::problem::{FixAction, Problem, ProblemCode, Result};
use crate::secrets::Secret;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub const CLIENT_ID_ENV: &str = "REPO_LAUNCHER_GITHUB_CLIENT_ID";
pub const APP_SLUG_ENV: &str = "REPO_LAUNCHER_GITHUB_APP_SLUG";
pub const CLIENT_ID_SETTING: &str = "github.client_id";
pub const APP_SLUG_SETTING: &str = "github.app_slug";

/// Where GitHub lists the apps a user has authorized (revocation needs a client secret, so
/// signing out locally points the user there).
pub const AUTHORIZATIONS_URL: &str = "https://github.com/settings/apps/authorizations";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub client_id: String,
    pub app_slug: Option<String>,
}

impl AppConfig {
    /// Environment variable, then the settings table, then a value compiled into the build.
    pub fn resolve(setting: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let pick = |env: &str, key: &str, built: Option<&'static str>| {
            std::env::var(env)
                .ok()
                .or_else(|| setting(key))
                .or_else(|| built.map(str::to_owned))
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty())
        };
        let client_id = pick(
            CLIENT_ID_ENV,
            CLIENT_ID_SETTING,
            option_env!("REPO_LAUNCHER_GITHUB_CLIENT_ID"),
        )?;
        let app_slug = pick(
            APP_SLUG_ENV,
            APP_SLUG_SETTING,
            option_env!("REPO_LAUNCHER_GITHUB_APP_SLUG"),
        );
        Some(Self {
            client_id,
            app_slug,
        })
    }

    /// The page where the user picks which repositories Repo Launcher may read.
    pub fn install_url(&self) -> String {
        match &self.app_slug {
            Some(slug) => format!("https://github.com/apps/{slug}/installations/new"),
            None => "https://github.com/settings/installations".into(),
        }
    }
}

pub fn not_configured() -> Problem {
    Problem::new(ProblemCode::GithubNotConfigured)
}

/// What the UI shows while the user approves the sign-in on github.com.
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub expires_in: u64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubAccount {
    pub login: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: Option<u64>,
}

/// A device code the engine is waiting on. The device code itself never leaves the engine.
#[derive(Clone)]
pub struct Pending {
    pub device_code: Secret,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
    refresh_token_expires_in: Option<i64>,
    error: Option<String>,
    interval: Option<u64>,
}

/// What is saved in the keychain under `github.token`.
#[derive(Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix seconds; `None` for tokens that don't expire.
    pub expires_at: Option<i64>,
    pub refresh_expires_at: Option<i64>,
}

impl std::fmt::Debug for StoredToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredToken")
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

impl StoredToken {
    fn from_response(r: TokenResponse, now: i64) -> Option<Self> {
        Some(Self {
            access_token: r.access_token?,
            refresh_token: r.refresh_token,
            expires_at: r.expires_in.map(|s| now + s),
            refresh_expires_at: r.refresh_token_expires_in.map(|s| now + s),
        })
    }

    pub fn to_secret(&self) -> Secret {
        Secret::new(serde_json::to_string(self).unwrap_or_default())
    }

    pub fn from_secret(s: &Secret) -> Option<Self> {
        let token: Self = serde_json::from_str(s.expose()).ok()?;
        crate::redact::remember(&token.access_token);
        if let Some(r) = &token.refresh_token {
            crate::redact::remember(r);
        }
        Some(token)
    }

    /// Needs a refresh within the next five minutes.
    pub fn needs_refresh(&self, now: i64) -> bool {
        self.expires_at.is_some_and(|t| t - 300 <= now)
    }
}

/// A repository the signed-in user has shared with Repo Launcher.
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRepo {
    pub full_name: String,
    pub description: Option<String>,
    pub private: bool,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub stars: u64,
    pub html_url: String,
    pub pushed_at: Option<String>,
}

#[derive(Deserialize)]
struct ApiRepo {
    full_name: String,
    description: Option<String>,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    stargazers_count: u64,
    html_url: String,
    pushed_at: Option<String>,
}

#[derive(Clone)]
pub struct Auth {
    http: reqwest::Client,
    web_base: String,
    api_base: String,
}

impl Auth {
    pub fn new(http: reqwest::Client, web_base: &str, api_base: &str) -> Self {
        Self {
            http,
            web_base: web_base.trim_end_matches('/').into(),
            api_base: api_base.trim_end_matches('/').into(),
        }
    }

    pub async fn request_device_code(&self, cfg: &AppConfig) -> Result<(DeviceCode, Pending)> {
        let resp = self
            .http
            .post(format!("{}/login/device/code", self.web_base))
            .header("Accept", "application/json")
            .form(&[("client_id", cfg.client_id.as_str())])
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if let Ok(err) = serde_json::from_str::<TokenResponse>(&text)
            && let Some(code) = err.error
        {
            return Err(oauth_error(&code));
        }
        let r: DeviceCodeResponse = serde_json::from_str(&text).map_err(|e| {
            Problem::new(ProblemCode::DownloadFailed)
                .fix(FixAction::Retry)
                .detail(format!("device code ({status}): {e}"))
        })?;
        crate::redact::remember(&r.device_code);
        Ok((
            DeviceCode {
                user_code: r.user_code,
                verification_uri: r.verification_uri,
                expires_in: r.expires_in,
            },
            Pending {
                device_code: Secret::new(r.device_code),
                interval: r.interval.unwrap_or(5).max(1),
                expires_in: r.expires_in,
            },
        ))
    }

    /// Polls until the user approves, declines, the code expires, or `cancel` fires.
    pub async fn poll_for_token(
        &self,
        cfg: &AppConfig,
        pending: &Pending,
        cancel: &CancellationToken,
        now: impl Fn() -> i64,
    ) -> Result<StoredToken> {
        let started = std::time::Instant::now();
        let mut interval = pending.interval;
        loop {
            tokio::select! {
                _ = cancel.cancelled() => return Err(Problem::new(ProblemCode::Cancelled)),
                _ = tokio::time::sleep(Duration::from_secs(interval)) => {}
            }
            if started.elapsed().as_secs() > pending.expires_in {
                return Err(Problem::new(ProblemCode::SignInExpired).fix(FixAction::ConnectGithub));
            }
            let r = self
                .token_request(&[
                    ("client_id", cfg.client_id.as_str()),
                    ("device_code", pending.device_code.expose()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])
                .await?;
            match r.error.as_deref() {
                None => {
                    return StoredToken::from_response(r, now())
                        .ok_or_else(|| Problem::internal("GitHub sent no token"));
                }
                Some("authorization_pending") => {}
                Some("slow_down") => interval = r.interval.unwrap_or(interval + 5),
                Some(code) => return Err(oauth_error(code)),
            }
        }
    }

    pub async fn refresh(
        &self,
        cfg: &AppConfig,
        refresh_token: &str,
        now: i64,
    ) -> Result<StoredToken> {
        let r = self
            .token_request(&[
                ("client_id", cfg.client_id.as_str()),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ])
            .await?;
        if let Some(code) = &r.error {
            return Err(match code.as_str() {
                "bad_refresh_token" | "unauthorized" => signed_out(),
                other => oauth_error(other),
            });
        }
        StoredToken::from_response(r, now).ok_or_else(signed_out)
    }

    async fn token_request(&self, form: &[(&str, &str)]) -> Result<TokenResponse> {
        let resp = self
            .http
            .post(format!("{}/login/oauth/access_token", self.web_base))
            .header("Accept", "application/json")
            .form(form)
            .send()
            .await?;
        let status = resp.status();
        let r: TokenResponse = resp.json().await.map_err(|e| {
            Problem::new(ProblemCode::DownloadFailed)
                .fix(FixAction::Retry)
                .detail(format!("token endpoint ({status}): {e}"))
        })?;
        if let Some(t) = &r.access_token {
            crate::redact::remember(t);
        }
        if let Some(t) = &r.refresh_token {
            crate::redact::remember(t);
        }
        Ok(r)
    }

    pub async fn user(&self, token: &str) -> Result<GitHubAccount> {
        #[derive(Deserialize)]
        struct User {
            login: String,
            avatar_url: Option<String>,
        }
        let resp = self
            .api_get(&format!("{}/user", self.api_base), token)
            .await?;
        let u: User = resp.json().await?;
        Ok(GitHubAccount {
            login: u.login,
            avatar_url: u.avatar_url,
        })
    }

    /// Repositories the user shared with Repo Launcher, across all their installations.
    pub async fn repositories(&self, token: &str) -> Result<Vec<RemoteRepo>> {
        #[derive(Deserialize)]
        struct Installations {
            installations: Vec<Installation>,
        }
        #[derive(Deserialize)]
        struct Installation {
            id: u64,
        }
        #[derive(Deserialize)]
        struct Repos {
            repositories: Vec<ApiRepo>,
        }
        let url = format!("{}/user/installations?per_page=100", self.api_base);
        let inst: Installations = self.api_get(&url, token).await?.json().await?;
        let mut out = Vec::new();
        for i in inst.installations {
            // Ten pages of 100 is more than anyone shares with a launcher.
            for page in 1..=10 {
                let url = format!(
                    "{}/user/installations/{}/repositories?per_page=100&page={page}",
                    self.api_base, i.id
                );
                let repos: Repos = self.api_get(&url, token).await?.json().await?;
                let n = repos.repositories.len();
                out.extend(repos.repositories.into_iter().map(|r| RemoteRepo {
                    full_name: r.full_name,
                    description: r.description,
                    private: r.private,
                    stars: r.stargazers_count,
                    html_url: r.html_url,
                    pushed_at: r.pushed_at,
                }));
                if n < 100 {
                    break;
                }
            }
        }
        out.sort_by(|a, b| b.pushed_at.cmp(&a.pushed_at));
        out.dedup_by(|a, b| a.full_name == b.full_name);
        Ok(out)
    }

    async fn api_get(&self, url: &str, token: &str) -> Result<reqwest::Response> {
        let resp = self
            .http
            .get(url)
            .bearer_auth(token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?;
        match resp.status().as_u16() {
            200 => Ok(resp),
            401 => Err(signed_out()),
            s => Err(Problem::new(ProblemCode::DownloadFailed)
                .fix(FixAction::Retry)
                .detail(format!("GET {url} -> {s}"))),
        }
    }
}

pub fn signed_out() -> Problem {
    Problem::new(ProblemCode::SignedOut).fix(FixAction::ConnectGithub)
}

fn oauth_error(code: &str) -> Problem {
    match code {
        "expired_token" | "token_expired" | "bad_verification_code" | "incorrect_device_code" => {
            Problem::new(ProblemCode::SignInExpired).fix(FixAction::ConnectGithub)
        }
        "access_denied" => Problem::new(ProblemCode::SignInDenied).fix(FixAction::ConnectGithub),
        "incorrect_client_credentials" | "device_flow_disabled" | "unsupported_grant_type" => {
            not_configured().detail(code)
        }
        other => Problem::internal(format!("GitHub sign-in: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn cfg() -> AppConfig {
        AppConfig {
            client_id: "Iv23test".into(),
            app_slug: Some("repo-launcher".into()),
        }
    }

    #[test]
    fn config_comes_from_settings_and_builds_install_url() {
        let c = AppConfig::resolve(|k| (k == CLIENT_ID_SETTING).then(|| " Iv23x ".into()));
        // An env var or compiled-in value may also be set on a developer machine.
        if std::env::var(CLIENT_ID_ENV).is_err()
            && option_env!("REPO_LAUNCHER_GITHUB_CLIENT_ID").is_none()
        {
            assert_eq!(c.as_ref().unwrap().client_id, "Iv23x");
        }
        assert_eq!(
            cfg().install_url(),
            "https://github.com/apps/repo-launcher/installations/new"
        );
    }

    #[tokio::test]
    async fn device_flow_waits_slows_down_and_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/device/code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "dc_0123456789", "user_code": "WDJB-MJHT",
                "verification_uri": "https://github.com/login/device",
                "expires_in": 900, "interval": 1
            })))
            .mount(&server)
            .await;
        // First poll: pending; second: slow down; third: token.
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"error": "authorization_pending"})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"error": "slow_down", "interval": 1})),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .and(body_string_contains("device_code=dc_0123456789"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "ghu_accessaccessaccess0123456789",
                "expires_in": 28800,
                "refresh_token": "ghr_refreshrefreshrefresh0123456789",
                "refresh_token_expires_in": 15897600,
                "token_type": "bearer", "scope": ""
            })))
            .mount(&server)
            .await;

        let auth = Auth::new(reqwest::Client::new(), &server.uri(), &server.uri());
        let (code, pending) = auth.request_device_code(&cfg()).await.unwrap();
        assert_eq!(code.user_code, "WDJB-MJHT");
        let token = auth
            .poll_for_token(&cfg(), &pending, &CancellationToken::new(), || 1000)
            .await
            .unwrap();
        assert_eq!(token.expires_at, Some(1000 + 28800));
        assert!(!token.needs_refresh(1000));
        assert!(token.needs_refresh(1000 + 28800 - 60));
        // No client secret is ever sent.
        for req in server.received_requests().await.unwrap() {
            assert!(!String::from_utf8_lossy(&req.body).contains("client_secret"));
        }
    }

    #[tokio::test]
    async fn declined_and_expired_sign_ins_are_explained() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"error": "access_denied"})),
            )
            .mount(&server)
            .await;
        let auth = Auth::new(reqwest::Client::new(), &server.uri(), &server.uri());
        let pending = Pending {
            device_code: Secret::new("dc"),
            interval: 1,
            expires_in: 900,
        };
        let err = auth
            .poll_for_token(&cfg(), &pending, &CancellationToken::new(), || 0)
            .await
            .unwrap_err();
        assert_eq!(err.code, ProblemCode::SignInDenied);

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/login/oauth/access_token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"error": "bad_refresh_token"})),
            )
            .mount(&server)
            .await;
        let auth = Auth::new(reqwest::Client::new(), &server.uri(), &server.uri());
        let err = auth.refresh(&cfg(), "ghr_x", 0).await.unwrap_err();
        assert_eq!(err.code, ProblemCode::SignedOut);
    }
}
