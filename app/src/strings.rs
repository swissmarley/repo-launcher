//! The few words the app shell shows itself: the tray menu, the macOS app menu and the terminal
//! in `--exec` mode. They come from the UI's own English file, like everything else the user
//! reads. `REPO_LAUNCHER_PSEUDO=1` shows them in the pseudo-locale (ui/src/lib/pseudo.ts), which
//! makes any text that skips this file stand out.

use serde_json::Value;
use std::sync::OnceLock;

static EN: &str = include_str!("../../ui/src/locales/en.json");

fn table() -> &'static Value {
    static TABLE: OnceLock<Value> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(EN).unwrap_or_default())
}

fn pseudo_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("REPO_LAUNCHER_PSEUDO").is_some_and(|v| v == "1"))
}

fn lookup(key: &str) -> Option<&'static str> {
    key.split('.')
        .try_fold(table(), |v, part| v.get(part))
        .and_then(Value::as_str)
}

/// The text at a dotted key such as `tray.quit`, with `{{name}}` placeholders filled in.
pub fn t(key: &str, params: &[(&str, &str)]) -> String {
    let text = lookup(key).unwrap_or(key);
    let text = if pseudo_enabled() {
        pseudo(text)
    } else {
        text.to_owned()
    };
    params
        .iter()
        .fold(text, |s, (k, v)| s.replace(&format!("{{{{{k}}}}}"), v))
}

/// Accented, bracketed and about 30% longer, with `{{placeholders}}` kept intact. Must match
/// `pseudo()` in ui/src/lib/pseudo.ts.
pub fn pseudo(text: &str) -> String {
    const FROM: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const TO: [char; 52] = [
        'á', 'ƀ', 'ç', 'ď', 'é', 'ƒ', 'ĝ', 'ĥ', 'í', 'ĵ', 'ķ', 'ĺ', 'ɱ', 'ñ', 'ó', 'þ', 'ǫ', 'ŕ',
        'š', 'ţ', 'ú', 'ṽ', 'ŵ', 'ẋ', 'ý', 'ž', 'Á', 'Ɓ', 'Ç', 'Ď', 'É', 'Ƒ', 'Ĝ', 'Ĥ', 'Í', 'Ĵ',
        'Ķ', 'Ĺ', 'Ṁ', 'Ñ', 'Ó', 'Þ', 'Ǫ', 'Ŕ', 'Š', 'Ţ', 'Ú', 'Ṽ', 'Ŵ', 'Ẋ', 'Ý', 'Ž',
    ];
    let mut out = String::from("⟦");
    let mut rest = text;
    let mut letters = 0usize;
    while !rest.is_empty() {
        if rest.starts_with("{{")
            && let Some(end) = rest.find("}}")
        {
            out.push_str(&rest[..end + 2]);
            rest = &rest[end + 2..];
            continue;
        }
        let c = rest.chars().next().unwrap_or_default();
        match FROM.find(c) {
            Some(i) => {
                out.push(TO[i]);
                letters += 1;
            }
            None => out.push(c),
        }
        rest = &rest[c.len_utf8()..];
    }
    let pad = letters.div_ceil(3);
    if pad > 0 {
        out.push(' ');
        out.extend(std::iter::repeat_n('~', pad));
    }
    out.push('⟧');
    out
}

#[cfg(test)]
mod tests {
    use super::{lookup, pseudo, t};

    #[test]
    fn looks_up_and_fills_in() {
        assert_eq!(t("tray.quit", &[]), "Quit Repo Launcher");
        assert_eq!(t("exec.finished", &[("name", "Demo")]), "Demo finished.");
        assert_eq!(t("no.such.key", &[]), "no.such.key");
    }

    #[test]
    fn pseudo_keeps_placeholders() {
        assert_eq!(pseudo("Open {{name}}"), "⟦Óþéñ {{name}} ~~⟧");
        assert_eq!(pseudo(""), "⟦⟧");
    }

    /// Every literal key the shell looks up exists in en.json, so no English is hard-coded in
    /// Rust and no key shows up as itself.
    #[test]
    fn every_key_used_in_the_shell_exists() {
        let sources = [
            include_str!("tray.rs"),
            include_str!("exec.rs"),
            include_str!("background.rs"),
            include_str!("lib.rs"),
            include_str!("uninstall.rs"),
        ];
        let mut checked = 0;
        for src in sources {
            let menu = keys(src, "label(\"", "\")")
                .into_iter()
                .map(|k| format!("menu.{k}"));
            for key in keys(src, "t(\"", "\"").into_iter().chain(menu) {
                assert!(lookup(&key).is_some(), "en.json lacks {key}");
                checked += 1;
            }
        }
        assert!(checked > 20, "only {checked} keys found");
    }

    /// The dotted keys in `src` written as `<prefix>key<suffix>`, where the prefix starts a word.
    fn keys(src: &str, prefix: &str, suffix: &str) -> Vec<String> {
        let bytes = src.as_bytes();
        let mut found = Vec::new();
        for (at, _) in src.match_indices(prefix) {
            let starts_word =
                at == 0 || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_');
            let rest = &src[at + prefix.len()..];
            let Some(end) = rest.find(suffix) else {
                continue;
            };
            let key = &rest[..end];
            if starts_word
                && !key.is_empty()
                && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.')
            {
                found.push(key.to_owned());
            }
        }
        found
    }
}
