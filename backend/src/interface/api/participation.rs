// SPDX-License-Identifier: AGPL-3.0-only

//! 投票・意見投稿・居住地属性。
//! 意見のレスポンス型には投稿者のフィールドを持たせない（匿名性の担保）。

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::interface::api::AppState;
use crate::interface::api::error::{ApiError, ApiJson};
use crate::interface::api::municipalities;
use crate::interface::api::participant::{MaybeParticipant, Participant};
use crate::process::analysis::store::{self, Stored};
use crate::process::moderation::{self, Verdict};
use crate::process::progress::{self, PostPermission, Progress, RESIDENCY_TYPES, has_relation};
use crate::shared::limits::MAX_STATEMENT_CHARS;
use crate::shared::util::{new_id, now_iso};

// ===== 居住地属性（このまちとの関わり方） =====

/// 地区は受け取らない。市民を地区で分けると分断を煽るため、集めない（ADR 0002）。
/// 古いクライアントが `district_id` を送ってきても、serde の既定の挙動で無視される
#[derive(Debug, Deserialize)]
pub struct RelationInput {
    pub residency_type: String,
}

#[derive(Debug, Serialize)]
pub struct Relation {
    pub residency_type: String,
}

/// PUT /api/municipalities/{slug}/relation — その自治体との関わり方を登録・更新する
pub async fn put_relation(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    MaybeParticipant(existing): MaybeParticipant,
    ApiJson(input): ApiJson<RelationInput>,
) -> Result<(Participant, Json<Relation>), ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;

    if !RESIDENCY_TYPES.contains(&input.residency_type.as_str()) {
        return Err(ApiError::validation(
            "invalid_residency_type",
            "residency_type は resident / neighbor / commuter / related のいずれかです",
        ));
    }

    let participant = Participant::ensure(&state, existing).await?;
    sqlx::query(
        "INSERT INTO user_municipality_relations
            (user_id, municipality_id, residency_type, district_id, created_at)
         VALUES ($1, $2, $3, NULL, $4)
         ON CONFLICT (user_id, municipality_id) DO UPDATE
            SET residency_type = excluded.residency_type, district_id = NULL",
    )
    .bind(&participant.user_id)
    .bind(&municipality.id)
    .bind(&input.residency_type)
    .bind(now_iso())
    .execute(&state.pool)
    .await?;

    Ok((
        participant,
        Json(Relation {
            residency_type: input.residency_type,
        }),
    ))
}

// ===== 次の意見 =====

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StatementCard {
    pub id: String,
    pub body: String,
    /// 最初の意見か（先に出すため。API では返さない）
    #[serde(skip)]
    pub is_seed: bool,
}

#[derive(Debug, Serialize)]
pub struct NextStatement {
    /// まだ投票していない承認済みの意見。全部投票し終えたら null
    pub statement: Option<StatementCard>,
    pub progress: Progress,
    /// この自治体の居住地属性が未登録なら true（投票前に聞く）
    pub needs_relation: bool,
    /// 意見を投稿できるか（ルールは `progress::post_permission`。投票のたびに変わるので、ここで返す）
    pub posting: PostPermission,
}

/// GET /api/municipalities/{slug}/topics/{topic_id}/next-statement
pub async fn next_statement(
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
    MaybeParticipant(user_id): MaybeParticipant,
) -> Result<Json<NextStatement>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    super::ensure_topic(&state.pool, &municipality.id, &topic_id).await?;
    let progress = progress::progress(&state.pool, &topic_id, user_id.as_deref()).await?;
    let closed = progress.closed;

    // Cookie の無い人は「まだ何も投票していない人」として扱う（空文字の user_id は存在しない）
    let user_id = user_id.unwrap_or_default();

    // 候補: 承認済みで、まだ投票していない意見
    let candidates: Vec<StatementCard> = sqlx::query_as(
        "SELECT s.id, s.body, s.is_seed FROM statements s
         WHERE s.topic_id = $1 AND s.status = 'approved'
           AND NOT EXISTS (
               SELECT 1 FROM votes v WHERE v.statement_id = s.id AND v.user_id = $2)
         ORDER BY s.id",
    )
    .bind(&topic_id)
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await?;

    // 投票していない最初の意見があれば、その中から選ぶ。分析を見られる条件（最初の意見すべて）に
    // 先に届くようにするため（Polis 本家は分けない。docs/polis-math.md）
    let candidates = if candidates.iter().any(|c| c.is_seed) {
        candidates.into_iter().filter(|c| c.is_seed).collect()
    } else {
        candidates
    };

    // Polis の comment-priorities で重みを付けて1件選ぶ（分析が無ければ一様）
    let priorities = match store::load(&state.pool, &topic_id).await? {
        Some(Stored::Ready { result, .. }) => result.comment_priorities,
        _ => Default::default(),
    };
    let weights: Vec<f64> = candidates
        .iter()
        .map(|c| priorities.get(&c.id).copied().unwrap_or(0.0))
        .collect();
    // 締め切ったトピックは、投票の対象を出さない（結果は読める）
    let statement = select_probabilistically(&weights, rand::random::<f64>())
        .filter(|_| !closed)
        .map(|i| candidates.into_iter().nth(i).expect("範囲内"));

    let needs_relation = !has_relation(&state.pool, &user_id, &municipality.id).await?;
    let posting = progress::post_permission(&state.pool, &topic_id, Some(&user_id)).await?;

    Ok(Json(NextStatement {
        statement,
        progress,
        needs_relation,
        posting,
    }))
}

/// 重み付きの抽選（Polis `server/src/nextComment.ts` の `selectProbabilistically` の移植。
/// Copyright (C) 2012-present, The Authors (Polis), AGPL-3.0）。
/// 重みが無い・0 の候補は 1 として扱う（`priorities[tid] || 1`）。`r` は [0, 1) の乱数
pub fn select_probabilistically(weights: &[f64], r: f64) -> Option<usize> {
    let mut cumulative = Vec::with_capacity(weights.len());
    let mut total = 0.0;
    for w in weights {
        total += if *w > 0.0 && w.is_finite() { *w } else { 1.0 };
        cumulative.push(total);
    }
    let target = r * total;
    cumulative.iter().position(|c| *c > target)
}

/// 締め切ったトピックへの投票・投稿（ADR 0005。結果は読める）
fn topic_closed() -> ApiError {
    ApiError::conflict(
        "topic_closed",
        "このトピックは締め切りました。結果は引き続き見られます",
    )
}

// ===== 投票 =====

#[derive(Debug, Deserialize)]
pub struct VoteInput {
    pub value: i64,
}

#[derive(Debug, Serialize)]
pub struct Vote {
    pub statement_id: String,
    pub value: i64,
}

/// POST /api/municipalities/{slug}/topics/{topic_id}/statements/{statement_id}/votes
/// 同じ意見への再投票はできない（409 `already_voted`）。締め切ったトピックには投票できない（409 `topic_closed`）
pub async fn vote(
    State(state): State<AppState>,
    Path((slug, topic_id, statement_id)): Path<(String, String, String)>,
    MaybeParticipant(user_id): MaybeParticipant,
    ApiJson(input): ApiJson<VoteInput>,
) -> Result<Json<Vote>, ApiError> {
    if ![-1, 0, 1].contains(&input.value) {
        return Err(ApiError::validation(
            "invalid_vote_value",
            "value は 1（賛成）/ -1（反対）/ 0（パス）のいずれかです",
        ));
    }
    let municipality = municipalities::resolve(&state.pool, &slug).await?;

    // その自治体のそのトピックの、承認済みの意見であること
    super::ensure_topic(&state.pool, &municipality.id, &topic_id).await?;
    if progress::topic_closed(&state.pool, &topic_id).await? {
        return Err(topic_closed());
    }
    let found: Option<String> = sqlx::query_scalar(
        "SELECT id FROM statements WHERE id = $1 AND topic_id = $2 AND status = 'approved'",
    )
    .bind(&statement_id)
    .bind(&topic_id)
    .fetch_optional(&state.pool)
    .await?;
    if found.is_none() {
        return Err(ApiError::NotFound);
    }

    // 投票には居住地属性の登録が必要。登録時に参加者が作られるので、ここでは作らない
    let relation_required = || {
        ApiError::conflict(
            "relation_required",
            "投票の前に、この自治体との関係を登録してください",
        )
    };
    let user_id = user_id.ok_or_else(relation_required)?;
    if !has_relation(&state.pool, &user_id, &municipality.id).await? {
        return Err(relation_required());
    }

    // 票はあとから変えられない（1人1票。分析を見てから票を合わせ直すことを防ぐ。Polis の画面も戻れない）
    let inserted = sqlx::query(
        "INSERT INTO votes (id, user_id, statement_id, vote_value, created_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id, statement_id) DO NOTHING",
    )
    .bind(new_id())
    .bind(&user_id)
    .bind(&statement_id)
    .bind(input.value)
    .bind(now_iso())
    .execute(&state.pool)
    .await?
    .rows_affected();
    if inserted == 0 {
        return Err(ApiError::conflict(
            "already_voted",
            "この意見には、もう投票しています",
        ));
    }

    Ok(Json(Vote {
        statement_id,
        value: input.value,
    }))
}

// ===== 意見の投稿 =====

#[derive(Debug, Deserialize)]
pub struct StatementInput {
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct PostedStatement {
    /// 保存した意見の ID。重複（`duplicate`）のときは保存しないので無い
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// approved / rejected / flagged / pending、または duplicate（同じ意見がすでにある。ADR 0005）
    pub status: &'static str,
    /// 非公開の理由の種類（harm / person / official_name / not_votable。ADR 0007）。理由の文は返さない
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection: Option<&'static str>,
    /// 非公開のときの書き直しの提案
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// 重複のとき、案内する既存の意見（その場で投票してもらう）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similar: Option<SimilarStatement>,
}

#[derive(Debug, Serialize)]
pub struct SimilarStatement {
    pub id: String,
    pub body: String,
    /// この参加者の票（1 / -1 / 0）。まだ投票していなければ無い（画面は投票のボタンを出す）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub your_vote: Option<i64>,
}

/// 前後の空白を除き、1〜140 文字であることを確かめる
pub fn validate_statement_body(body: &str) -> Result<&str, ApiError> {
    let body = body.trim();
    let chars = body.chars().count();
    if chars == 0 {
        return Err(ApiError::validation(
            "invalid_body",
            "意見を入力してください",
        ));
    }
    if chars > MAX_STATEMENT_CHARS {
        return Err(ApiError::validation(
            "invalid_body",
            format!("意見は {MAX_STATEMENT_CHARS} 文字以内です（現在 {chars} 文字）"),
        ));
    }
    Ok(body)
}

/// POST /api/municipalities/{slug}/topics/{topic_id}/statements
pub async fn post_statement(
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
    MaybeParticipant(existing): MaybeParticipant,
    ApiJson(input): ApiJson<StatementInput>,
) -> Result<(StatusCode, Participant, Json<PostedStatement>), ApiError> {
    let body = validate_statement_body(&input.body)?;
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    super::ensure_topic(&state.pool, &municipality.id, &topic_id).await?;

    // 投稿にはパスキーの登録が要り（ADR 0006。投票は匿名のままでよい）、
    // 分析を見られるまで投票してから、1トピック1件まで（ADR 0005。ルールは progress::post_permission）
    match progress::post_permission(&state.pool, &topic_id, existing.as_deref()).await? {
        PostPermission::Allowed => {}
        PostPermission::Closed => return Err(topic_closed()),
        PostPermission::RegistrationRequired => {
            return Err(ApiError::unauthorized(
                "registration_required",
                "意見を投稿するには、パスキーの登録が要ります",
            ));
        }
        PostPermission::VotesRequired { remaining } => {
            return Err(ApiError::forbidden(
                "votes_required",
                format!("最初の意見にあと{remaining}件投票すると、意見を投稿できます"),
            ));
        }
        PostPermission::LimitReached => {
            return Err(ApiError::conflict(
                "statement_limit",
                "このトピックに投稿できる意見の数に達しました",
            ));
        }
    }
    let participant = Participant::ensure(&state, existing).await?;

    // いったん非公開（pending）で保存してから判定する
    let id = new_id();
    sqlx::query(
        "INSERT INTO statements (id, topic_id, user_id, body, status, created_at)
         VALUES ($1, $2, $3, $4, 'pending', $5)",
    )
    .bind(&id)
    .bind(&topic_id)
    .bind(&participant.user_id)
    .bind(body)
    .bind(now_iso())
    .execute(&state.pool)
    .await?;

    let context = moderation::context_for_topic(&state.pool, &topic_id, body, Some(&id)).await?;
    let review = state.moderation.review(&state.pool, body, &context).await;

    // 同じ意見がすでにあれば、保存せず（上限にも数えない）、その意見への投票を案内する（ADR 0005）
    if let Verdict::Duplicate { statement_id } = &review.verdict {
        sqlx::query("DELETE FROM statements WHERE id = $1")
            .bind(&id)
            .execute(&state.pool)
            .await?;
        let body = context
            .statements
            .iter()
            .find(|(sid, _)| sid == statement_id)
            .map(|(_, b)| b.clone())
            .unwrap_or_default();
        let your_vote: Option<i64> = sqlx::query_scalar(
            "SELECT vote_value FROM votes WHERE user_id = $1 AND statement_id = $2",
        )
        .bind(&participant.user_id)
        .bind(statement_id)
        .fetch_optional(&state.pool)
        .await?;
        return Ok((
            StatusCode::OK,
            participant,
            Json(PostedStatement {
                id: None,
                status: "duplicate",
                rejection: None,
                suggestion: None,
                similar: Some(SimilarStatement {
                    id: statement_id.clone(),
                    body,
                    your_vote,
                }),
            }),
        ));
    }

    moderation::save(&state.pool, &id, &review).await?;
    // 書いた人は自分の意見に賛成したものとする（Polis と同じ。自分の意見が次の意見に出ず、立場が意見マップに入る）
    if !matches!(review.verdict, Verdict::Rejected { .. }) {
        sqlx::query(
            "INSERT INTO votes (id, user_id, statement_id, vote_value, created_at)
             VALUES ($1, $2, $3, 1, $4)",
        )
        .bind(new_id())
        .bind(&participant.user_id)
        .bind(&id)
        .bind(now_iso())
        .execute(&state.pool)
        .await?;
    }
    let (rejection, suggestion) = match &review.verdict {
        Verdict::Rejected { reason, suggestion } => (Some(reason.as_str()), suggestion.clone()),
        _ => (None, None),
    };
    Ok((
        StatusCode::CREATED,
        participant,
        Json(PostedStatement {
            id: Some(id),
            status: review.verdict.status().as_str(),
            rejection,
            suggestion,
            similar: None,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::select_probabilistically;

    #[test]
    fn picks_by_cumulative_weight() {
        // 累積 [1, 4, 5]。r * 5 を最初に超える位置
        let w = [1.0, 3.0, 1.0];
        assert_eq!(select_probabilistically(&w, 0.0), Some(0));
        assert_eq!(select_probabilistically(&w, 0.19), Some(0));
        assert_eq!(select_probabilistically(&w, 0.2), Some(1));
        assert_eq!(select_probabilistically(&w, 0.79), Some(1));
        assert_eq!(select_probabilistically(&w, 0.8), Some(2));
        assert_eq!(select_probabilistically(&w, 0.999), Some(2));
    }

    #[test]
    fn missing_or_zero_weights_count_as_one() {
        // [0, 0] → [1, 1] として扱う
        assert_eq!(select_probabilistically(&[0.0, 0.0], 0.4), Some(0));
        assert_eq!(select_probabilistically(&[0.0, 0.0], 0.6), Some(1));
    }

    #[test]
    fn no_candidates() {
        assert_eq!(select_probabilistically(&[], 0.5), None);
    }
}
