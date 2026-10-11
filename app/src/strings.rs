//! The few words the app shell shows itself: the tray menu and the terminal in `--exec` mode.
//! They come from the UI's own English file, like everything else the user reads.

use serde_json::Value;
use std::sync::OnceLock;

static EN: &str = include_str!("../../ui/src/locales/en.json");

fn table() -> &'static Value {
    static TABLE: OnceLock<Value> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(EN).unwrap_or_default())
}

/// The text at a dotted key such as `tray.quit`, with `{{name}}` placeholders filled in.
pub fn t(key: &str, params: &[(&str, &str)]) -> String {
    let text = key
        .split('.')
        .try_fold(table(), |v, part| v.get(part))
        .and_then(Value::as_str)
        .unwrap_or(key);
    params.iter().fold(text.to_owned(), |s, (k, v)| {
        s.replace(&format!("{{{{{k}}}}}"), v)
    })
}

#[cfg(test)]
mod tests {
    use super::t;

    #[test]
    fn looks_up_and_fills_in() {
        assert_eq!(t("tray.quit", &[]), "Quit Repo Launcher");
        assert_eq!(t("exec.finished", &[("name", "Demo")]), "Demo finished.");
        assert_eq!(t("no.such.key", &[]), "no.such.key");
    }
}
