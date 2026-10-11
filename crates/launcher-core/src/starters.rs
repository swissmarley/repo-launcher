//! "Starter projects": the short, curated list an empty library offers (PLAN §11). Each one
//! needs no keys and no account, and each is installed and launched for real on all three OSes
//! by `tests/real_repos.rs` (a test there checks the two lists match). Names and descriptions
//! are the UI's (`starters.<key>` in en.json): the engine never sends English.

use crate::recipe::AppKind;
use serde::Serialize;

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Starter {
    /// Names the wording in en.json.
    pub key: String,
    /// What gets added, exactly as a user would paste it.
    pub link: String,
    pub owner: String,
    pub avatar_url: String,
    pub kind: AppKind,
}

/// (key, link, owner, GitHub user id for the avatar, kind)
const LIST: &[(&str, &str, &str, u64, AppKind)] = &[
    (
        "uberPickups",
        "https://github.com/streamlit/demo-uber-nyc-pickups",
        "streamlit",
        45109972,
        AppKind::Web,
    ),
    (
        "gradioHello",
        "https://github.com/gradio-app/gradio/tree/main/demo/hello_world",
        "gradio-app",
        51063788,
        AppKind::Web,
    ),
    (
        "viteReact",
        "https://github.com/vitejs/vite/tree/main/packages/create-vite/template-react",
        "vitejs",
        65625612,
        AppKind::Web,
    ),
    (
        "astroBasics",
        "https://github.com/withastro/astro/tree/main/examples/basics",
        "withastro",
        44914786,
        AppKind::Web,
    ),
    (
        "cowsay",
        "https://github.com/piuccio/cowsay",
        "piuccio",
        680284,
        AppKind::Terminal,
    ),
];

pub fn list() -> Vec<Starter> {
    LIST.iter()
        .map(|&(key, link, owner, user_id, kind)| Starter {
            key: key.to_owned(),
            link: link.to_owned(),
            owner: owner.to_owned(),
            // The same host the CSP allows for owners' avatars on project cards.
            avatar_url: format!("https://avatars.githubusercontent.com/u/{user_id}?s=96&v=4"),
            kind,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::url;

    #[test]
    fn every_starter_is_a_valid_link() {
        let all = list();
        assert_eq!(all.len(), 5);
        for s in &all {
            let r = url::parse(&s.link).unwrap_or_else(|p| panic!("{}: {p}", s.link));
            assert_eq!(r.owner, s.owner);
        }
    }
}
