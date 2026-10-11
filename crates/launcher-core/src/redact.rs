//! Removes secrets from text before it is logged, shown or copied as diagnostics.
//!
//! Two layers: the exact values of every secret the engine has handled this session (keys the
//! user entered, the GitHub token), and patterns for well-known token formats, which also catch
//! secrets the engine never saw (a project printing a key from its own config file).

use std::borrow::Cow;
use std::sync::{LazyLock, RwLock};

pub const MASK: &str = "[hidden]";

/// Shorter values would hide ordinary words; real keys are far longer.
const MIN_LEN: usize = 6;

static KNOWN: RwLock<Vec<String>> = RwLock::new(Vec::new());

static PATTERNS: LazyLock<Vec<regex::Regex>> = LazyLock::new(|| {
    [
        // GitHub tokens: classic, OAuth, user/server, refresh, fine-grained.
        r"\bgh[pousr]_[A-Za-z0-9]{20,}",
        r"\bgithub_pat_[A-Za-z0-9_]{20,}",
        // Anthropic, OpenAI and other "sk-" keys.
        r"\bsk-[A-Za-z0-9_\-]{16,}",
        // Hugging Face, Groq, Replicate, Slack, Google, AWS access key ids.
        r"\bhf_[A-Za-z0-9]{20,}",
        r"\bgsk_[A-Za-z0-9]{20,}",
        r"\br8_[A-Za-z0-9]{20,}",
        r"\bxox[abprs]-[A-Za-z0-9\-]{10,}",
        r"\bAIza[0-9A-Za-z_\-]{30,}",
        r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",
        // Authorization headers and credentials inside URLs or query strings.
        r"(?i)\b(?:bearer|token|basic)\s+[A-Za-z0-9._~+/=\-]{12,}",
        r"(?i)([?&](?:access_token|token|key|api_key|apikey|code|sig)=)[^&\s#]+",
        r"(://[^/\s:@]+:)[^@\s/]+@",
        r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?(?:-----END [A-Z ]*PRIVATE KEY-----|$)",
    ]
    .iter()
    .map(|p| regex::Regex::new(p).expect("valid pattern"))
    .collect()
});

/// Registers a secret value so it is hidden wherever it appears from now on.
pub fn remember(value: &str) {
    let value = value.trim();
    if value.len() < MIN_LEN {
        return;
    }
    let mut known = KNOWN.write().unwrap_or_else(|e| e.into_inner());
    if !known.iter().any(|k| k == value) {
        known.push(value.to_owned());
        // Longest first, so a secret containing another is hidden whole.
        known.sort_by_key(|k| std::cmp::Reverse(k.len()));
    }
}

/// Returns `text` with known secrets and token-like strings replaced by [`MASK`].
pub fn scrub(text: &str) -> Cow<'_, str> {
    let mut out = Cow::Borrowed(text);
    {
        let known = KNOWN.read().unwrap_or_else(|e| e.into_inner());
        for k in known.iter() {
            if out.contains(k.as_str()) {
                out = Cow::Owned(out.replace(k.as_str(), MASK));
            }
        }
    }
    for re in PATTERNS.iter() {
        if re.is_match(&out) {
            // Keep a captured prefix (`?token=`, `https://user:`) so the text stays readable.
            let replaced = re
                .replace_all(&out, |c: &regex::Captures<'_>| {
                    format!("{}{MASK}", c.get(1).map_or("", |m| m.as_str()))
                })
                .into_owned();
            out = Cow::Owned(replaced);
        }
    }
    out
}

pub fn scrub_owned(text: String) -> String {
    match scrub(&text) {
        Cow::Borrowed(_) => text,
        Cow::Owned(s) => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_remembered_values() {
        remember("my-very-private-value-123");
        let s = scrub("the key is my-very-private-value-123, ok");
        assert_eq!(s, "the key is [hidden], ok");
        remember("abc");
        assert_eq!(scrub("abc"), "abc", "too short to be a secret");
    }

    #[test]
    fn hides_token_patterns() {
        let cases = [
            ("token ghu_abcdefghijklmnopqrstuvwxyz0123", "ghu_"),
            (
                "ANTHROPIC_API_KEY=sk-ant-api03-abcdefghijklmnopqrstu",
                "sk-ant",
            ),
            ("OPENAI=sk-proj-abcdefghijklmnopqrstuvwx", "sk-proj"),
            ("hf_abcdefghijklmnopqrstuvwxyz", "hf_"),
            ("Authorization: Bearer abc.def.ghi-jkl_mno", "abc.def"),
            (
                "GET https://codeload.github.com/o/r/tar.gz/x?token=AAAABBBB -> 404",
                "AAAABBBB",
            ),
            (
                "https://x-access-token:ghs_secretsecret@github.com/o/r",
                "secretsecret",
            ),
            ("AKIAABCDEFGHIJKLMNOP", "AKIA"),
        ];
        for (input, leaked) in cases {
            let s = scrub(input);
            assert!(!s.contains(leaked), "{input} -> {s}");
            assert!(s.contains(MASK), "{input} -> {s}");
        }
        assert_eq!(
            scrub("GET https://x/y?token=AAAABBBB&page=2"),
            "GET https://x/y?token=[hidden]&page=2"
        );
        assert_eq!(scrub("nothing to see"), "nothing to see");
    }
}
