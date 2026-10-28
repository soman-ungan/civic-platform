// SPDX-License-Identifier: AGPL-3.0-only

//! PDF のページごとの本文（`pdftotext`。poppler-utils）。AI が返した事業の名前と金額が、
//! そのページに本当に書かれているかを確かめるのに使う。
//! poppler-utils（GPL）はプログラムに組み込まず、別のコマンドとして呼ぶ。
//! PDF は一時ファイルに書き、読んだらすぐに消す（原本を保持しない）。

use std::io::Write;
use std::process::Command;

/// ページごとの本文（`[0]` が1ページ目）
pub fn page_texts(pdf: &[u8]) -> anyhow::Result<Vec<String>> {
    let path = std::env::temp_dir().join(format!(
        "civic-budget-{}.pdf",
        crate::shared::util::new_id()
    ));
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

/// 比べるための形にする。空白・改行・桁区切りのカンマを除き、全角の英数字・記号を半角にする。
/// 長音とハイフンの類い（ー・－・‐ など）は同じ文字にそろえる（PDF では「ハ－モニ－」のように
/// 全角のハイフンで書かれていることがある）
pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '，')
        .map(|c| match c {
            'ー' | '－' | '-' | '‐' | '‑' | '‒' | '–' | '—' | '―' | 'ｰ' | '−' => {
                'ー'
            }
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// 名前の一部として認める最も短いかけら（文字）
const MIN_PART: usize = 2;
/// 名前が分かれてよい数（表の中で何行かに折り返されることがある）
const MAX_PARTS: usize = 4;

/// 名前が本文に含まれるか。表の中で名前が何行かに折り返され、間に金額などが入ることがあるので、
/// いくつかのかけらに分かれて（この順に）含まれていてもよいことにする。
/// 前から順に、本文に含まれる最も長いかけらを取っていく
pub fn contains_name(page: &str, name: &str) -> bool {
    let (page, name) = (normalize(page), normalize(name));
    if name.is_empty() {
        return false;
    }
    if page.contains(&name) {
        return true;
    }
    let chars: Vec<char> = name.chars().collect();
    let (mut from, mut at, mut parts) = (0usize, 0usize, 0usize);
    while from < chars.len() {
        if parts == MAX_PARTS {
            return false;
        }
        let rest = &page[at..];
        // ここから始まる、本文に含まれる最も長いかけら
        let Some((len, pos)) = (MIN_PART.min(chars.len() - from)..=chars.len() - from)
            .rev()
            .find_map(|len| {
                let part: String = chars[from..from + len].iter().collect();
                rest.find(&part).map(|pos| (len, pos + part.len()))
            })
        else {
            return false;
        };
        from += len;
        at += pos;
        parts += 1;
    }
    true
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
    fn names_over_three_lines_and_dash_variants() {
        // 大田原市の令和8年度の概要の実物（19ページ・14ページ）
        let page = "戸籍住民基本台帳費、住民基本台帳等\n      戸籍ソフトウェアリース料、住民情報管理業務委託、\n電算処理事務費、住民基本台帳ネット            28,248\n      マイナンバーカード交付業務委託等\nワークシステム管理費\n";
        assert!(contains_name(
            page,
            "戸籍住民基本台帳費、住民基本台帳等電算処理事務費、住民基本台帳ネットワークシステム管理費"
        ));
        let page = "那須野が原ハ－モニ－ホ－ル管理運営費          378,758\n";
        assert!(contains_name(page, "那須野が原ハーモニーホール管理運営費"));
        // 順番が違うものは認めない
        assert!(!contains_name(
            "管理運営費 那須野が原",
            "那須野が原管理運営費の補助"
        ));
    }

    #[test]
    fn normalize_removes_commas_and_full_width() {
        assert_eq!(normalize("１１，９１４ 千円"), "11914千円");
        assert_eq!(normalize("105,325"), "105325");
    }
}
