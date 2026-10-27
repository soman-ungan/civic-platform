// SPDX-License-Identifier: AGPL-3.0-only

//! 会議録の手動取り込み（docs/plans/2026-10-27-phase5a-minutes.md）。
//!
//! 運営者が手でコピーした会議録（`data/private/` に置く。リポジトリに入れない）を読み、
//! 発言者を名簿と対応付け、AI にトピック案を作らせ、確かめてから下書きとして保存する。
//! DB には本文を残さない（出典の情報と短い引用だけ）。

pub mod extract;
pub mod parse;

use std::collections::{BTreeSet, HashMap};

use regex::Regex;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::llm::{Effort, Llm, Purpose, Request};
use crate::moderation::{Moderation, Verdict};
use crate::util::{new_id, now_iso};
use parse::{Patterns, Utterance, normalize_name};

/// 一度に取り込める文字数の上限（これを超えたら分けて取り込む）
pub const MAX_CHARS: usize = 300_000;

/// 取り込む会議録
#[derive(Debug, Clone)]
pub struct Input {
    pub municipality_id: String,
    /// 会議名（例: 令和7年第3回定例会 9月5日）
    pub title: String,
    /// 開催日 YYYY-MM-DD
    pub held_on: String,
    /// 会議録のページの URL
    pub url: Option<String>,
    pub text: String,
}

/// 読み取りと名簿との対応付けの結果（AI は呼ばない）
#[derive(Debug)]
pub struct Prepared {
    pub sha256: String,
    pub utterances: Vec<Utterance>,
    /// 発言の番号 → 公人の ID
    pub officials: HashMap<usize, String>,
    /// 名簿に無い首長・議員（「12番（鈴木花子）」の形）
    pub unmatched: Vec<String>,
    /// 出してはいけない氏名（名簿の全員と、会議録に出てきた全員）
    pub names: Vec<String>,
    /// 既に取り込んだ同じ文書の ID
    pub existing_document: Option<String>,
}

/// 自治体の会議録の読み取りの形。`collector_sources.config` の `speaker_pattern` /
/// `skip_pattern` があればそれを使う
pub async fn patterns(pool: &SqlitePool, municipality_id: &str) -> anyhow::Result<Patterns> {
    let config: Option<String> = sqlx::query_scalar(
        "SELECT config FROM collector_sources
         WHERE municipality_id = $1 AND kind = 'minutes' AND adapter = 'manual_upload'",
    )
    .bind(municipality_id)
    .fetch_optional(pool)
    .await?;
    let config: serde_json::Value = config
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();
    let mut patterns = Patterns::default();
    if let Some(p) = config["speaker_pattern"].as_str() {
        patterns.speaker = Regex::new(p)?;
    }
    if let Some(p) = config["skip_pattern"].as_str() {
        patterns.skip = Regex::new(p)?;
    }
    Ok(patterns)
}

pub async fn prepare(pool: &SqlitePool, input: &Input) -> anyhow::Result<Prepared> {
    let patterns = patterns(pool, &input.municipality_id).await?;
    let utterances = parse::parse(&input.text, &patterns);
    let sha256: String = Sha256::digest(input.text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    // 開催日に任期中の公人
    let roster: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, name, role FROM public_officials
         WHERE municipality_id = $1 AND term_start <= $2 AND (term_end IS NULL OR term_end >= $2)",
    )
    .bind(&input.municipality_id)
    .bind(&input.held_on)
    .fetch_all(pool)
    .await?;
    let all_names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM public_officials WHERE municipality_id = $1")
            .bind(&input.municipality_id)
            .fetch_all(pool)
            .await?;

    let mut officials = HashMap::new();
    let mut unmatched = BTreeSet::new();
    for u in &utterances {
        let Some(role) = u.kind.official_role() else {
            continue;
        };
        let found = roster
            .iter()
            .find(|(_, name, r)| r == role && normalize_name(name) == u.name);
        match found {
            Some((id, _, _)) => {
                officials.insert(u.no, id.clone());
            }
            None => {
                unmatched.insert(format!("{}（{}）", u.role, u.name));
            }
        }
    }

    let mut names: BTreeSet<String> = all_names.iter().map(|n| normalize_name(n)).collect();
    names.extend(utterances.iter().map(|u| u.name.clone()));
    names.retain(|n| !n.is_empty());

    let existing_document =
        sqlx::query_scalar("SELECT id FROM documents WHERE municipality_id = $1 AND sha256 = $2")
            .bind(&input.municipality_id)
            .bind(&sha256)
            .fetch_optional(pool)
            .await?;

    Ok(Prepared {
        sha256,
        utterances,
        officials,
        unmatched: unmatched.into_iter().collect(),
        names: names.into_iter().collect(),
        existing_document,
    })
}

/// 取り込みの結果
#[derive(Debug)]
pub enum Outcome {
    /// 名簿に無い首長・議員がいるので止めた。名簿に登録してからやり直す
    Unmatched(Vec<String>),
    /// 同じ文書を取り込み済み（やり直すときは `again`）
    Duplicate(String),
    /// 発言が見つからない（見出しの形が違う）
    NoUtterances,
    Imported(Report),
}

#[derive(Debug)]
pub struct Report {
    pub document_id: String,
    pub utterances: usize,
    /// 下書きとして保存したトピック (id, タイトル)
    pub topics: Vec<(String, String)>,
    /// 捨てたものとその理由
    pub notes: Vec<String>,
}

/// 会議録を取り込み、トピック案を下書きとして保存する。
/// `again` なら、取り込み済みの文書からもう一度トピック案を作る（指示文を直して試すとき）
pub async fn import(
    pool: &SqlitePool,
    llm: &Llm,
    model: &str,
    moderation: &Moderation,
    input: &Input,
    again: bool,
) -> anyhow::Result<Outcome> {
    if input.text.chars().count() > MAX_CHARS {
        anyhow::bail!("会議録が長すぎます（{MAX_CHARS}文字まで）。分けて取り込んでください");
    }
    let prepared = prepare(pool, input).await?;
    if prepared.utterances.is_empty() {
        return Ok(Outcome::NoUtterances);
    }
    if !prepared.unmatched.is_empty() {
        return Ok(Outcome::Unmatched(prepared.unmatched));
    }
    if let (Some(id), false) = (&prepared.existing_document, again) {
        return Ok(Outcome::Duplicate(id.clone()));
    }

    let request = Request {
        purpose: Purpose::Extraction,
        model: model.to_owned(),
        system: extract::SYSTEM_PROMPT.into(),
        user: extract::build_input(&input.title, &input.held_on, &prepared.utterances),
        schema: extract::schema(),
        effort: Effort::Medium,
        max_tokens: 32_000,
    };
    let value = llm.structured(pool, &request).await?;
    let raw: extract::RawOutput = serde_json::from_value(value)?;
    let (checked, mut notes) = extract::check(raw, &prepared.utterances, &prepared.names);

    // 最初の意見もモデレーションに通し、承認されたものだけを残す
    let mut topics = Vec::new();
    for mut topic in checked {
        let mut approved = Vec::new();
        for body in topic.statements {
            let review = moderation.review(pool, &body).await;
            if review.verdict == Verdict::Approved {
                approved.push(body);
            } else {
                notes.push(format!(
                    "{}: モデレーションで {} になった意見を捨てた:「{body}」",
                    topic.title,
                    review.verdict.status()
                ));
            }
        }
        if approved.len() < extract::MIN_STATEMENTS {
            notes.push(format!(
                "モデレーションのあと意見が{}件未満なので捨てた: {}",
                extract::MIN_STATEMENTS,
                topic.title
            ));
            continue;
        }
        topic.statements = approved;
        topics.push(topic);
    }

    // 保存（トピックは下書き）
    let mut tx = pool.begin().await?;
    let document_id = match &prepared.existing_document {
        Some(id) => id.clone(),
        None => {
            let id = new_id();
            sqlx::query(
                "INSERT INTO documents (id, municipality_id, kind, title, url, held_on, sha256, char_count, imported_at)
                 VALUES ($1, $2, 'minutes', $3, $4, $5, $6, $7, $8)",
            )
            .bind(&id)
            .bind(&input.municipality_id)
            .bind(&input.title)
            .bind(&input.url)
            .bind(&input.held_on)
            .bind(&prepared.sha256)
            .bind(input.text.chars().count() as i64)
            .bind(now_iso())
            .execute(&mut *tx)
            .await?;
            id
        }
    };
    let by_no: HashMap<usize, &Utterance> = prepared.utterances.iter().map(|u| (u.no, u)).collect();
    let mut saved = Vec::new();
    for topic in &topics {
        let topic_id = new_id();
        let now = now_iso();
        sqlx::query(
            "INSERT INTO topics (id, municipality_id, title, description, source_type, source_url, created_at, status)
             VALUES ($1, $2, $3, $4, 'assembly', $5, $6, 'draft')",
        )
        .bind(&topic_id)
        .bind(&input.municipality_id)
        .bind(&topic.title)
        .bind(&topic.description)
        .bind(&input.url)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        for body in &topic.statements {
            sqlx::query(
                "INSERT INTO statements (id, topic_id, user_id, body, status, created_at)
                 VALUES ($1, $2, NULL, $3, 'approved', $4)",
            )
            .bind(new_id())
            .bind(&topic_id)
            .bind(body)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }
        for (position, source) in topic.sources.iter().enumerate() {
            let u = by_no[&source.utterance_no];
            sqlx::query(
                "INSERT INTO topic_sources (id, topic_id, document_id, position, utterance_no, speaker_label, official_id, quote)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(new_id())
            .bind(&topic_id)
            .bind(&document_id)
            .bind(position as i64)
            .bind(source.utterance_no as i64)
            .bind(u.speaker_label())
            .bind(prepared.officials.get(&source.utterance_no))
            .bind(&source.quote)
            .execute(&mut *tx)
            .await?;
        }
        saved.push((topic_id, topic.title.clone()));
    }
    tx.commit().await?;

    Ok(Outcome::Imported(Report {
        document_id,
        utterances: prepared.utterances.len(),
        topics: saved,
        notes,
    }))
}
