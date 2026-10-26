// SPDX-License-Identifier: AGPL-3.0-or-later

//! 投票・意見投稿・居住地属性。
//! 意見のレスポンス型には投稿者のフィールドを持たせない（匿名性の担保）。

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::AppState;
use crate::error::{ApiError, ApiJson};
use crate::moderation::Verdict;
use crate::participant::{MaybeParticipant, Participant};
use crate::routes::{municipalities, topics};
use crate::util::{new_id, now_iso};

/// 意見の最大文字数（DB の CHECK と同じ。文字数は Unicode のコードポイント単位）
pub const MAX_STATEMENT_CHARS: usize = 140;

const RESIDENCY_TYPES: [&str; 4] = ["resident", "neighbor", "commuter", "related"];

async fn has_relation(
    pool: &SqlitePool,
    user_id: &str,
    municipality_id: &str,
) -> Result<bool, ApiError> {
    let found: Option<String> = sqlx::query_scalar(
        "SELECT user_id FROM user_municipality_relations WHERE user_id = $1 AND municipality_id = $2",
    )
    .bind(user_id)
    .bind(municipality_id)
    .fetch_optional(pool)
    .await?;
    Ok(found.is_some())
}

// ===== 居住地属性 =====

#[derive(Debug, Deserialize)]
pub struct RelationInput {
    pub residency_type: String,
    pub district_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Relation {
    pub residency_type: String,
    pub district_id: Option<String>,
}

/// PUT /api/municipalities/{slug}/relation — その自治体との関係を登録・更新する
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
    if let Some(district_id) = &input.district_id {
        let found: Option<String> =
            sqlx::query_scalar("SELECT id FROM districts WHERE id = $1 AND municipality_id = $2")
                .bind(district_id)
                .bind(&municipality.id)
                .fetch_optional(&state.pool)
                .await?;
        if found.is_none() {
            return Err(ApiError::validation(
                "invalid_district",
                "この自治体の地区ではありません",
            ));
        }
    }

    let participant = Participant::ensure(&state, existing).await?;
    sqlx::query(
        "INSERT INTO user_municipality_relations
            (user_id, municipality_id, residency_type, district_id, created_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id, municipality_id) DO UPDATE
            SET residency_type = excluded.residency_type, district_id = excluded.district_id",
    )
    .bind(&participant.user_id)
    .bind(&municipality.id)
    .bind(&input.residency_type)
    .bind(&input.district_id)
    .bind(now_iso())
    .execute(&state.pool)
    .await?;

    Ok((
        participant,
        Json(Relation {
            residency_type: input.residency_type,
            district_id: input.district_id,
        }),
    ))
}

// ===== 次の意見 =====

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StatementCard {
    pub id: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct Progress {
    pub voted: i64,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct NextStatement {
    /// まだ投票していない承認済みの意見。全部投票し終えたら null
    pub statement: Option<StatementCard>,
    pub progress: Progress,
    /// この自治体の居住地属性が未登録なら true（投票前に聞く）
    pub needs_relation: bool,
}

/// GET /api/municipalities/{slug}/topics/{topic_id}/next-statement
pub async fn next_statement(
    State(state): State<AppState>,
    Path((slug, topic_id)): Path<(String, String)>,
    MaybeParticipant(user_id): MaybeParticipant,
) -> Result<Json<NextStatement>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    topics::ensure_in_municipality(&state.pool, &municipality.id, &topic_id).await?;

    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM statements WHERE topic_id = $1 AND status = 'approved'",
    )
    .bind(&topic_id)
    .fetch_one(&state.pool)
    .await?;

    // Cookie の無い人は「まだ何も投票していない人」として扱う（空文字の user_id は存在しない）
    let user_id = user_id.unwrap_or_default();

    let voted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM votes v
         JOIN statements s ON s.id = v.statement_id
         WHERE s.topic_id = $1 AND s.status = 'approved' AND v.user_id = $2",
    )
    .bind(&topic_id)
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await?;

    let statement = sqlx::query_as(
        "SELECT s.id, s.body FROM statements s
         WHERE s.topic_id = $1 AND s.status = 'approved'
           AND NOT EXISTS (
               SELECT 1 FROM votes v WHERE v.statement_id = s.id AND v.user_id = $2)
         ORDER BY random()
         LIMIT 1",
    )
    .bind(&topic_id)
    .bind(&user_id)
    .fetch_optional(&state.pool)
    .await?;

    let needs_relation = !has_relation(&state.pool, &user_id, &municipality.id).await?;

    Ok(Json(NextStatement {
        statement,
        progress: Progress { voted, total },
        needs_relation,
    }))
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
/// 同じ意見への再投票は、最後の判断で上書きする。
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
    let found: Option<String> = sqlx::query_scalar(
        "SELECT s.id FROM statements s
         JOIN topics t ON t.id = s.topic_id
         WHERE s.id = $1 AND s.topic_id = $2 AND t.municipality_id = $3 AND s.status = 'approved'",
    )
    .bind(&statement_id)
    .bind(&topic_id)
    .bind(&municipality.id)
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

    sqlx::query(
        "INSERT INTO votes (id, user_id, statement_id, vote_value, created_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id, statement_id) DO UPDATE
            SET vote_value = excluded.vote_value, created_at = excluded.created_at",
    )
    .bind(new_id())
    .bind(&user_id)
    .bind(&statement_id)
    .bind(input.value)
    .bind(now_iso())
    .execute(&state.pool)
    .await?;

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
    pub id: String,
    pub status: &'static str,
    /// 却下されたときの書き直しの提案（フェーズ4の AI モデレーションで使う）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
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
    topics::ensure_in_municipality(&state.pool, &municipality.id, &topic_id).await?;

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

    let verdict = state.moderation.review(body).await;
    sqlx::query("UPDATE statements SET status = $1 WHERE id = $2")
        .bind(verdict.status())
        .bind(&id)
        .execute(&state.pool)
        .await?;

    let suggestion = match &verdict {
        Verdict::Rejected { suggestion } => suggestion.clone(),
        _ => None,
    };
    Ok((
        StatusCode::CREATED,
        participant,
        Json(PostedStatement {
            id,
            status: verdict.status(),
            suggestion,
        }),
    ))
}
