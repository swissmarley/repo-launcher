//! Talks to GitHub. Anonymous use is the default, so only `metadata` touches the rate-limited
//! REST API (60 calls/hour/IP). Ref resolution and archive downloads use quota-free endpoints.
//!
//! A token is passed only for private repositories; public ones never send it (D-002).

use crate::problem::{FixAction, Problem, ProblemCode, Result};
use crate::source::url::RepoRef;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct GitHub {
    http: reqwest::Client,
    api_base: String,
    web_base: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoMeta {
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    #[serde(rename = "stargazers_count")]
    pub stars: u64,
    pub default_branch: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub archived: bool,
    pub license: Option<License>,
    pub created_at: Option<String>,
    pub pushed_at: Option<String>,
    pub owner: Owner,
}

#[derive(Debug, Clone, Deserialize)]
pub struct License {
    pub spdx_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Owner {
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRef {
    /// Branch or tag name, or the commit itself when the user linked a commit.
    pub name: String,
    pub commit: String,
    pub subdir: Option<String>,
}

impl GitHub {
    pub fn new(http: reqwest::Client) -> Self {
        Self::with_bases(http, "https://api.github.com", "https://github.com")
    }

    pub fn with_bases(http: reqwest::Client, api: &str, web: &str) -> Self {
        Self {
            http,
            api_base: api.trim_end_matches('/').into(),
            web_base: web.trim_end_matches('/').into(),
        }
    }

    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    pub fn web_base(&self) -> &str {
        &self.web_base
    }

    pub async fn metadata(&self, r: &RepoRef, token: Option<&str>) -> Result<RepoMeta> {
        let url = format!("{}/repos/{}/{}", self.api_base, r.owner, r.repo);
        let mut req = self
            .http
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }
        let resp = req.send().await?;
        match resp.status().as_u16() {
            200 => Ok(resp.json().await?),
            404 | 451 => Err(not_found(r)),
            403 | 429 if is_rate_limited(&resp) => Err(rate_limited(&resp)),
            s => Err(Problem::new(ProblemCode::DownloadFailed)
                .fix(FixAction::Retry)
                .detail(format!("GET {url} -> {s}"))),
        }
    }

    /// Lists refs through Git's public smart-HTTP endpoint (no API quota, no git binary).
    /// Account creation date, for "this account is brand new" warnings. One anonymous API call.
    pub async fn owner_created_at(&self, owner: &str) -> Result<Option<String>> {
        #[derive(Deserialize)]
        struct User {
            created_at: Option<String>,
        }
        let url = format!("{}/users/{owner}", self.api_base);
        let resp = self
            .http
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        Ok(resp.json::<User>().await?.created_at)
    }

    /// With a token, Git's endpoint takes it as the password for user `x-access-token`.
    pub async fn resolve_ref(&self, r: &RepoRef, token: Option<&str>) -> Result<ResolvedRef> {
        let url = format!(
            "{}/{}/{}.git/info/refs?service=git-upload-pack",
            self.web_base, r.owner, r.repo
        );
        let mut req = self.http.get(&url);
        if let Some(t) = token {
            req = req.basic_auth("x-access-token", Some(t));
        }
        let resp = req.send().await?;
        match resp.status().as_u16() {
            200 => {}
            401 | 403 | 404 => return Err(not_found(r)),
            s => {
                return Err(Problem::new(ProblemCode::DownloadFailed)
                    .fix(FixAction::Retry)
                    .detail(format!("GET {url} -> {s}")));
            }
        }
        let body = resp.text().await?;
        let refs = parse_ref_advertisement(&body);
        pick_ref(&refs, r.tree.as_deref())
            .ok_or_else(|| not_found(r).detail(format!("ref {:?} not found", r.tree)))
    }

    /// Private repositories download through the API, which redirects to a short-lived link.
    pub fn api_tarball_url(&self, r: &RepoRef, commit: &str) -> String {
        format!(
            "{}/repos/{}/{}/tarball/{}",
            self.api_base, r.owner, r.repo, commit
        )
    }

    pub fn archive_url(&self, r: &RepoRef, commit: &str) -> String {
        format!(
            "{}/{}/{}/archive/{}.tar.gz",
            self.web_base, r.owner, r.repo, commit
        )
    }
}

fn not_found(r: &RepoRef) -> Problem {
    Problem::new(ProblemCode::RepoNotFoundOrPrivate).param("repo", r.slug())
}

fn is_rate_limited(resp: &reqwest::Response) -> bool {
    resp.headers()
        .get("x-ratelimit-remaining")
        .is_some_and(|v| v == "0")
        || resp.headers().contains_key("retry-after")
}

fn rate_limited(resp: &reqwest::Response) -> Problem {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let header = |name: &str| {
        resp.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
    };
    let wait = header("retry-after")
        .or_else(|| header("x-ratelimit-reset").map(|reset| reset.saturating_sub(now)))
        .unwrap_or(60)
        .max(1);
    Problem::new(ProblemCode::RateLimited)
        .param("minutes", wait.div_ceil(60).to_string())
        .fix(FixAction::WaitAndRetry { seconds: wait })
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RefAdvertisement {
    pub head_target: Option<String>,
    pub head_commit: Option<String>,
    /// (full ref name, commit). Annotated tags are resolved to the peeled commit.
    pub refs: Vec<(String, String)>,
}

/// Parses the pkt-line body of `info/refs?service=git-upload-pack`.
pub fn parse_ref_advertisement(body: &str) -> RefAdvertisement {
    let mut out = RefAdvertisement::default();
    let mut rest = body;
    while rest.len() >= 4 {
        let Ok(len) = usize::from_str_radix(&rest[..4], 16) else {
            break;
        };
        if len == 0 {
            rest = &rest[4..];
            continue;
        }
        if len < 4 || len > rest.len() {
            break;
        }
        let line = rest[4..len].trim_end_matches('\n');
        rest = &rest[len..];
        if line.starts_with('#') {
            continue;
        }
        let (refpart, caps) = line.split_once('\0').unwrap_or((line, ""));
        let Some((sha, name)) = refpart.split_once(' ') else {
            continue;
        };
        if let Some(target) = caps.split(' ').find_map(|c| c.strip_prefix("symref=HEAD:")) {
            out.head_target = Some(target.to_owned());
        }
        if name == "HEAD" {
            out.head_commit = Some(sha.to_owned());
        } else if let Some(tag) = name.strip_suffix("^{}") {
            // Peeled annotated tag: point the tag at the commit, not the tag object.
            if let Some(entry) = out.refs.iter_mut().find(|(n, _)| n == tag) {
                entry.1 = sha.to_owned();
            }
        } else {
            out.refs.push((name.to_owned(), sha.to_owned()));
        }
    }
    out
}

/// Splits a `/tree/<path>` tail into the longest matching branch/tag plus a subdirectory.
pub fn pick_ref(adv: &RefAdvertisement, tree: Option<&str>) -> Option<ResolvedRef> {
    let Some(tree) = tree else {
        let commit = adv.head_commit.clone()?;
        let name = adv
            .head_target
            .as_deref()
            .map(|t| t.trim_start_matches("refs/heads/").to_owned())
            .unwrap_or_else(|| "HEAD".into());
        return Some(ResolvedRef {
            name,
            commit,
            subdir: None,
        });
    };

    let segs: Vec<&str> = tree.split('/').collect();
    for n in (1..=segs.len()).rev() {
        let candidate = segs[..n].join("/");
        let found = ["refs/heads/", "refs/tags/"].iter().find_map(|prefix| {
            let full = format!("{prefix}{candidate}");
            adv.refs
                .iter()
                .find(|(name, _)| *name == full)
                .map(|(_, sha)| sha.clone())
        });
        let commit = found.or_else(|| is_full_sha(&candidate).then(|| candidate.clone()));
        if let Some(commit) = commit {
            let subdir = (n < segs.len()).then(|| segs[n..].join("/"));
            return Some(ResolvedRef {
                name: candidate,
                commit,
                subdir,
            });
        }
    }
    None
}

fn is_full_sha(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkt(line: &str) -> String {
        format!("{:04x}{}", line.len() + 4, line)
    }

    fn sample() -> String {
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        let c = "c".repeat(40);
        let t = "d".repeat(40);
        let mut s = pkt("# service=git-upload-pack\n");
        s.push_str("0000");
        s.push_str(&pkt(&format!(
            "{a} HEAD\0multi_ack symref=HEAD:refs/heads/main agent=git/x\n"
        )));
        s.push_str(&pkt(&format!("{a} refs/heads/main\n")));
        s.push_str(&pkt(&format!("{b} refs/heads/feature/x\n")));
        s.push_str(&pkt(&format!("{t} refs/tags/v1.0\n")));
        s.push_str(&pkt(&format!("{c} refs/tags/v1.0^{{}}\n")));
        s.push_str("0000");
        s
    }

    #[test]
    fn parses_advertisement() {
        let adv = parse_ref_advertisement(&sample());
        assert_eq!(adv.head_target.as_deref(), Some("refs/heads/main"));
        assert_eq!(adv.head_commit, Some("a".repeat(40)));
        assert_eq!(adv.refs.len(), 3);
        assert!(
            adv.refs
                .contains(&("refs/tags/v1.0".into(), "c".repeat(40)))
        );
    }

    #[test]
    fn picks_default_branch() {
        let adv = parse_ref_advertisement(&sample());
        let r = pick_ref(&adv, None).unwrap();
        assert_eq!(
            (r.name.as_str(), r.commit, r.subdir),
            ("main", "a".repeat(40), None)
        );
    }

    #[test]
    fn picks_longest_branch_with_slashes_and_subdir() {
        let adv = parse_ref_advertisement(&sample());
        let r = pick_ref(&adv, Some("feature/x/apps/web")).unwrap();
        assert_eq!(r.name, "feature/x");
        assert_eq!(r.commit, "b".repeat(40));
        assert_eq!(r.subdir.as_deref(), Some("apps/web"));
    }

    #[test]
    fn picks_tags_and_commits() {
        let adv = parse_ref_advertisement(&sample());
        assert_eq!(pick_ref(&adv, Some("v1.0")).unwrap().commit, "c".repeat(40));
        let sha = "e".repeat(40);
        assert_eq!(pick_ref(&adv, Some(&sha)).unwrap().commit, sha);
        assert!(pick_ref(&adv, Some("nope")).is_none());
    }
}
