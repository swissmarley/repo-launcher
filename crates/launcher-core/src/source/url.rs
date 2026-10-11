//! Parses whatever the user pasted into a GitHub repo reference.

use crate::problem::{Problem, ProblemCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRef {
    pub owner: String,
    pub repo: String,
    /// Everything after `/tree/` or `/blob/`, e.g. `feature/x/docs`. Branch names may contain
    /// slashes, so splitting into ref + subdirectory needs the repo's real ref list.
    pub tree: Option<String>,
}

impl RepoRef {
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

pub fn parse(input: &str) -> Result<RepoRef, Problem> {
    let s = input
        .trim()
        .trim_matches(|c| c == '<' || c == '>' || c == '"' || c == '\'');
    if s.is_empty() {
        return Err(invalid(input));
    }

    let path = if let Some(rest) = strip_ssh(s) {
        rest
    } else if let Some(rest) = strip_web(s)? {
        rest
    } else if looks_like_shorthand(s) {
        s
    } else {
        return Err(invalid(input));
    };

    let path = path.split(['?', '#']).next().unwrap_or_default();
    let mut segs = path.split('/').filter(|p| !p.is_empty());
    let owner = segs.next().ok_or_else(|| invalid(input))?;
    let repo = segs.next().ok_or_else(|| invalid(input))?;
    let repo = repo.strip_suffix(".git").unwrap_or(repo);

    if !valid_owner(owner) || !valid_repo(repo) {
        return Err(invalid(input));
    }

    let rest: Vec<&str> = segs.collect();
    let tree = match rest.as_slice() {
        ["tree" | "blob" | "commit", tail @ ..] if !tail.is_empty() => Some(tail.join("/")),
        _ => None,
    };

    Ok(RepoRef {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
        tree,
    })
}

fn strip_ssh(s: &str) -> Option<&str> {
    s.strip_prefix("git@github.com:")
        .or_else(|| s.strip_prefix("ssh://git@github.com/"))
        .or_else(|| s.strip_prefix("git://github.com/"))
}

/// Returns `Ok(None)` when the input isn't a URL at all.
fn strip_web(s: &str) -> Result<Option<&str>, Problem> {
    let without_scheme = ["https://", "http://"]
        .iter()
        .find(|p| {
            s.get(..p.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(p))
        })
        .map_or(s, |p| &s[p.len()..]);
    let (host, rest) = match without_scheme.split_once('/') {
        Some((h, r)) => (h, r),
        None => (without_scheme, ""),
    };
    let host = host.to_ascii_lowercase();
    let is_url = s.contains("://") || host.contains('.');
    if !is_url {
        return Ok(None);
    }
    match host.as_str() {
        "github.com" | "www.github.com" => Ok(Some(rest)),
        _ => Err(Problem::new(ProblemCode::NotGithub).param("input", s)),
    }
}

fn looks_like_shorthand(s: &str) -> bool {
    let parts: Vec<&str> = s.trim_end_matches('/').split('/').collect();
    parts.len() == 2 && !s.contains(' ')
}

fn valid_owner(s: &str) -> bool {
    (1..=39).contains(&s.len())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !s.starts_with('-')
}

fn valid_repo(s: &str) -> bool {
    (1..=100).contains(&s.len())
        && s != "."
        && s != ".."
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn invalid(input: &str) -> Problem {
    Problem::new(ProblemCode::InvalidLink).param("input", input.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(input: &str, owner: &str, repo: &str, tree: Option<&str>) {
        let r = parse(input).unwrap_or_else(|e| panic!("{input:?} failed: {e}"));
        assert_eq!(
            (r.owner.as_str(), r.repo.as_str(), r.tree.as_deref()),
            (owner, repo, tree),
            "{input:?}"
        );
    }

    fn err(input: &str, code: ProblemCode) {
        match parse(input) {
            Ok(r) => panic!("{input:?} should fail, got {r:?}"),
            Err(p) => assert_eq!(p.code, code, "{input:?}"),
        }
    }

    #[test]
    fn accepts_common_forms() {
        ok(
            "https://github.com/streamlit/streamlit",
            "streamlit",
            "streamlit",
            None,
        );
        ok("http://github.com/a/b", "a", "b", None);
        ok("github.com/a/b", "a", "b", None);
        ok("www.github.com/a/b/", "a", "b", None);
        ok("HTTPS://GitHub.com/Owner/Repo", "Owner", "Repo", None);
        ok("https://github.com/a/b.git", "a", "b", None);
        ok("  https://github.com/a/b  \n", "a", "b", None);
        ok("<https://github.com/a/b>", "a", "b", None);
        ok("a/b", "a", "b", None);
        ok("a-b/c.d_e", "a-b", "c.d_e", None);
        ok("git@github.com:a/b.git", "a", "b", None);
        ok("ssh://git@github.com/a/b.git", "a", "b", None);
    }

    #[test]
    fn keeps_tree_paths() {
        ok("https://github.com/a/b/tree/main", "a", "b", Some("main"));
        ok(
            "https://github.com/a/b/tree/feature/x/docs",
            "a",
            "b",
            Some("feature/x/docs"),
        );
        ok(
            "https://github.com/a/b/blob/dev/README.md",
            "a",
            "b",
            Some("dev/README.md"),
        );
        ok(
            "https://github.com/a/b/tree/main?tab=readme#usage",
            "a",
            "b",
            Some("main"),
        );
    }

    #[test]
    fn ignores_other_subpages() {
        ok("https://github.com/a/b/issues/12", "a", "b", None);
        ok("https://github.com/a/b?tab=readme-ov-file", "a", "b", None);
        ok("https://github.com/a/b#readme", "a", "b", None);
        ok("https://github.com/a/b/tree/", "a", "b", None);
    }

    #[test]
    fn rejects_bad_input() {
        err("", ProblemCode::InvalidLink);
        err("   ", ProblemCode::InvalidLink);
        err("hello", ProblemCode::InvalidLink);
        err("just some words/here", ProblemCode::InvalidLink);
        err("https://github.com/onlyowner", ProblemCode::InvalidLink);
        err("https://github.com/", ProblemCode::InvalidLink);
        err("a/b/c", ProblemCode::InvalidLink);
        err("-bad/repo", ProblemCode::InvalidLink);
        err("owner/..", ProblemCode::InvalidLink);
        err("ünïcode/répo", ProblemCode::InvalidLink);
        err("httpé://x/a/b", ProblemCode::NotGithub);
        err("own er/repo", ProblemCode::InvalidLink);
    }

    #[test]
    fn rejects_other_hosts() {
        err("https://gitlab.com/a/b", ProblemCode::NotGithub);
        err("bitbucket.org/a/b", ProblemCode::NotGithub);
        err(
            "https://github.com.evil.example/a/b",
            ProblemCode::NotGithub,
        );
    }
}
