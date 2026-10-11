//! Chooses a Python minor version from a PEP 440 `requires-python` specifier.

/// Newest first. 3.12 is preferred because it has the broadest prebuilt-package coverage
/// (see DECISIONS D-016).
pub const CANDIDATES: &[(u32, u32)] = &[(3, 13), (3, 12), (3, 11), (3, 10), (3, 9)];
pub const DEFAULT: (u32, u32) = (3, 12);

pub fn choose(requires: Option<&str>, pinned: Option<&str>) -> String {
    if let Some(p) = pinned.and_then(parse_minor) {
        return fmt(p);
    }
    let Some(spec) = requires else {
        return fmt(DEFAULT);
    };
    if satisfies(spec, DEFAULT) {
        return fmt(DEFAULT);
    }
    CANDIDATES
        .iter()
        .copied()
        .find(|v| satisfies(spec, *v))
        .map(fmt)
        .unwrap_or_else(|| fmt(DEFAULT))
}

fn fmt((a, b): (u32, u32)) -> String {
    format!("{a}.{b}")
}

fn parse_minor(s: &str) -> Option<(u32, u32)> {
    let s = s
        .trim()
        .trim_start_matches("python")
        .trim_start_matches('-');
    let mut it = s.split('.');
    let major = it.next()?.trim().parse().ok()?;
    let minor = it.next()?.trim().trim_end_matches('*').parse().ok()?;
    Some((major, minor))
}

/// Evaluates each comma-separated clause against `X.Y` (treated as `X.Y.0` for lower bounds and
/// as the whole `X.Y` series for `==X.Y.*`-style clauses). Unknown clauses are ignored.
pub fn satisfies(spec: &str, v: (u32, u32)) -> bool {
    spec.split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .all(|clause| {
            let (op, ver) = split_op(clause);
            let Some(target) = parse_version(ver) else {
                return true;
            };
            let cur = [v.0, v.1, 0];
            let cmp = compare(&cur, &target);
            match op {
                ">=" => cmp.is_ge(),
                ">" => cmp.is_gt() && !(target.len() <= 2 && cur[..2] == target[..2]),
                "<=" => cmp.is_le() || prefix_eq(&cur, &target),
                "<" => cmp.is_lt(),
                "==" | "===" => prefix_eq(&cur, &target),
                "!=" => !prefix_eq(&cur, &target) || target.len() > 2,
                "~=" => {
                    // ~=3.10 means >=3.10,<4 ; ~=3.10.2 means >=3.10.2,<3.11
                    let upper: Vec<u32> = if target.len() <= 2 {
                        vec![target[0] + 1]
                    } else {
                        vec![target[0], target[1] + 1]
                    };
                    compare(&cur, &target).is_ge() && compare(&cur, &upper).is_lt()
                }
                _ => true,
            }
        })
}

fn split_op(clause: &str) -> (&str, &str) {
    for op in ["===", "~=", "==", "!=", ">=", "<=", ">", "<"] {
        if let Some(rest) = clause.strip_prefix(op) {
            return (op, rest.trim());
        }
    }
    ("==", clause)
}

fn parse_version(s: &str) -> Option<Vec<u32>> {
    let parts: Vec<u32> = s
        .trim_end_matches(".*")
        .split('.')
        .map(|p| {
            p.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect::<Option<_>>()?;
    (!parts.is_empty()).then_some(parts)
}

fn compare(a: &[u32], b: &[u32]) -> std::cmp::Ordering {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| a.get(i).unwrap_or(&0).cmp(b.get(i).unwrap_or(&0)))
        .find(|o| o.is_ne())
        .unwrap_or(std::cmp::Ordering::Equal)
}

fn prefix_eq(cur: &[u32], target: &[u32]) -> bool {
    target
        .iter()
        .enumerate()
        .all(|(i, t)| cur.get(i) == Some(t) || (i >= 2 && *t == 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_versions() {
        assert_eq!(choose(None, None), "3.12");
        assert_eq!(choose(Some(">=3.9"), None), "3.12");
        assert_eq!(choose(Some(">=3.13"), None), "3.13");
        assert_eq!(choose(Some(">=3.8,<3.12"), None), "3.11");
        assert_eq!(choose(Some("~=3.10"), None), "3.12");
        assert_eq!(choose(Some("==3.10.*"), None), "3.10");
        assert_eq!(choose(Some(">=3.10,<4"), None), "3.12");
        assert_eq!(choose(Some(">=3.9, !=3.12.*"), None), "3.13");
        assert_eq!(choose(Some(">=3.12"), Some("3.11.9")), "3.11");
        assert_eq!(choose(Some("garbage"), None), "3.12");
    }
}
