// SPDX-License-Identifier: AGPL-3.0-only

//! 市民のまとめ（集約ページ。ADR 0005「市民のまとめ」。docs/plans/2026-10-30-topic-map.md）。
//!
//! - 起点の締め切ったトピックと、その近傍（`topic_map::Closed::neighbors`）から選んだトピックごとに、
//!   合意点・意見が分かれた点・グループごとの代表的な意見を、原文と数字のまま並べる
//! - 中身はプログラムだけで決める。AI で論点をまとめない。グループを AI で要約しない（人をひとくくりに名付けない。ADR 0002）
//!   - 合意点: 全グループで (A+1)/(S+2) ≥ 0.8（タイムラインのピン留めと同じ）
//!   - 意見が分かれた点: あるグループで (A+1)/(S+2) ≥ `DIVIDED_THRESHOLD`、かつ別のグループで (D+1)/(S+2) ≥ 同じ値。
//!     5人未満のグループは数えない
//! - 分析の対象が5人未満のトピックは、中身を出さない（分析の公表の決まりと同じ）
//! - 市民はこれを見ながら、自分で次のトピックを書いて提案する（`parent_ids`。週1件の提案に数える）

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use sqlx::SqlitePool;

use crate::process::analysis::conversation::ConvResult;
use crate::process::analysis::disclosure::{
    MIN_CELL_SIZE, MIN_DISPLAY_PARTICIPANTS, PIN_THRESHOLD, group_label, suppress,
};
use crate::process::analysis::repness::RepfulFor;
use crate::process::analysis::store::{self, Stored};
use crate::process::progress::RESIDENCY_TYPES;
use crate::process::topic_map::{self, Closed};

/// 意見が分かれた点のしきい値（2026-10-30 に仮置き。変えるときはここだけ）
pub const DIVIDED_THRESHOLD: f64 = 0.6;
/// 1回のまとめで選べるトピックの数
pub const MAX_TOPICS: usize = 10;

/// 合意点か、意見が分かれた点か
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Consensus,
    Divided,
}

/// グループごとの票
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GroupVotes {
    /// A, B, C…（そのトピックの分析での並び）
    pub label: String,
    pub members: usize,
    pub agree: u32,
    pub disagree: u32,
    /// 投票した人数（パスを含む）
    pub seen: u32,
}

/// 合意点か、意見が分かれた点。数字は分析に含まれた人の票
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    pub kind: ItemKind,
    pub statement_id: String,
    pub body: String,
    pub agree: u32,
    pub disagree: u32,
    pub seen: u32,
    pub groups: Vec<GroupVotes>,
}

fn agree_rate(a: u32, s: u32) -> f64 {
    (f64::from(a) + 1.0) / (f64::from(s) + 2.0)
}

/// トピックの分析から、合意点と意見が分かれた点を選ぶ（合意点が先。それぞれ目立つ順）
pub fn items(
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

/// グループの代表的な意見（Polis の repness。分析で計算済み）
#[derive(Debug, Clone, Serialize)]
pub struct Representative {
    pub statement_id: String,
    pub body: String,
    /// このグループが多く賛成したか、多く反対したか
    pub repful_for: RepfulFor,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupDigest {
    pub label: String,
    pub members: usize,
    pub representative: Vec<Representative>,
}

/// 選んだトピック1件のまとめ
#[derive(Debug, Clone, Serialize)]
pub struct TopicDigest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub closes_at: String,
    /// 起点との似ている度合い（起点は 1）
    pub score: f64,
    /// 分析の対象の人数。5人未満なら中身（下の4つ）は空
    pub analyzed: usize,
    pub sufficient: bool,
    /// 関わり方ごとの人数（二次秘匿済み。隠した区分は null）
    pub residency: BTreeMap<String, Option<i64>>,
    pub consensus: Vec<Item>,
    pub divided: Vec<Item>,
    pub groups: Vec<GroupDigest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Digest {
    pub anchor: String,
    pub divided_threshold: f64,
    pub topics: Vec<TopicDigest>,
}

/// まとめにできない理由
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DigestError {
    #[error("起点のトピックが、この自治体の締め切ったトピックにありません")]
    NoAnchor,
    #[error("起点の近くにない（似ていない）トピックは選べません: {0}")]
    NotNeighbor(String),
    #[error("一度に選べるトピックは{MAX_TOPICS}件までです")]
    TooMany,
}

/// `anchor` を先頭に、重ならないよう並べ、どれも起点の近傍かを確かめる
pub fn check_selection(
    closed: &Closed,
    anchor: &str,
    topic_ids: &[String],
) -> Result<Vec<String>, DigestError> {
    if closed.get(anchor).is_none() {
        return Err(DigestError::NoAnchor);
    }
    let mut ids = vec![anchor.to_owned()];
    for id in topic_ids {
        if ids.contains(id) {
            continue;
        }
        if !closed.in_neighborhood(anchor, id) {
            return Err(DigestError::NotNeighbor(id.clone()));
        }
        ids.push(id.clone());
    }
    if ids.len() > MAX_TOPICS {
        return Err(DigestError::TooMany);
    }
    Ok(ids)
}

/// 分析に含まれた参加者の、関わり方ごとの人数（二次秘匿の済んだもの）
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

/// まとめを作る（保存しない。締め切ったトピックは変わらないので、そのつど計算する）
pub async fn digest(
    pool: &SqlitePool,
    municipality_id: &str,
    anchor: &str,
    topic_ids: &[String],
) -> Result<Result<Digest, DigestError>, sqlx::Error> {
    let closed = topic_map::closed(pool, municipality_id).await?;
    let ids = match check_selection(&closed, anchor, topic_ids) {
        Ok(ids) => ids,
        Err(e) => return Ok(Err(e)),
    };
    let mut topics = Vec::new();
    for id in ids {
        let t = closed.get(&id).expect("確かめ済み");
        let mut out = TopicDigest {
            id: t.id.clone(),
            title: t.title.clone(),
            description: t.description.clone(),
            closes_at: t.closes_at.clone(),
            score: if id == anchor {
                1.0
            } else {
                closed.index.score(anchor, &id)
            },
            analyzed: 0,
            sufficient: false,
            residency: BTreeMap::new(),
            consensus: Vec::new(),
            divided: Vec::new(),
            groups: Vec::new(),
        };
        // 保存した分析を読めない（JSON の形が違う）のは DB の中身の読み違いとして扱う
        let stored = store::load(pool, &id)
            .await
            .map_err(|e| sqlx::Error::Decode(e.into()))?;
        if let Some(Stored::Ready { result, state, .. }) = stored {
            out.analyzed = result.n_in_conv;
            if result.n_in_conv >= MIN_DISPLAY_PARTICIPANTS {
                let bodies: HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
                    "SELECT id, body FROM statements WHERE topic_id = $1 AND status = 'approved'",
                )
                .bind(&id)
                .fetch_all(pool)
                .await?
                .into_iter()
                .collect();
                let (consensus, divided): (Vec<Item>, Vec<Item>) =
                    items(&result, &bodies, DIVIDED_THRESHOLD)
                        .into_iter()
                        .partition(|i| i.kind == ItemKind::Consensus);
                out.sufficient = true;
                out.consensus = consensus;
                out.divided = divided;
                out.groups = groups(&result, &bodies);
                out.residency = residency(pool, municipality_id, &state.in_conv).await?;
            }
        }
        topics.push(out);
    }
    Ok(Ok(Digest {
        anchor: anchor.to_owned(),
        divided_threshold: DIVIDED_THRESHOLD,
        topics,
    }))
}

/// グループごとの代表的な意見（分析の API と同じく、best-agree は賛成として出す）
fn groups(result: &ConvResult, bodies: &HashMap<String, String>) -> Vec<GroupDigest> {
    result
        .groups
        .iter()
        .enumerate()
        .map(|(i, g)| GroupDigest {
            label: group_label(i),
            members: g.n_members,
            representative: result
                .repness
                .get(&g.id)
                .into_iter()
                .flatten()
                .filter_map(|c| {
                    let repful_for = if c.best_agree && c.n_agree.is_some() {
                        RepfulFor::Agree
                    } else {
                        c.repful_for
                    };
                    Some(Representative {
                        body: bodies.get(&c.tid)?.clone(),
                        statement_id: c.tid.clone(),
                        repful_for,
                    })
                })
                .collect(),
        })
        .collect()
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
        let picked = items(&r, &bodies(&tids), DIVIDED_THRESHOLD);
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
        let lower = items(&r, &bodies(&tids), 0.5);
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
        assert!(items(&r, &bodies(&tids), DIVIDED_THRESHOLD).is_empty());
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
        assert!(items(&r, &HashMap::new(), DIVIDED_THRESHOLD).is_empty());
    }
}
