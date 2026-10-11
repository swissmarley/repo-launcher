//! Splits a simple `package.json` script (`vite --open`, `NODE_ENV=dev next dev`) into env
//! assignments and argv. Anything needing a real shell (`&&`, pipes, `$VAR`, redirects) is
//! rejected so the caller can fall back to running the script through the package manager.

#[derive(Debug, PartialEq, Eq)]
pub struct Simple {
    pub env: Vec<(String, String)>,
    pub argv: Vec<String>,
}

pub fn split(script: &str) -> Option<Simple> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut chars = script.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' | '"' => {
                in_word = true;
                for q in chars.by_ref() {
                    if q == c {
                        break;
                    }
                    if c == '"' && matches!(q, '$' | '`') {
                        return None;
                    }
                    cur.push(q);
                }
            }
            ' ' | '\t' => {
                if in_word {
                    words.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            '&' | '|' | ';' | '<' | '>' | '$' | '`' | '(' | ')' | '\\' | '\n' | '*' => return None,
            _ => {
                in_word = true;
                cur.push(c);
            }
        }
    }
    if in_word {
        words.push(cur);
    }

    let mut env = Vec::new();
    let mut argv = Vec::new();
    for w in words {
        let is_assignment = argv.is_empty()
            && w.split_once('=').is_some_and(|(k, _)| {
                !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            });
        if is_assignment {
            let (k, v) = w.split_once('=').unwrap_or_default();
            env.push((k.to_owned(), v.to_owned()));
        } else {
            argv.push(w);
        }
    }
    // `cross-env FOO=bar cmd` is just a portable way of writing env assignments.
    if argv.first().is_some_and(|a| a == "cross-env") {
        argv.remove(0);
        while let Some((k, v)) = argv
            .first()
            .and_then(|a| a.split_once('='))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
        {
            env.push((k, v));
            argv.remove(0);
        }
    }
    (!argv.is_empty()).then_some(Simple { env, argv })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_simple_scripts() {
        let s = split("vite --open").unwrap();
        assert_eq!(s.argv, ["vite", "--open"]);
        let s = split("NODE_ENV=development next dev --turbo").unwrap();
        assert_eq!(s.env, vec![("NODE_ENV".into(), "development".into())]);
        assert_eq!(s.argv, ["next", "dev", "--turbo"]);
        let s = split("cross-env PORT=4000 node 'server file.js'").unwrap();
        assert_eq!(s.env, vec![("PORT".into(), "4000".into())]);
        assert_eq!(s.argv, ["node", "server file.js"]);
    }

    #[test]
    fn rejects_shell_features() {
        assert!(split("npm run build && vite preview").is_none());
        assert!(split("vite | tee log").is_none());
        assert!(split("node server.js $PORT").is_none());
        assert!(split("echo \"$HOME\"").is_none());
        assert!(split("   ").is_none());
    }
}
