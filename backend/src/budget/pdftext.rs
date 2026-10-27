// SPDX-License-Identifier: AGPL-3.0-only

//! PDF のページごとの本文（`pdftotext`。poppler-utils）。AI が返した事業の名前と金額が、
//! そのページに本当に書かれているかを確かめるのに使う。
//! poppler-utils（GPL）はプログラムに組み込まず、別のコマンドとして呼ぶ。
//! PDF は一時ファイルに書き、読んだらすぐに消す（原本を保持しない）。

use std::io::Write;
use std::process::Command;

/// ページごとの本文（`[0]` が1ページ目）
pub fn page_texts(pdf: &[u8]) -> anyhow::Result<Vec<String>> {
    let path = std::env::temp_dir().join(format!("civic-budget-{}.pdf", crate::util::new_id()));
    let result = (|| {
        std::fs::File::create(&path)?.write_all(pdf)?;
        let out = Command::new("pdftotext")
            .args(["-layout", "-enc", "UTF-8"])
            .arg(&path)
            .arg("-")
            .output()
            .map_err(|e| {
                anyhow::anyhow!("pdftotext を実行できません（poppler-utils を入れてください）: {e}")
            })?;
        if !out.status.success() {
            anyhow::bail!(
                "pdftotext が失敗しました: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        // ページの区切りは改ページ（\x0c）
        let text = String::from_utf8_lossy(&out.stdout);
        Ok(text.split('\x0c').map(str::to_owned).collect())
    })();
    let _ = std::fs::remove_file(&path);
    result
}

/// 比べるための形にする。空白・改行・桁区切りのカンマを除き、全角の英数字・記号を半角にする
pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '，')
        .map(|c| match c {
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// 名前が本文に含まれるか。表の中で名前が2行に折り返され、間に金額などが入ることがあるので、
/// 前と後ろの2つに分かれて（この順に）含まれていてもよいことにする
pub fn contains_name(page: &str, name: &str) -> bool {
    let (page, name) = (normalize(page), normalize(name));
    if name.is_empty() {
        return false;
    }
    if page.contains(&name) {
        return true;
    }
    let chars: Vec<char> = name.chars().collect();
    // 前半は2文字以上、後半も2文字以上
    (2..chars.len().saturating_sub(1)).rev().any(|k| {
        let head: String = chars[..k].iter().collect();
        let tail: String = chars[k..].iter().collect();
        page.find(&head)
            .is_some_and(|i| page[i + head.len()..].contains(&tail))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_may_wrap_across_lines() {
        // 大田原市の概要の実物の並び（名前が折り返され、間に金額が入る）
        let page = "とちぎの元気な森づくり市町村交付金\n                           11,914   里山林管理事業補助金等\n事業費\n";
        assert!(contains_name(
            page,
            "とちぎの元気な森づくり市町村交付金事業費"
        ));
        assert!(contains_name(page, "とちぎの元気な森づくり市町村交付金"));
        assert!(!contains_name(page, "架空の事業費"));
    }

    #[test]
    fn normalize_removes_commas_and_full_width() {
        assert_eq!(normalize("１１，９１４ 千円"), "11914千円");
        assert_eq!(normalize("105,325"), "105325");
    }
}
