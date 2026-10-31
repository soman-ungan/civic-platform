// SPDX-License-Identifier: AGPL-3.0-only

//! 議員の記録（ADR 0008）。議員 ＞ その人の発言が出典になったトピック ＞ 発言の引用。
//! 中身は `records::officials`。順位・評価・市民の投票の結果は返さない

use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;

use crate::interface::api::AppState;
use crate::interface::api::error::ApiError;
use crate::interface::api::municipalities;
use crate::interface::api::topics::Tag;
use crate::process::progress;
use crate::records::officials;

#[derive(Debug, Serialize)]
pub struct Official {
    pub id: String,
    pub name: String,
    /// mayor / councilor
    pub role: String,
    pub term_start: String,
    pub term_end: Option<String>,
    pub topic_count: i64,
}

impl From<officials::Official> for Official {
    fn from(o: officials::Official) -> Self {
        Self {
            id: o.id,
            name: o.name,
            role: o.role,
            term_start: o.term_start,
            term_end: o.term_end,
            topic_count: o.topic_count,
        }
    }
}

/// 取り込んだ会議録の範囲（YYYY-MM-DD）。会議録をまだ取り込んでいなければ null
#[derive(Debug, Serialize)]
pub struct Coverage {
    pub from: String,
    pub to: String,
}

async fn coverage(state: &AppState, municipality_id: &str) -> Result<Option<Coverage>, ApiError> {
    Ok(officials::coverage(&state.pool, municipality_id)
        .await?
        .map(|(from, to)| Coverage { from, to }))
}

#[derive(Debug, Serialize)]
pub struct OfficialList {
    pub officials: Vec<Official>,
    pub coverage: Option<Coverage>,
}

/// GET /api/municipalities/{slug}/officials — 現職の首長・議員と、任期の終わった人のうち記録のある人。
/// 首長が先、議員は読みの順（数では並べない）
pub async fn list(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<OfficialList>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    let list = officials::list(&state.pool, &municipality.id).await?;
    Ok(Json(OfficialList {
        officials: list.into_iter().map(Official::from).collect(),
        coverage: coverage(&state, &municipality.id).await?,
    }))
}

#[derive(Debug, Serialize)]
pub struct Quote {
    pub quote: String,
    pub document_title: String,
    pub held_on: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OfficialTopic {
    pub id: String,
    pub title: String,
    pub closed: bool,
    pub tags: Vec<Tag>,
    /// この人の発言の引用（会議録の原文と一致するもの）
    pub quotes: Vec<Quote>,
}

#[derive(Debug, Serialize)]
pub struct OfficialDetail {
    #[serde(flatten)]
    pub official: Official,
    /// 新しい会議の順
    pub topics: Vec<OfficialTopic>,
    pub coverage: Option<Coverage>,
}

/// GET /api/municipalities/{slug}/officials/{official_id} — その人の発言が出典になった公開済みのトピックと引用
pub async fn detail(
    State(state): State<AppState>,
    Path((slug, official_id)): Path<(String, String)>,
) -> Result<Json<OfficialDetail>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    let official = officials::find(&state.pool, &municipality.id, &official_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let quotes = officials::quotes(&state.pool, &official.id).await?;
    let tags = officials::topic_tags(&state.pool, &official.id).await?;

    // 引用は新しい会議の順に並んでいるので、トピックはその人の最初に出てきた順（= 新しい順）にまとめる
    let mut topics: Vec<OfficialTopic> = Vec::new();
    for q in quotes {
        let quote = Quote {
            quote: q.quote,
            document_title: q.document_title,
            held_on: q.held_on,
            url: q.url,
        };
        match topics.iter_mut().find(|t| t.id == q.topic_id) {
            Some(topic) => topic.quotes.push(quote),
            None => topics.push(OfficialTopic {
                tags: tags
                    .iter()
                    .filter(|t| t.topic_id == q.topic_id)
                    .map(|t| Tag {
                        id: t.id.clone(),
                        kind: t.kind.clone(),
                        name: t.name.clone(),
                    })
                    .collect(),
                closed: progress::is_closed(q.closes_at.as_deref()),
                id: q.topic_id,
                title: q.topic_title,
                quotes: vec![quote],
            }),
        }
    }

    Ok(Json(OfficialDetail {
        official: official.into(),
        topics,
        coverage: coverage(&state, &municipality.id).await?,
    }))
}
