// SPDX-License-Identifier: AGPL-3.0-only

//! robots.txt の最小限の読み取り（`docs/design/collection.md`「取得の共通処理」）。
//!
//! 自分の User-Agent に当てはまるグループ（無ければ `*`）の `Allow` / `Disallow` だけを見る。
//! 規則が複数当てはまるときは、いちばん長いものを採る（同じ長さなら `Allow`。RFC 9309）。
//! `*`（任意の文字列）と末尾の `$`（URL の終わり）に対応する。

/// 1つのサイトの robots.txt のうち、自分に当てはまる規則
#[derive(Debug, Clone, Default)]
pub struct Robots {
    rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
struct Rule {
    allow: bool,
    pattern: String,
}

impl Robots {
    /// robots.txt が無い（404 など）ときは、すべて許可
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// `product` は User-Agent の製品名（例: `civic-platform-collector`）。大文字・小文字は区別しない
    pub fn parse(text: &str, product: &str) -> Self {
        let product = product.to_ascii_lowercase();
        // (グループの User-Agent の一覧, 規則)
        let mut groups: Vec<(Vec<String>, Vec<Rule>)> = Vec::new();
        let mut in_agents = false;
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
            match key.as_str() {
                "user-agent" => {
                    // User-agent が続く間は同じグループ
                    if !in_agents {
                        groups.push((Vec::new(), Vec::new()));
                    }
                    in_agents = true;
                    if let Some(g) = groups.last_mut() {
                        g.0.push(value.to_ascii_lowercase());
                    }
                }
                "allow" | "disallow" => {
                    in_agents = false;
                    let Some(g) = groups.last_mut() else {
                        continue;
                    };
                    // 空の Disallow は「すべて許可」なので規則にしない
                    if value.is_empty() {
                        continue;
                    }
                    g.1.push(Rule {
                        allow: key == "allow",
                        pattern: value.to_owned(),
                    });
                }
                _ => in_agents = false,
            }
        }
        // 自分の名前のグループがあればそれだけ、無ければ `*` のグループ
        let is_mine = |agents: &Vec<String>| {
            agents
                .iter()
                .any(|a| a != "*" && product.contains(a.as_str()))
        };
        let specific = groups.iter().any(|(agents, _)| is_mine(agents));
        let rules = groups
            .into_iter()
            .filter(|(agents, _)| {
                if specific {
                    is_mine(agents)
                } else {
                    agents.iter().any(|a| a == "*")
                }
            })
            .flat_map(|(_, rules)| rules)
            .collect();
        Self { rules }
    }

    /// `path` はパスとクエリ（例: `/iken/zaisei/card.html`）
    pub fn is_allowed(&self, path: &str) -> bool {
        let mut best: Option<(usize, bool)> = None;
        for rule in &self.rules {
            if matches(&rule.pattern, path) {
                let len = rule.pattern.len();
                let better = match best {
                    None => true,
                    Some((l, allow)) => len > l || (len == l && rule.allow && !allow),
                };
                if better {
                    best = Some((len, rule.allow));
                }
            }
        }
        best.is_none_or(|(_, allow)| allow)
    }
}

/// robots.txt のパターン（`*` と末尾の `$`）が、パスの先頭から当てはまるか
fn matches(pattern: &str, path: &str) -> bool {
    let (pattern, anchored) = match pattern.strip_suffix('$') {
        Some(p) => (p, true),
        None => (pattern, false),
    };
    glob(pattern.as_bytes(), path.as_bytes(), anchored)
}

/// `anchored` なら `s` の全体に、そうでなければ `s` の先頭に当てはまるか
fn glob(p: &[u8], s: &[u8], anchored: bool) -> bool {
    match p.first() {
        None => !anchored || s.is_empty(),
        Some(b'*') => (0..=s.len()).any(|i| glob(&p[1..], &s[i..], anchored)),
        Some(c) => s.first() == Some(c) && glob(&p[1..], &s[1..], anchored),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UA: &str = "civic-platform-collector";

    #[test]
    fn other_agents_rules_do_not_apply() {
        // 総務省の robots.txt（2026-10-27）
        let r = Robots::parse("User-agent: ia_archiver\nDisallow: /\n", UA);
        assert!(r.is_allowed("/iken/zaisei/card.html"));
    }

    #[test]
    fn star_group_applies_when_no_specific_group() {
        let r = Robots::parse("User-agent: *\nDisallow: /tenant/js/\n", UA);
        assert!(!r.is_allowed("/tenant/js/app.js"));
        assert!(r.is_allowed("/tenant/ohtawara/pg/index.html"));
    }

    #[test]
    fn specific_group_replaces_star_group() {
        let text = "User-agent: *\nDisallow: /\n\nUser-agent: civic-platform-collector\nDisallow: /private/\n";
        let r = Robots::parse(text, UA);
        assert!(r.is_allowed("/public/a.pdf"));
        assert!(!r.is_allowed("/private/a.pdf"));
    }

    #[test]
    fn longest_match_wins_and_allow_breaks_ties() {
        let text = "User-agent: *\nDisallow: /docs/\nAllow: /docs/open/\nDisallow: /a\nAllow: /a\n";
        let r = Robots::parse(text, UA);
        assert!(!r.is_allowed("/docs/secret.pdf"));
        assert!(r.is_allowed("/docs/open/a.pdf"));
        assert!(r.is_allowed("/a"));
    }

    #[test]
    fn wildcard_and_end_anchor() {
        let text = "User-agent: *\nDisallow: /*.xls$\nDisallow: /tmp*/cache\n";
        let r = Robots::parse(text, UA);
        assert!(!r.is_allowed("/main/a.xls"));
        assert!(r.is_allowed("/main/a.xlsx"));
        assert!(!r.is_allowed("/tmp1/cache/x"));
    }

    #[test]
    fn empty_disallow_and_comments() {
        let text = "# comment\nUser-agent: * # all\nDisallow:\n";
        let r = Robots::parse(text, UA);
        assert!(r.is_allowed("/anything"));
    }

    #[test]
    fn grouped_user_agents_share_rules() {
        let text = "User-agent: foo\nUser-agent: civic-platform-collector\nDisallow: /x/\n";
        let r = Robots::parse(text, UA);
        assert!(!r.is_allowed("/x/1"));
    }
}
