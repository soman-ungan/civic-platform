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
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::extraction::{Extraction, Prompt};
use crate::llm::{Effort, Llm, Purpose, Request};
use crate::moderation::{Moderation, Verdict};
use crate::status::StatementStatus;
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
    /// 議事日程
    pub agenda: Vec<String>,
    /// トピックを作る議事（一般質問・代表質問など）があるか
    pub topical: bool,
}

/// 自治体の会議録の読み取りの形。`collector_sources.config` の `speaker_pattern` /
/// `skip_pattern` / `agenda_pattern` / `topical_agenda_pattern` があればそれを使う
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
    if let Some(p) = config["agenda_pattern"].as_str() {
        patterns.agenda = Regex::new(p)?;
    }
    if let Some(p) = config["topical_agenda_pattern"].as_str() {
        patterns.topical_agenda = Regex::new(p)?;
    }
    Ok(patterns)
}

pub async fn prepare(pool: &SqlitePool, input: &Input) -> anyhow::Result<Prepared> {
    let patterns = patterns(pool, &input.municipality_id).await?;
    let utterances = parse::parse(&input.text, &patterns);
    let agenda = parse::agenda(&input.text, &patterns);
    let topical = agenda.iter().any(|a| patterns.topical_agenda.is_match(a));
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
        agenda,
        topical,
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
    /// トピックを作る議事（一般質問など）が無い、形式的な議事の会議録なので取り込まなかった
    /// （取り込むときは `force`）
    NotTopical(Vec<String>),
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

/// AI に送る発言（議長の短い進行の発言は除く。発言の番号は変えない）
fn speeches(prepared: &Prepared) -> Vec<Utterance> {
    prepared
        .utterances
        .iter()
        .filter(|u| !parse::is_procedural(u))
        .cloned()
        .collect()
}

/// AI に渡すもの（API に送るものと同じ）。共同作業では、これを書き出して読んでもらう
pub async fn prompt(pool: &SqlitePool, input: &Input) -> anyhow::Result<Prompt> {
    let prepared = prepare(pool, input).await?;
    Ok(Prompt {
        system: extract::SYSTEM_PROMPT.into(),
        user: extract::build_input(&input.title, &input.held_on, &speeches(&prepared)),
        schema: extract::schema(),
        attachment: None,
    })
}

/// API を呼んでトピック案を作り、そのままの出力（JSON）を返す（`extract minutes`。保存はしない）
pub async fn api_output(
    pool: &SqlitePool,
    llm: &Llm,
    model: &str,
    input: &Input,
) -> anyhow::Result<Value> {
    let prepared = prepare(pool, input).await?;
    call_api(pool, llm, model, input, &speeches(&prepared)).await
}

async fn call_api(
    pool: &SqlitePool,
    llm: &Llm,
    model: &str,
    input: &Input,
    speeches: &[Utterance],
) -> anyhow::Result<Value> {
    let request = Request {
        purpose: Purpose::Extraction,
        model: model.to_owned(),
        system: extract::SYSTEM_PROMPT.into(),
        user: extract::build_input(&input.title, &input.held_on, speeches),
        schema: extract::schema(),
        effort: Effort::Medium,
        max_tokens: 32_000,
        pdf: None,
    };
    // まれに、中身の無いトピックだけが返ることがある（タイトルが空）。そのときは1回だけやり直す
    let mut attempt = 0;
    loop {
        let value = llm.structured(pool, &request).await?;
        // 指示文を直すときの確認用（RUST_LOG=backend=debug で出る）
        tracing::debug!(output = %value, "extraction output");
        let raw: extract::RawOutput = serde_json::from_value(value.clone())?;
        let degenerate =
            !raw.topics.is_empty() && raw.topics.iter().all(|t| t.title.trim().is_empty());
        if !degenerate || attempt == 1 {
            break Ok(value);
        }
        tracing::warn!("extraction returned only empty topics; retrying once");
        attempt += 1;
    }
}

/// AI の出力を得る（API を呼ぶか、保存してある出力をそのまま使う）
async fn raw_output(
    pool: &SqlitePool,
    extraction: &Extraction<'_>,
    input: &Input,
    speeches: &[Utterance],
) -> anyhow::Result<extract::RawOutput> {
    let value = match extraction {
        Extraction::Api { llm, model } => call_api(pool, llm, model, input, speeches).await?,
        Extraction::Manual(value) | Extraction::Saved(value) => value.clone(),
    };
    serde_json::from_value(value).map_err(|e| {
        anyhow::anyhow!("出力の形が合いません（extract minutes --manual の「出力の形」）: {e}")
    })
}

/// 共同作業の出力などを、保存せずに確かめる（`import` と同じ検査。最初の意見のモデレーションはしない）
pub async fn preview(
    pool: &SqlitePool,
    extraction: &Extraction<'_>,
    input: &Input,
) -> anyhow::Result<(Vec<extract::CheckedTopic>, Vec<String>)> {
    let prepared = prepare(pool, input).await?;
    let speeches = speeches(&prepared);
    let raw = raw_output(pool, extraction, input, &speeches).await?;
    Ok(extract::check(raw, &speeches, &prepared.names))
}

/// 会議録を取り込み、トピック案を下書きとして保存する。
/// `again` なら、取り込み済みの文書からもう一度トピック案を作る（指示文を直して試すとき）。
/// `force` なら、形式的な議事だけの会議録でも取り込む
pub async fn import(
    pool: &SqlitePool,
    extraction: &Extraction<'_>,
    moderation: &Moderation,
    input: &Input,
    again: bool,
    force: bool,
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
    if !prepared.topical && !force {
        return Ok(Outcome::NotTopical(prepared.agenda));
    }
    let speeches = speeches(&prepared);

    let raw = raw_output(pool, extraction, input, &speeches).await?;
    // 引用と意見の根拠は、AI に送った発言の中から選ばれていること
    let (checked, mut notes) = extract::check(raw, &speeches, &prepared.names);

    // 最初の意見もモデレーションに通す。承認されたものと、判定できなかったもの（pending。
    // 通常の投稿と同じく、あとで判定し直す）を残し、非公開・要確認になったものは捨てる
    let mut topics: Vec<(
        extract::CheckedTopic,
        Vec<(&extract::CheckedStatement, StatementStatus)>,
    )> = Vec::new();
    for topic in &checked {
        let mut kept = Vec::new();
        for statement in &topic.statements {
            let review = moderation.review(pool, &statement.body).await;
            match review.verdict {
                Verdict::Approved | Verdict::Pending => {
                    kept.push((statement, review.verdict.status()));
                }
                _ => notes.push(format!(
                    "{}: モデレーションで {} になった意見を捨てた:「{}」",
                    topic.title,
                    review.verdict.status().as_str(),
                    statement.body
                )),
            }
        }
        if kept.len() < extract::MIN_STATEMENTS {
            notes.push(format!(
                "モデレーションのあと意見が{}件未満なので捨てた: {}",
                extract::MIN_STATEMENTS,
                topic.title
            ));
            continue;
        }
        if kept
            .iter()
            .any(|(_, status)| *status == StatementStatus::Pending)
        {
            notes.push(format!(
                "{}: 判定できなかった意見を確認待ち（pending）で残した。あとで判定し直す",
                topic.title
            ));
        }
        topics.push((topic.clone(), kept));
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
    sqlx::query("UPDATE documents SET extraction = $1 WHERE id = $2")
        .bind(extraction.method())
        .bind(&document_id)
        .execute(&mut *tx)
        .await?;
    let by_no: HashMap<usize, &Utterance> = prepared.utterances.iter().map(|u| (u.no, u)).collect();
    let mut saved = Vec::new();
    for (topic, statements) in &topics {
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
        for (statement, status) in statements {
            let statement_id = new_id();
            sqlx::query(
                "INSERT INTO statements (id, topic_id, user_id, body, status, created_at)
                 VALUES ($1, $2, NULL, $3, $4, $5)",
            )
            .bind(&statement_id)
            .bind(&topic_id)
            .bind(&statement.body)
            .bind(status.as_str())
            .bind(&now)
            .execute(&mut *tx)
            .await?;
            // 根拠の発言（運営者の確認用。API では返さない）
            for no in &statement.basis {
                sqlx::query(
                    "INSERT INTO statement_sources (statement_id, document_id, utterance_no, speaker_label)
                     VALUES ($1, $2, $3, $4)",
                )
                .bind(&statement_id)
                .bind(&document_id)
                .bind(*no as i64)
                .bind(by_no[no].speaker_label())
                .execute(&mut *tx)
                .await?;
            }
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
