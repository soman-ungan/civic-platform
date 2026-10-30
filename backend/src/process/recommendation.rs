// SPDX-License-Identifier: AGPL-3.0-only

//! 提言の中身を決める（ADR 0005「提言の作り方」。docs/plans/2026-10-30-opinion-synthesis.md）。
//!
//! - 提言にできるのは、締め切ったトピックだけ（結果が固まっている）。分析の対象が5人以上、意見のグループが2つ以上
//! - 項目はプログラムだけで決める（AI も運営者も足し引きしない）
//!   - 合意点: 全グループで (A+1)/(S+2) ≥ 0.8（タイムラインのピン留めと同じ）
//!   - 意見が分かれた点: あるグループで (A+1)/(S+2) ≥ しきい値、かつ別のグループで (D+1)/(S+2) ≥ しきい値。
//!     5人未満のグループは数えない
//! - 子の提言（公開済み）の項目は、見出し・前書きを使わずに原文と数字をたどって集める
//! - AI が書くのは中立な見出しと短い前書きの下書きだけ。送るのは項目の原文と元のトピックの題（公開済みのもの）

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::process::analysis::conversation::ConvResult;
use crate::process::analysis::disclosure::{
    MIN_CELL_SIZE, MIN_DISPLAY_PARTICIPANTS, PIN_THRESHOLD, group_label, suppress,
};
use crate::process::analysis::store::{self, Stored};
use crate::process::moderation;
use crate::process::progress::{self, RESIDENCY_TYPES};
use crate::process::similarity::{self, Index};
use crate::records::recommendations::{self, GroupVotes, Item, NewRecommendation, NewTopic};
use crate::shared::llm::{Effort, Llm, LlmError, Purpose, Request};
use crate::shared::status::ItemKind;

/// 意見が分かれた点のしきい値の既定（2026-10-30 に仮置き。`.env` の `RECOMMENDATION_DIVIDED_THRESHOLD` で変える）
pub const DEFAULT_DIVIDED_THRESHOLD: f64 = 0.6;
/// 見出しと前書きの長さの上限（DB の CHECK と同じ）
pub const MAX_TITLE_CHARS: usize = 60;
pub const MAX_PREFACE_CHARS: usize = 400;

fn agree_rate(a: u32, s: u32) -> f64 {
    (f64::from(a) + 1.0) / (f64::from(s) + 2.0)
}

/// 締め切ったトピックの分析から、合意点と意見が分かれた点を選ぶ（合意点が先。それぞれ目立つ順）
pub fn items(
    topic_id: &str,
    result: &ConvResult,
    bodies: &HashMap<String, String>,
    divided_threshold: f64,
) -> Vec<Item> {
    let min_members = MIN_CELL_SIZE as usize;
    let mut consensus = Vec::new();
    let mut divided = Vec::new();
    for tid in &result.tids {
        // 分析のあとに非公開になった意見は出さない
        let Some(body) = bodies.get(tid) else {
            continue;
        };
        let counts: Vec<_> = result.groups.iter().map(|g| (g, g.votes[tid])).collect();
        let item = |kind| Item {
            kind,
            topic_id: topic_id.to_owned(),
            statement_id: tid.clone(),
            body: body.clone(),
            agree: counts.iter().map(|(_, c)| c.a).sum(),
            disagree: counts.iter().map(|(_, c)| c.d).sum(),
            seen: counts.iter().map(|(_, c)| c.s).sum(),
            groups: counts
                .iter()
                .enumerate()
                .map(|(i, (g, c))| GroupVotes {
                    label: group_label(i),
                    members: g.n_members,
                    agree: c.a,
                    disagree: c.d,
                    seen: c.s,
                })
                .collect(),
        };
        if counts
            .iter()
            .all(|(_, c)| agree_rate(c.a, c.s) >= PIN_THRESHOLD)
        {
            let gac = result
                .group_aware_consensus
                .get(tid)
                .copied()
                .unwrap_or(0.0);
            consensus.push((gac, item(ItemKind::Consensus)));
            continue;
        }
        // あるグループの賛成と、別のグループの反対の、弱いほうが一番強い組
        let counted: Vec<_> = counts
            .iter()
            .filter(|(g, _)| g.n_members >= min_members)
            .map(|(_, c)| *c)
            .collect();
        let mut best = 0.0_f64;
        for (i, a) in counted.iter().enumerate() {
            for (j, d) in counted.iter().enumerate() {
                if i != j {
                    best = best.max(agree_rate(a.a, a.s).min(agree_rate(d.d, d.s)));
                }
            }
        }
        if best >= divided_threshold {
            divided.push((best, item(ItemKind::Divided)));
        }
    }
    let ordered = |mut v: Vec<(f64, Item)>| {
        v.sort_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then(a.1.statement_id.cmp(&b.1.statement_id))
        });
        v.into_iter().map(|(_, item)| item)
    };
    ordered(consensus).chain(ordered(divided)).collect()
}

/// 提言にできない理由
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Ineligible {
    #[error("トピックが見つかりません（別の自治体のものも含む）: {0}")]
    NotFound(String),
    #[error("締め切っていないトピックは提言にできません: {0}")]
    NotClosed(String),
    #[error("分析の対象が{MIN_DISPLAY_PARTICIPANTS}人未満です: {0}")]
    TooFewParticipants(String),
    #[error("意見のグループが1つしかありません: {0}")]
    OneGroup(String),
}

/// 提言にできるトピック（分析の結果つき）
pub struct Eligible {
    pub topic_id: String,
    pub title: String,
    pub result: Box<ConvResult>,
    /// 分析に含まれた参加者
    pub in_conv: Vec<String>,
}

/// トピックが提言にできるかを確かめる（公開済み・締め切った・5人以上・2グループ以上）
pub async fn check_topic(
    pool: &SqlitePool,
    municipality_id: &str,
    topic_id: &str,
) -> Result<Result<Eligible, Ineligible>, sqlx::Error> {
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT title, closes_at FROM topics
         WHERE id = $1 AND municipality_id = $2 AND status = 'published'",
    )
    .bind(topic_id)
    .bind(municipality_id)
    .fetch_optional(pool)
    .await?;
    let id = || topic_id.to_owned();
    let Some((title, closes_at)) = row else {
        return Ok(Err(Ineligible::NotFound(id())));
    };
    if !progress::is_closed(closes_at.as_deref()) {
        return Ok(Err(Ineligible::NotClosed(id())));
    }
    // 保存した分析を読めない（JSON の形が違う）のは DB の中身の読み違いとして扱う
    let stored = store::load(pool, topic_id)
        .await
        .map_err(|e| sqlx::Error::Decode(e.into()))?;
    let Some(Stored::Ready { result, state, .. }) = stored else {
        return Ok(Err(Ineligible::TooFewParticipants(id())));
    };
    if result.n_in_conv < MIN_DISPLAY_PARTICIPANTS {
        return Ok(Err(Ineligible::TooFewParticipants(id())));
    }
    if result.groups.len() < 2 {
        return Ok(Err(Ineligible::OneGroup(id())));
    }
    Ok(Ok(Eligible {
        topic_id: id(),
        title,
        result,
        in_conv: state.in_conv.clone(),
    }))
}

/// 提言にできるトピック（運営者が選ぶ一覧）
#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub id: String,
    pub title: String,
    pub closes_at: Option<String>,
    pub participants: usize,
    /// 分野タグの名前
    pub fields: Vec<String>,
}

/// 自治体の、提言にできるトピック（締め切った順）と、まとまりの候補（同じ分野タグを持ち、似ているもの）
pub async fn candidates(
    pool: &SqlitePool,
    municipality_id: &str,
) -> Result<(Vec<Candidate>, Vec<Vec<String>>), sqlx::Error> {
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, description, closes_at FROM topics
         WHERE municipality_id = $1 AND status = 'published' AND closes_at IS NOT NULL
         ORDER BY closes_at DESC, id",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    let mut texts = Vec::new();
    for (id, description, closes_at) in rows {
        let Ok(eligible) = check_topic(pool, municipality_id, &id).await? else {
            continue;
        };
        let fields: Vec<String> = sqlx::query_scalar(
            "SELECT t.name FROM topic_tags tt JOIN tags t ON t.id = tt.tag_id
             WHERE tt.topic_id = $1 AND t.kind = 'field' ORDER BY t.id",
        )
        .bind(&id)
        .fetch_all(pool)
        .await?;
        texts.push((id.clone(), format!("{} {description}", eligible.title)));
        out.push(Candidate {
            id,
            title: eligible.title,
            closes_at,
            participants: eligible.result.n_in_conv,
            fields,
        });
    }
    let fields: HashMap<&str, HashSet<&str>> = out
        .iter()
        .map(|c| (c.id.as_str(), c.fields.iter().map(String::as_str).collect()))
        .collect();
    let index = Index::build(texts);
    let pairs: Vec<(String, String)> = index
        .pairs(similarity::SIMILAR_TOPIC_SCORE)
        .into_iter()
        .filter(|(a, b, _)| !fields[a.as_str()].is_disjoint(&fields[b.as_str()]))
        .map(|(a, b, _)| (a, b))
        .collect();
    let ids: Vec<String> = out.iter().map(|c| c.id.clone()).collect();
    Ok((out, similarity::connect(&ids, &pairs)))
}

/// 分析に含まれた参加者の、関わり方ごとの人数（二次秘匿の済んだもの。隠した区分は None）
async fn residency(
    pool: &SqlitePool,
    municipality_id: &str,
    in_conv: &[String],
) -> Result<BTreeMap<String, Option<i64>>, sqlx::Error> {
    let relations: HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
        "SELECT user_id, residency_type FROM user_municipality_relations WHERE municipality_id = $1",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();
    let mut counts: HashMap<&str, i64> = HashMap::new();
    for pid in in_conv {
        if let Some(t) = relations.get(pid) {
            *counts.entry(t.as_str()).or_default() += 1;
        }
    }
    let values: Vec<i64> = RESIDENCY_TYPES
        .iter()
        .map(|t| counts.get(t).copied().unwrap_or(0))
        .collect();
    let visible = suppress(&values);
    Ok(RESIDENCY_TYPES
        .iter()
        .zip(values.iter().zip(&visible))
        .map(|(t, (n, v))| ((*t).to_owned(), v.then_some(*n)))
        .collect())
}

/// 提言を作れない理由
#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    #[error("トピックか子の提言を1つ以上選んでください")]
    Empty,
    #[error(transparent)]
    Ineligible(#[from] Ineligible),
    #[error("子にできるのは、同じ自治体の公開済みの提言だけです: {0}")]
    InvalidChild(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// 提言の下書きを作る（見出しと前書きは空。`draft_text` か運営者が書く）。作った提言の ID を返す
pub async fn create(
    pool: &SqlitePool,
    municipality_id: &str,
    topic_ids: &[String],
    child_ids: &[String],
    divided_threshold: f64,
) -> Result<String, CreateError> {
    let topic_ids: Vec<&String> = dedup(topic_ids);
    let child_ids: Vec<&String> = dedup(child_ids);
    if topic_ids.is_empty() && child_ids.is_empty() {
        return Err(CreateError::Empty);
    }
    let mut topics = Vec::new();
    let mut items_out: Vec<Item> = Vec::new();
    let mut seen_statements = HashSet::new();
    for topic_id in topic_ids {
        let eligible = check_topic(pool, municipality_id, topic_id).await??;
        let bodies: HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
            "SELECT id, body FROM statements WHERE topic_id = $1 AND status = 'approved'",
        )
        .bind(topic_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect();
        for item in items(topic_id, &eligible.result, &bodies, divided_threshold) {
            if seen_statements.insert(item.statement_id.clone()) {
                items_out.push(item);
            }
        }
        topics.push(NewTopic {
            topic_id: topic_id.clone(),
            participants: eligible.result.n_in_conv as i64,
            residency: residency(pool, municipality_id, &eligible.in_conv).await?,
        });
    }
    for child_id in &child_ids {
        if !recommendations::is_published_in(pool, municipality_id, child_id).await? {
            return Err(CreateError::InvalidChild((*child_id).clone()));
        }
        // 子の見出し・前書きは使わず、項目（原文と数字）をたどって集める
        for item in recommendations::items(pool, child_id).await? {
            if seen_statements.insert(item.statement_id.clone()) {
                items_out.push(item);
            }
        }
    }
    let id = recommendations::insert(
        pool,
        &NewRecommendation {
            municipality_id: municipality_id.to_owned(),
            consensus_threshold: PIN_THRESHOLD,
            divided_threshold,
            topics,
            children: child_ids.into_iter().cloned().collect(),
            items: items_out,
        },
    )
    .await?;
    Ok(id)
}

fn dedup(ids: &[String]) -> Vec<&String> {
    let mut seen = HashSet::new();
    ids.iter().filter(|id| seen.insert(id.as_str())).collect()
}

// ===== 見出しと前書きの下書き（AI） =====

const SYSTEM_PROMPT: &str = "\
あなたは、自治体の政策について市民が投票した結果を「提言」のページにまとめる編集係です。
提言の中身（合意点と意見が分かれた点）は、市民の投票からプログラムが決めた原文です。あなたが書くのは、その見出しと短い前書きだけです。

- 見出し（title）: 何についての提言かを、中立な言葉で40文字以内で書く。例「市内の路線バスの便数についての提言」
- 前書き（preface）: どんな話題について、どのような点で意見が一致し、どのような点で分かれたかを、200文字以内で淡々と紹介する
- 原文にない主張・結論・提案を作らない。賛否や良し悪しを評価しない。どちらかの立場に寄せない
- 人の氏名を書かない。数字（人数・割合）は書かない（ページに原文と数字がそのまま載る）
";

fn text_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "title": {"type": "string"},
            "preface": {"type": "string"}
        },
        "required": ["title", "preface"],
        "additionalProperties": false
    })
}

/// AI に渡す入力: 元のトピックの題と、項目の原文
pub fn text_input(topic_titles: &[String], items: &[Item]) -> String {
    let mut out = String::from("<topics>\n");
    for t in topic_titles {
        out.push_str(&format!("- {t}\n"));
    }
    out.push_str("</topics>\n<consensus>\n");
    for i in items.iter().filter(|i| i.kind == ItemKind::Consensus) {
        out.push_str(&format!("- {}\n", i.body));
    }
    out.push_str("</consensus>\n<divided>\n");
    for i in items.iter().filter(|i| i.kind == ItemKind::Divided) {
        out.push_str(&format!("- {}\n", i.body));
    }
    out.push_str("</divided>\n");
    out
}

#[derive(Debug, Deserialize)]
struct DraftText {
    title: String,
    preface: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TextError {
    #[error("下書きの提言が見つかりません: {0}")]
    NotFound(String),
    #[error("AI の下書きに公人の氏名（{0}）があるので使いませんでした。手で書いてください")]
    Name(String),
    #[error(transparent)]
    Llm(#[from] LlmError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

fn cut(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

/// 下書きの提言の見出しと前書きを AI に書いてもらい、保存する（上書き）。保存した (見出し, 前書き) を返す
pub async fn draft_text(
    pool: &SqlitePool,
    llm: &Llm,
    model: &str,
    recommendation_id: &str,
) -> Result<(String, String), TextError> {
    let Some(detail) = recommendations::show(pool, recommendation_id).await? else {
        return Err(TextError::NotFound(recommendation_id.to_owned()));
    };
    if detail.status != "draft" {
        return Err(TextError::NotFound(recommendation_id.to_owned()));
    }
    let titles: Vec<String> = detail.topics.iter().map(|t| t.title.clone()).collect();
    let request = Request {
        purpose: Purpose::Recommendation,
        model: model.to_owned(),
        system: SYSTEM_PROMPT.into(),
        user: text_input(&titles, &detail.items),
        schema: text_schema(),
        effort: Effort::Low,
        max_tokens: 2048,
        pdf: None,
    };
    let value = llm.structured(pool, &request).await?;
    let text: DraftText =
        serde_json::from_value(value).map_err(|e| LlmError::Invalid(e.to_string()))?;
    let (title, preface) = (
        cut(&text.title, MAX_TITLE_CHARS),
        cut(&text.preface, MAX_PREFACE_CHARS),
    );
    let officials = moderation::official_names(pool, &detail.municipality_id).await?;
    if let Some(name) = moderation::official_name_in(&format!("{title}{preface}"), &officials) {
        return Err(TextError::Name(name));
    }
    // 長さは切ってあるので、失敗するのは下書きでなくなったときだけ
    if recommendations::edit(pool, recommendation_id, Some(&title), Some(&preface))
        .await?
        .is_err()
    {
        return Err(TextError::NotFound(recommendation_id.to_owned()));
    }
    Ok((title, preface))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::process::analysis::conversation::{GroupOut, VoteCounts};

    fn group(id: i64, members: usize, votes: &[(&str, u32, u32, u32)]) -> GroupOut {
        GroupOut {
            id,
            base_ids: vec![],
            n_members: members,
            votes: votes
                .iter()
                .map(|(t, a, d, s)| {
                    (
                        (*t).to_owned(),
                        VoteCounts {
                            a: *a,
                            d: *d,
                            s: *s,
                        },
                    )
                })
                .collect(),
        }
    }

    fn result(groups: Vec<GroupOut>, tids: &[&str]) -> ConvResult {
        let mut r: ConvResult = serde_json::from_value(serde_json::json!({
            "n_participants": 0, "n_in_conv": 0, "tids": [], "pca": {"center": [], "comps": []},
            "comment_projection": [], "comment_extremity": [], "base_clusters": [], "groups": [],
            "repness": {}, "group_aware_consensus": {}, "consensus": {"agree": [], "disagree": []},
            "comment_priorities": {}
        }))
        .unwrap();
        r.tids = tids.iter().map(|t| (*t).to_owned()).collect();
        r.groups = groups;
        r
    }

    fn bodies(tids: &[&str]) -> HashMap<String, String> {
        tids.iter()
            .map(|t| ((*t).to_owned(), format!("意見{t}")))
            .collect()
    }

    #[test]
    fn picks_consensus_and_divided() {
        let tids = ["agree", "split", "mild", "none"];
        let r = result(
            vec![
                group(
                    0,
                    10,
                    &[
                        ("agree", 9, 0, 10),
                        ("split", 8, 1, 10),
                        ("mild", 6, 2, 10),
                        ("none", 2, 2, 10),
                    ],
                ),
                group(
                    1,
                    8,
                    &[
                        ("agree", 7, 0, 8),
                        ("split", 0, 7, 8),
                        ("mild", 2, 5, 8),
                        ("none", 3, 3, 8),
                    ],
                ),
            ],
            &tids,
        );
        let picked = items("t", &r, &bodies(&tids), DEFAULT_DIVIDED_THRESHOLD);
        let kinds: Vec<(&str, ItemKind)> = picked
            .iter()
            .map(|i| (i.statement_id.as_str(), i.kind))
            .collect();
        assert_eq!(
            kinds,
            [("agree", ItemKind::Consensus), ("split", ItemKind::Divided)]
        );
        assert_eq!((picked[0].agree, picked[0].seen), (16, 18));
        assert_eq!(picked[1].groups[1].label, "B");
        // しきい値を下げれば、弱く分かれた意見も入る
        let lower = items("t", &r, &bodies(&tids), 0.5);
        assert!(lower.iter().any(|i| i.statement_id == "mild"));
    }

    #[test]
    fn small_groups_do_not_make_a_split() {
        let tids = ["split"];
        let r = result(
            vec![
                group(0, 10, &[("split", 9, 0, 10)]),
                group(1, 3, &[("split", 0, 3, 3)]),
            ],
            &tids,
        );
        assert!(items("t", &r, &bodies(&tids), DEFAULT_DIVIDED_THRESHOLD).is_empty());
    }

    #[test]
    fn hidden_statements_are_left_out() {
        let tids = ["agree"];
        let r = result(
            vec![
                group(0, 10, &[("agree", 9, 0, 10)]),
                group(1, 8, &[("agree", 7, 0, 8)]),
            ],
            &tids,
        );
        assert!(items("t", &r, &HashMap::new(), DEFAULT_DIVIDED_THRESHOLD).is_empty());
    }

    #[test]
    fn text_input_has_only_titles_and_bodies() {
        let item = Item {
            kind: ItemKind::Divided,
            topic_id: "t-secret".into(),
            statement_id: "s-secret".into(),
            body: "バスを増やす".into(),
            agree: 1,
            disagree: 1,
            seen: 2,
            groups: vec![],
        };
        let input = text_input(&["路線バス".into()], &[item]);
        assert!(input.contains("路線バス") && input.contains("<divided>\n- バスを増やす"));
        assert!(!input.contains("secret"));
    }
}
