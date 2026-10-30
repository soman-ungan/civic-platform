// SPDX-License-Identifier: AGPL-3.0-only

//! 取り込んだ会議録と、そこから作ったトピック（CLI の list / show minutes）。
//! 取り込みは `ingest::minutes`。トピックの確認と公開は `records::topics`

use std::sync::LazyLock;

use regex::Regex;
use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct Summary {
    /// `documents.id`
    pub id: String,
    pub slug: String,
    pub title: String,
    pub held_on: Option<String>,
    /// AI の出力の作り方（api / manual）
    pub extraction: Option<String>,
    /// 作ったトピックの数
    pub topics: i64,
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Summary>, sqlx::Error> {
    sqlx::query_as(
        "SELECT d.id, m.slug, d.title, d.held_on, d.extraction,
                (SELECT COUNT(DISTINCT ts.topic_id) FROM topic_sources ts WHERE ts.document_id = d.id) AS topics
         FROM documents d JOIN municipalities m ON m.id = d.municipality_id
         WHERE d.kind = 'minutes' ORDER BY m.slug, d.held_on, d.title",
    )
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct Document {
    pub title: String,
    pub held_on: Option<String>,
    pub url: Option<String>,
    pub extraction: Option<String>,
    pub imported_at: String,
}

/// 会議録から作ったトピック
#[derive(Debug, sqlx::FromRow)]
pub struct TopicLine {
    pub id: String,
    pub status: String,
    pub title: String,
}

/// 1つの会議録と、そこから作ったトピック。無ければ None
pub async fn load(
    pool: &SqlitePool,
    document_id: &str,
) -> Result<Option<(Document, Vec<TopicLine>)>, sqlx::Error> {
    let doc: Option<Document> = sqlx::query_as(
        "SELECT title, held_on, url, extraction, imported_at FROM documents
         WHERE id = $1 AND kind = 'minutes'",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    let Some(doc) = doc else {
        return Ok(None);
    };
    let topics = sqlx::query_as(
        "SELECT DISTINCT t.id, t.status, t.title FROM topics t
         JOIN topic_sources ts ON ts.topic_id = t.id
         WHERE ts.document_id = $1 ORDER BY t.id",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    Ok(Some((doc, topics)))
}

/// 会期の表示（例「令和8年 第2回定例会（6月）」）。`title` は会議録の資料名、`held_on` はその会議の日（月だけを使う）。
/// 資料名の「令和8年」は暦年（第1回定例会は2〜3月）なので、年度とは書かない。
/// 年・回・定例会か臨時会かが読めなければ、資料名から末尾の「（第N号）」を除いたものを出す
/// （自治体によって資料名の形が違えば、ここで読み方を足す）
pub fn session_label(title: &str, held_on: Option<&str>) -> String {
    static SESSION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(令和|平成)(元|\d+)年.*?第(\d+)回.*?(定例会|臨時会)").expect("正しい正規表現")
    });
    static ISSUE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\s*[（(]第\d+号[）)]\s*$").expect("正しい正規表現"));
    let title = ascii_digits(title);
    let base = match SESSION.captures(&title) {
        Some(c) => {
            let year = if &c[2] == "元" {
                "元"
            } else {
                c[2].trim_start_matches('0')
            };
            format!(
                "{}{}年 第{}回{}",
                &c[1],
                year,
                c[3].trim_start_matches('0'),
                &c[4]
            )
        }
        None => ISSUE.replace(&title, "").into_owned(),
    };
    let month = held_on
        .and_then(|d| d.get(5..7))
        .and_then(|m| m.parse::<u32>().ok());
    match month {
        Some(m) => format!("{base}（{m}月）"),
        None => base,
    }
}

/// 全角の数字を半角にする（資料名は「令和８年第２回」のように全角のことがある）
fn ascii_digits(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '０'..='９' => char::from(b'0' + (c as u32 - '０' as u32) as u8),
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::session_label;

    #[test]
    fn reads_the_session_from_the_title() {
        assert_eq!(
            session_label(
                "令和８年第２回大田原市議会定例会（第３号）",
                Some("2026-06-17")
            ),
            "令和8年 第2回定例会（6月）"
        );
        assert_eq!(
            session_label("令和元年第1回臨時会", Some("2019-05-01")),
            "令和元年 第1回臨時会（5月）"
        );
        // 読めなければ資料名から号を除く
        assert_eq!(
            session_label("予算特別委員会（第２号）", None),
            "予算特別委員会"
        );
    }
}
