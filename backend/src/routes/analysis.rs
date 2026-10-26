// SPDX-License-Identifier: AGPL-3.0-only

//! 分析結果の API。参加者 ID は返さない（基底クラスタは中心と人数だけ）。

use std::collections::{BTreeMap, HashMap};

use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::AppState;
use crate::analysis::conversation::{self, ConvResult, ConvState};
use crate::analysis::repness::RepfulFor;
use crate::analysis::store::{self, Stored};
use crate::error::ApiError;
use crate::participant::MaybeParticipant;
use crate::routes::{municipalities, topics};

/// グループを表示する最小の参加者数（分析に含まれる人数）。
/// 少人数ではグループの集計が個人の投票そのものになるため（Polis にはない、このプロジェクトの判断）
pub const MIN_DISPLAY_PARTICIPANTS: usize = 5;
/// 居住地属性ごとの集計で、これ未満の人数のセルは出さない
pub const MIN_CELL_SIZE: i64 = 5;
/// タイムラインにピン留めする合意点の条件: 全グループで (A+1)/(S+2) がこの値以上
pub const PIN_THRESHOLD: f64 = 0.8;

const RESIDENCY_TYPES: [&str; 4] = ["resident", "neighbor", "commuter", "related"];

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
        map: MapView,
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

fn group_label(index: usize) -> String {
    char::from(b'A' + (index % 26) as u8).to_string()
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

/// 居住地属性ごとの、意見ごとの賛成数と投票数（人数が少ないセルは null）
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

    let mut by_statement: BTreeMap<String, BTreeMap<String, Option<Cell>>> = BTreeMap::new();
    for (sid, rtype, agree, seen) in rows {
        let cells = by_statement.entry(sid).or_insert_with(|| {
            RESIDENCY_TYPES
                .iter()
                .map(|t| ((*t).to_owned(), None))
                .collect()
        });
        if seen >= MIN_CELL_SIZE {
            cells.insert(rtype, Some(Cell { agree, seen }));
        }
    }
    Ok(by_statement
        .into_iter()
        .filter_map(|(sid, by_type)| {
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
    topics::ensure_in_municipality(&state.pool, &municipality.id, &topic_id).await?;

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
                    Some(RepresentativeOut {
                        body: bodies.get(&c.tid)?.clone(),
                        statement_id: c.tid.clone(),
                        repful_for: c.repful_for,
                        n_success: c.n_success,
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
    let map = MapView {
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
    };

    Ok(Json(AnalysisView::Ready {
        computed_at,
        participant_count: result.n_participants,
        analyzed_count: result.n_in_conv,
        consensus: pinned_consensus(&result, &bodies, &labels),
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

/// タイムラインにピン留めする件数の上限
const MAX_PINNED: usize = 5;

/// GET /api/municipalities/{slug}/consensus — 自治体内の全トピックの合意点（ピン留め用）
pub async fn municipality_consensus(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<MunicipalityConsensus>>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    let topics: Vec<(String, String)> =
        sqlx::query_as("SELECT id, title FROM topics WHERE municipality_id = $1")
            .bind(&municipality.id)
            .fetch_all(&state.pool)
            .await?;

    let mut out = Vec::new();
    for (topic_id, title) in topics {
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
