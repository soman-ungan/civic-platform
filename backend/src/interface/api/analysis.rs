// SPDX-License-Identifier: AGPL-3.0-only

//! 分析結果の API。参加者 ID は返さない（基底クラスタは中心と人数だけ）。

use std::collections::{BTreeMap, HashMap};

use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::interface::api::AppState;
use crate::interface::api::error::ApiError;
use crate::interface::api::municipalities;
use crate::interface::api::participant::MaybeParticipant;
use crate::process::analysis::conversation::{self, ConvResult, ConvState};
use crate::process::analysis::disclosure::{
    MIN_DISPLAY_PARTICIPANTS, PIN_THRESHOLD, group_label, suppress,
};
use crate::process::analysis::repness::RepfulFor;
use crate::process::analysis::store::{self, Stored};
use crate::process::progress::{self, RESIDENCY_TYPES};

#[derive(Debug, Serialize)]
pub struct RepresentativeOut {
    pub statement_id: String,
    pub body: String,
    pub repful_for: RepfulFor,
    pub n_success: f64,
    pub n_trials: f64,
}

#[derive(Debug, Serialize)]
pub struct GroupView {
    pub id: i64,
    pub label: String,
    pub size: usize,
    pub representative: Vec<RepresentativeOut>,
}

#[derive(Debug, Serialize)]
pub struct GroupAgree {
    pub group_id: i64,
    pub label: String,
    pub agree: u32,
    pub seen: u32,
}

#[derive(Debug, Serialize)]
pub struct ConsensusView {
    pub statement_id: String,
    pub body: String,
    /// group-aware consensus（グループごとの (A+1)/(S+2) の積）
    pub gac: f64,
    pub groups: Vec<GroupAgree>,
}

#[derive(Debug, Serialize)]
pub struct MapPoint {
    pub x: f64,
    pub y: f64,
    pub count: usize,
    pub group_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct StatementPoint {
    pub statement_id: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Serialize)]
pub struct You {
    pub x: f64,
    pub y: f64,
    pub group_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MapView {
    pub base_clusters: Vec<MapPoint>,
    pub statements: Vec<StatementPoint>,
    pub you: Option<You>,
}

/// あるグループの、ある意見への票（Polis の group-votes）
#[derive(Debug, Serialize)]
pub struct GroupVoteCell {
    pub group_id: i64,
    pub agree: u32,
    pub disagree: u32,
    /// 投票した人数（パスを含む）
    pub seen: u32,
}

/// 意見ごとの、グループ別の票（グループの特徴を一覧する表の1行）
#[derive(Debug, Serialize)]
pub struct StatementByGroup {
    pub statement_id: String,
    pub body: String,
    pub groups: Vec<GroupVoteCell>,
}

/// 意見グループの、このまちとの関わり方の構成。人数の少ない区分は null
#[derive(Debug, Serialize)]
pub struct GroupComposition {
    pub group_id: i64,
    pub by_type: BTreeMap<String, Option<i64>>,
}

#[derive(Debug, Serialize)]
pub struct Cell {
    pub agree: i64,
    pub seen: i64,
}

#[derive(Debug, Serialize)]
pub struct ResidencyRow {
    pub statement_id: String,
    pub body: String,
    /// 居住地属性ごとの集計。人数が少ないセルは null
    pub by_type: BTreeMap<String, Option<Cell>>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AnalysisView {
    /// 未投票の最初の意見が残っているので見せない（全体の傾向に投票が左右されないようにするため）
    Locked {
        /// 残りの未投票の最初の意見の数
        remaining: i64,
        total: i64,
    },
    /// まだ計算されていない
    Pending,
    Insufficient {
        participant_count: usize,
        required: usize,
    },
    Ready {
        computed_at: String,
        participant_count: usize,
        analyzed_count: usize,
        groups: Vec<GroupView>,
        consensus: Vec<ConsensusView>,
        /// 承認済みの意見を投稿順に並べた、グループ別の票
        statements_by_group: Vec<StatementByGroup>,
        /// 意見グループごとの、関わり方の構成
        group_composition: Vec<GroupComposition>,
        map: Box<MapView>,
        residency: Vec<ResidencyRow>,
    },
}

/// 承認済みの意見の本文（非公開の意見は分析の表示にも出さない）
async fn approved_bodies(
    pool: &SqlitePool,
    topic_id: &str,
) -> Result<HashMap<String, String>, ApiError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, body FROM statements WHERE topic_id = $1 AND status = 'approved'",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// 承認済みの意見（投稿順）ごとに、各グループの賛成・反対・投票数を並べる。
/// 分析の後に投稿された意見は、分析に含まれていないので出さない
async fn statements_by_group(
    pool: &SqlitePool,
    topic_id: &str,
    result: &ConvResult,
) -> Result<Vec<StatementByGroup>, ApiError> {
    let ordered: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, body FROM statements
         WHERE topic_id = $1 AND status = 'approved'
         ORDER BY created_at, id",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    Ok(ordered
        .into_iter()
        .filter(|(id, _)| result.tids.contains(id))
        .map(|(id, body)| StatementByGroup {
            groups: result
                .groups
                .iter()
                .map(|g| {
                    let c = g.votes[&id];
                    GroupVoteCell {
                        group_id: g.id,
                        agree: c.a,
                        disagree: c.d,
                        seen: c.s,
                    }
                })
                .collect(),
            statement_id: id,
            body,
        })
        .collect())
}

/// ピン留めする合意点: 全グループで (A+1)/(S+2) ≥ 0.8。group-aware consensus の大きい順
fn pinned_consensus(
    result: &ConvResult,
    bodies: &HashMap<String, String>,
    labels: &HashMap<i64, String>,
) -> Vec<ConsensusView> {
    let mut out: Vec<ConsensusView> = result
        .tids
        .iter()
        .filter_map(|tid| {
            let body = bodies.get(tid)?;
            let pinned = result.groups.iter().all(|g| {
                let c = g.votes[tid];
                (f64::from(c.a) + 1.0) / (f64::from(c.s) + 2.0) >= PIN_THRESHOLD
            });
            pinned.then(|| ConsensusView {
                statement_id: tid.clone(),
                body: body.clone(),
                gac: result.group_aware_consensus[tid],
                groups: result
                    .groups
                    .iter()
                    .map(|g| GroupAgree {
                        group_id: g.id,
                        label: labels[&g.id].clone(),
                        agree: g.votes[tid].a,
                        seen: g.votes[tid].s,
                    })
                    .collect(),
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.gac
            .partial_cmp(&a.gac)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// Cookie の参加者の、意見マップ上の位置とグループ
async fn locate_you(
    pool: &SqlitePool,
    user_id: &str,
    result: &ConvResult,
    state: &ConvState,
    bodies: &HashMap<String, String>,
) -> Result<Option<You>, ApiError> {
    let votes: Vec<(String, i64)> =
        sqlx::query_as("SELECT statement_id, vote_value FROM votes WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await?;
    let mine: HashMap<String, i64> = votes.into_iter().collect();
    if !result.tids.iter().any(|t| mine.contains_key(t)) {
        return Ok(None);
    }
    // 分析時の列の並びに合わせる。非公開の意見は分析と同じく 0 にする（zero-out-columns）
    let row: Vec<Option<f64>> = result
        .tids
        .iter()
        .map(|t| {
            if !bodies.contains_key(t) {
                Some(0.0)
            } else {
                mine.get(t).map(|v| -(*v as f64))
            }
        })
        .collect();
    let [x, y] = conversation::project(&result.pca, &row);
    let base_id = state
        .base_clusters
        .iter()
        .find(|c| c.members.iter().any(|m| m == user_id))
        .map(|c| c.id);
    let group_id = base_id.and_then(|b| {
        result
            .groups
            .iter()
            .find(|g| g.base_ids.contains(&b))
            .map(|g| g.id)
    });
    Ok(Some(You { x, y, group_id }))
}

/// 意見グループごとの、このまちとの関わり方の構成（Polis のレポートの「各グループの内訳」に相当）。
/// 人数の少ない区分は二次秘匿で隠す
async fn group_composition(
    pool: &SqlitePool,
    municipality_id: &str,
    result: &ConvResult,
    state: &ConvState,
) -> Result<Vec<GroupComposition>, ApiError> {
    let relations: HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
        "SELECT user_id, residency_type FROM user_municipality_relations WHERE municipality_id = $1",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();
    let members: HashMap<i64, &Vec<String>> = state
        .base_clusters
        .iter()
        .map(|c| (c.id, &c.members))
        .collect();

    Ok(result
        .groups
        .iter()
        .map(|g| {
            let mut counts: HashMap<&str, i64> = HashMap::new();
            for pid in g
                .base_ids
                .iter()
                .filter_map(|b| members.get(b))
                .flat_map(|m| m.iter())
            {
                if let Some(t) = relations.get(pid) {
                    *counts.entry(t.as_str()).or_default() += 1;
                }
            }
            let values: Vec<i64> = RESIDENCY_TYPES
                .iter()
                .map(|t| counts.get(t).copied().unwrap_or(0))
                .collect();
            let visible = suppress(&values);
            GroupComposition {
                group_id: g.id,
                by_type: RESIDENCY_TYPES
                    .iter()
                    .zip(values.iter().zip(&visible))
                    .map(|(t, (n, v))| ((*t).to_owned(), v.then_some(*n)))
                    .collect(),
            }
        })
        .collect())
}

async fn residency_rows(
    pool: &SqlitePool,
    topic_id: &str,
    municipality_id: &str,
    bodies: &HashMap<String, String>,
) -> Result<Vec<ResidencyRow>, ApiError> {
    let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT v.statement_id, r.residency_type,
                sum(CASE WHEN v.vote_value = 1 THEN 1 ELSE 0 END), count(*)
         FROM votes v
         JOIN statements s ON s.id = v.statement_id
         JOIN user_municipality_relations r
           ON r.user_id = v.user_id AND r.municipality_id = $2
         WHERE s.topic_id = $1 AND s.status = 'approved'
         GROUP BY v.statement_id, r.residency_type",
    )
    .bind(topic_id)
    .bind(municipality_id)
    .fetch_all(pool)
    .await?;

    // 意見ごとに、区分ごとの (賛成数, 投票数) を集める
    let mut by_statement: BTreeMap<String, HashMap<String, (i64, i64)>> = BTreeMap::new();
    for (sid, rtype, agree, seen) in rows {
        by_statement
            .entry(sid)
            .or_default()
            .insert(rtype, (agree, seen));
    }
    Ok(by_statement
        .into_iter()
        .filter_map(|(sid, cells)| {
            // 投票数で二次秘匿を判定する
            let seen: Vec<i64> = RESIDENCY_TYPES
                .iter()
                .map(|t| cells.get(*t).map_or(0, |c| c.1))
                .collect();
            let visible = suppress(&seen);
            let by_type: BTreeMap<String, Option<Cell>> = RESIDENCY_TYPES
                .iter()
                .zip(&visible)
                .map(|(t, v)| {
                    let cell = cells.get(*t).filter(|_| *v).map(|(agree, seen)| Cell {
                        agree: *agree,
                        seen: *seen,
                    });
                    ((*t).to_owned(), cell)
                })
                .collect();
            // 表示できるセルが1つもない意見は出さない
            if by_type.values().all(Option::is_none) {
                return None;
            }
            Some(ResidencyRow {
                body: bodies.get(&sid)?.clone(),
                statement_id: sid,
                by_type,
            })
        })
        .collect())
}

/// GET /api/municipalities/{slug}/topics/{topic_id}/analysis
pub async fn topic_analysis(
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
    MaybeParticipant(user_id): MaybeParticipant,
) -> Result<Json<AnalysisView>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    super::ensure_topic(&state.pool, &municipality.id, &topic_id).await?;

    // 最初の意見すべてに投票するまでは見せない（ルールは progress::Progress::analysis_unlocked）
    let p = progress::progress(&state.pool, &topic_id, user_id.as_deref()).await?;
    if !p.analysis_unlocked() {
        return Ok(Json(AnalysisView::Locked {
            remaining: p.analysis_remaining(),
            total: p.total,
        }));
    }

    let stored = store::load(&state.pool, &topic_id).await?;
    let (result, conv_state, computed_at) = match stored {
        None => return Ok(Json(AnalysisView::Pending)),
        Some(Stored::Insufficient { detail, .. }) => {
            return Ok(Json(AnalysisView::Insufficient {
                participant_count: detail.participants,
                required: MIN_DISPLAY_PARTICIPANTS,
            }));
        }
        Some(Stored::Ready {
            result,
            state,
            computed_at,
        }) => (result, state, computed_at),
    };
    if result.n_in_conv < MIN_DISPLAY_PARTICIPANTS {
        return Ok(Json(AnalysisView::Insufficient {
            participant_count: result.n_in_conv,
            required: MIN_DISPLAY_PARTICIPANTS,
        }));
    }

    let bodies = approved_bodies(&state.pool, &topic_id).await?;
    let labels: HashMap<i64, String> = result
        .groups
        .iter()
        .enumerate()
        .map(|(i, g)| (g.id, group_label(i)))
        .collect();

    let groups = result
        .groups
        .iter()
        .map(|g| GroupView {
            id: g.id,
            label: labels[&g.id].clone(),
            size: g.n_members,
            representative: result
                .repness
                .get(&g.id)
                .into_iter()
                .flatten()
                .filter_map(|c| {
                    // best-agree は賛成として表示する（Polis のクライアントが n-agree を使うのと同じ）
                    let (repful_for, n_success) = match c.n_agree {
                        Some(na) if c.best_agree => (RepfulFor::Agree, na),
                        _ => (c.repful_for, c.n_success),
                    };
                    Some(RepresentativeOut {
                        body: bodies.get(&c.tid)?.clone(),
                        statement_id: c.tid.clone(),
                        repful_for,
                        n_success,
                        n_trials: c.n_trials,
                    })
                })
                .collect(),
        })
        .collect();

    let base_group: HashMap<i64, i64> = result
        .groups
        .iter()
        .flat_map(|g| g.base_ids.iter().map(move |b| (*b, g.id)))
        .collect();
    let map = Box::new(MapView {
        base_clusters: result
            .base_clusters
            .iter()
            .map(|c| MapPoint {
                x: c.x,
                y: c.y,
                count: c.count,
                group_id: base_group.get(&c.id).copied(),
            })
            .collect(),
        statements: result
            .tids
            .iter()
            .zip(&result.comment_projection)
            .filter(|(tid, _)| bodies.contains_key(*tid))
            .map(|(tid, [x, y])| StatementPoint {
                statement_id: tid.clone(),
                x: *x,
                y: *y,
            })
            .collect(),
        you: match &user_id {
            Some(uid) => locate_you(&state.pool, uid, &result, &conv_state, &bodies).await?,
            None => None,
        },
    });

    Ok(Json(AnalysisView::Ready {
        computed_at,
        participant_count: result.n_participants,
        analyzed_count: result.n_in_conv,
        consensus: pinned_consensus(&result, &bodies, &labels),
        statements_by_group: statements_by_group(&state.pool, &topic_id, &result).await?,
        group_composition: group_composition(&state.pool, &municipality.id, &result, &conv_state)
            .await?,
        groups,
        map,
        residency: residency_rows(&state.pool, &topic_id, &municipality.id, &bodies).await?,
    }))
}

#[derive(Debug, Serialize)]
pub struct MunicipalityConsensus {
    pub topic_id: String,
    pub topic_title: String,
    pub statement_id: String,
    pub body: String,
    pub gac: f64,
}

/// タイムラインにピン留めする件数の上限（画面では最初の数件だけを出し、残りは「もっと見る」）
const MAX_PINNED: usize = 20;

/// GET /api/municipalities/{slug}/consensus — 自治体内の合意点（ピン留め用）。
/// 分析と同じく、その参加者がすべての意見に投票したトピックの合意点だけを返す
pub async fn municipality_consensus(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    MaybeParticipant(user_id): MaybeParticipant,
) -> Result<Json<Vec<MunicipalityConsensus>>, ApiError> {
    let Some(user_id) = user_id else {
        return Ok(Json(Vec::new()));
    };
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    let topics: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, title FROM topics WHERE municipality_id = $1 AND status = 'published'",
    )
    .bind(&municipality.id)
    .fetch_all(&state.pool)
    .await?;

    let progress =
        progress::progress_by_topic(&state.pool, &municipality.id, Some(&user_id)).await?;

    let mut out = Vec::new();
    for (topic_id, title) in topics {
        let Some(p) = progress.get(&topic_id) else {
            continue;
        };
        // 投票していないトピックは出さない（最初の意見が無いトピックは、投票しなくても分析が開くため）
        if p.voted == 0 || !p.analysis_unlocked() {
            continue;
        }
        let Some(Stored::Ready { result, .. }) = store::load(&state.pool, &topic_id).await? else {
            continue;
        };
        if result.n_in_conv < MIN_DISPLAY_PARTICIPANTS {
            continue;
        }
        let bodies = approved_bodies(&state.pool, &topic_id).await?;
        let labels: HashMap<i64, String> = result
            .groups
            .iter()
            .map(|g| (g.id, String::new()))
            .collect();
        for c in pinned_consensus(&result, &bodies, &labels) {
            out.push(MunicipalityConsensus {
                topic_id: topic_id.clone(),
                topic_title: title.clone(),
                statement_id: c.statement_id,
                body: c.body,
                gac: c.gac,
            });
        }
    }
    out.sort_by(|a, b| {
        b.gac
            .partial_cmp(&a.gac)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(MAX_PINNED);
    Ok(Json(out))
}
