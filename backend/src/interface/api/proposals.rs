// SPDX-License-Identifier: AGPL-3.0-only

//! 市民からのトピックの提案（ADR 0005）。提案は下書きになり、運営者が確かめてから公開する。
//! 提案した人は、本人の「自分の提案」の一覧のほかには返さない（匿名性。運営者の API でも）

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::interface::api::AppState;
use crate::interface::api::error::{ApiError, ApiJson};
use crate::interface::api::municipalities;
use crate::interface::api::participant::MaybeParticipant;
use crate::process::moderation;
use crate::process::progress::{self, ProposePermission};
use crate::records::proposals::{self, MyProposal};
use crate::shared::limits::{MAX_DESCRIPTION_CHARS, MAX_STATEMENT_CHARS, MAX_TITLE_CHARS};

const MAX_PROPOSAL_STATEMENTS: usize = progress::PROPOSAL_SEEDS;

#[derive(Debug, Deserialize)]
pub struct ProposalInput {
    pub title: String,
    pub description: String,
    pub statements: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Proposal {
    /// トピックの ID（公開されたら、この ID でトピックを開ける）
    pub id: String,
    pub title: String,
    pub description: String,
    /// reviewing（確認中）/ published（公開）/ declined（見送り）/ withdrawn（公開のあとに取り下げ）
    pub state: &'static str,
    pub created_at: String,
}

impl From<MyProposal> for Proposal {
    fn from(p: MyProposal) -> Self {
        let state = match p.status.as_str() {
            "draft" => "reviewing",
            "published" => "published",
            _ if p.published_at.is_some() => "withdrawn",
            _ => "declined",
        };
        Self {
            id: p.id,
            title: p.title,
            description: p.description,
            state,
            created_at: p.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Proposals {
    /// 提案できるか（ルールは `progress::propose_permission`）
    pub permission: ProposePermission,
    /// 本人の提案（新しい順）。Cookie が無ければ空
    pub items: Vec<Proposal>,
}

/// GET /api/municipalities/{slug}/proposals — 本人の提案と、提案できるか
pub async fn mine(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    MaybeParticipant(user_id): MaybeParticipant,
) -> Result<Json<Proposals>, ApiError> {
    let municipality = municipalities::resolve(&state.pool, &slug).await?;
    let permission =
        progress::propose_permission(&state.pool, &municipality.id, user_id.as_deref()).await?;
    let items = match &user_id {
        Some(user_id) => proposals::mine(&state.pool, &municipality.id, user_id)
            .await?
            .into_iter()
            .map(Proposal::from)
            .collect(),
        None => Vec::new(),
    };
    Ok(Json(Proposals { permission, items }))
}

/// 前後の空白を除き、1〜`max` 文字であることを確かめる
fn check_length<'a>(text: &'a str, what: &str, max: usize) -> Result<&'a str, ApiError> {
    let text = text.trim();
    let chars = text.chars().count();
    if chars == 0 {
        return Err(ApiError::validation(
            "invalid_proposal",
            format!("{what}を入力してください"),
        ));
    }
    if chars > max {
        return Err(ApiError::validation(
            "invalid_proposal",
            format!("{what}は{max}文字以内です（現在{chars}文字）"),
        ));
    }
    Ok(text)
}

/// POST /api/municipalities/{slug}/proposals — トピックを提案する（下書きとして保存）
pub async fn create(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    MaybeParticipant(user_id): MaybeParticipant,
    ApiJson(input): ApiJson<ProposalInput>,
) -> Result<(StatusCode, Json<Proposal>), ApiError> {
    let title = check_length(&input.title, "タイトル", MAX_TITLE_CHARS)?;
    let description = check_length(&input.description, "説明", MAX_DESCRIPTION_CHARS)?;
    let statements: Vec<&str> = input
        .statements
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if statements.is_empty() || statements.len() > MAX_PROPOSAL_STATEMENTS {
        return Err(ApiError::validation(
            "invalid_proposal",
            format!("意見は1〜{MAX_PROPOSAL_STATEMENTS}件にしてください"),
        ));
    }
    for body in &statements {
        check_length(body, "意見", MAX_STATEMENT_CHARS)?;
    }
    let municipality = municipalities::resolve(&state.pool, &slug).await?;

    // パスキーの登録・関わり方の登録・自治体ごとに週1件（ADR 0005。ルールは progress::propose_permission）
    match progress::propose_permission(&state.pool, &municipality.id, user_id.as_deref()).await? {
        ProposePermission::Allowed => {}
        ProposePermission::RegistrationRequired => {
            return Err(ApiError::unauthorized(
                "registration_required",
                "トピックを提案するには、パスキーの登録が要ります",
            ));
        }
        ProposePermission::RelationRequired => {
            return Err(ApiError::conflict(
                "relation_required",
                "提案の前に、この自治体との関係を登録してください",
            ));
        }
        ProposePermission::WeeklyLimit { .. } => {
            return Err(ApiError::conflict(
                "proposal_limit",
                "この自治体には、この1週間にもう提案しています（1週間に1件まで）",
            ));
        }
    }
    let user_id = user_id.expect("提案できるなら参加者がいる");

    // 公人の氏名は出さない（ADR 0004・0007）。AI は呼ばない（運営者が全件を確かめる）
    let officials = moderation::official_names(&state.pool, &municipality.id).await?;
    let fields = [("タイトル", title), ("説明", description)]
        .into_iter()
        .chain(statements.iter().map(|s| ("意見", *s)));
    for (what, text) in fields {
        if let Some(name) = moderation::official_name_in(text, &officials) {
            return Err(ApiError::validation(
                "official_name",
                format!(
                    "{what}に公人の氏名（{name}）が入っています。人ではなく、政策や事業について書いてください"
                ),
            ));
        }
    }

    let id = proposals::create(
        &state.pool,
        &municipality.id,
        &user_id,
        title,
        description,
        &statements,
    )
    .await?;
    let created = proposals::mine(&state.pool, &municipality.id, &user_id)
        .await?
        .into_iter()
        .find(|p| p.id == id)
        .expect("保存した提案がある");
    Ok((StatusCode::CREATED, Json(created.into())))
}
