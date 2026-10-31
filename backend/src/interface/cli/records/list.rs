// SPDX-License-Identifier: AGPL-3.0-only

//! list: 一覧を出す

use crate::ingest::collect;
use crate::interface::cli::{Ctx, municipality_id, required};
use crate::process::{self, similarity};
use crate::records::{self, accounts, publishing, topics};

use super::{ListArgs, Target};

/// 似ているトピック・意見の組。度合いの目安を手元の DB で確かめるのに使う
async fn list_similar(ctx: &Ctx, args: &ListArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    if let Some(topic) = &args.topic {
        let pairs =
            similarity::statement_pairs(pool, topic, similarity::SIMILAR_STATEMENT_SCORE).await?;
        if pairs.is_empty() {
            println!("似ている意見の組はありません");
        }
        for ((a, body_a), (b, body_b), score) in pairs {
            println!("{score:.2}\n  {a}  {body_a}\n  {b}  {body_b}");
        }
        return Ok(());
    }
    let slug = required(
        &args.municipality,
        "list similar -m <slug> または --topic <id>",
    )?;
    let id = municipality_id(pool, slug).await?;
    let pairs = similarity::topic_pairs(pool, &id).await?;
    if pairs.is_empty() {
        println!("似ているトピックの組はありません");
    }
    for (a, b) in pairs {
        println!(
            "{:.2}\n  {}  [{}]  {}\n  {}  [{}]  {}",
            a.score, a.id, a.status, a.title, b.id, b.status, b.title
        );
    }
    Ok(())
}

pub async fn list(ctx: &Ctx, args: ListArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    let by_slug = |slug: &str| args.municipality.as_deref().is_none_or(|m| m == slug);
    match args.target {
        Target::Budget => {
            let rows: Vec<_> = records::budget::list(pool)
                .await?
                .into_iter()
                .filter(|(slug, ..)| by_slug(slug))
                .collect();
            if rows.is_empty() {
                println!("予算はまだありません");
            }
            for (slug, year, status, total) in rows {
                println!("{slug}  {year}年度  [{status}]  歳出 {total} 千円");
            }
        }
        Target::Settlement => {
            let rows: Vec<_> = records::settlement::list(pool)
                .await?
                .into_iter()
                .filter(|r| by_slug(&r.slug))
                .collect();
            if rows.is_empty() {
                println!("決算カードはまだありません");
            }
            for r in rows {
                println!(
                    "{}  {}年度  歳出 {} 千円  人口 {} 人",
                    r.slug,
                    r.fiscal_year,
                    r.expenditure,
                    r.population.map_or("-".into(), |p| p.to_string())
                );
            }
        }
        Target::Minutes => {
            let rows: Vec<_> = records::minutes::list(pool)
                .await?
                .into_iter()
                .filter(|r| by_slug(&r.slug))
                .collect();
            if rows.is_empty() {
                println!("取り込んだ会議録はありません");
            }
            for r in rows {
                println!(
                    "{}  {}  {}  {}  トピック {} 件  [{}]",
                    r.id,
                    r.slug,
                    r.held_on.as_deref().unwrap_or("-"),
                    r.title,
                    r.topics,
                    r.extraction.as_deref().unwrap_or("-"),
                );
            }
        }
        Target::Topic if args.drafts => {
            let filter = match &args.municipality {
                Some(slug) => Some(municipality_id(pool, slug).await?),
                None => None,
            };
            let items: Vec<_> = topics::list(pool)
                .await?
                .into_iter()
                .filter(|d| filter.as_ref().is_none_or(|m| *m == d.municipality_id))
                .collect();
            if items.is_empty() {
                println!("下書きのトピックはありません");
            }
            for d in items {
                println!(
                    "{}  {}  [{}]  {}{}",
                    d.id,
                    d.created_at,
                    d.municipality_id,
                    d.title,
                    topics::source_mark(&d.source_type)
                );
            }
        }
        Target::Topic => {
            let rows: Vec<_> = topics::list_all(pool)
                .await?
                .into_iter()
                .filter(|r| by_slug(&r.slug))
                .collect();
            if rows.is_empty() {
                println!("トピックはありません");
            }
            for r in rows {
                println!(
                    "{}  {}  {}  [{}]  {}{}",
                    r.id,
                    r.created_at,
                    r.slug,
                    r.status,
                    r.title,
                    topics::source_mark(&r.source_type)
                );
            }
        }
        Target::Statement => list_statements(ctx, &args).await?,
        Target::Similar => list_similar(ctx, &args).await?,
        Target::Tag => {
            let filter = match &args.municipality {
                Some(slug) => Some(municipality_id(pool, slug).await?),
                None => None,
            };
            for tag in records::tags::list(pool, filter.as_deref()).await? {
                let kind = if tag.kind == "field" {
                    "分野"
                } else {
                    "地区"
                };
                println!("{}  [{kind}]  {}", tag.id, tag.name);
            }
        }
        Target::Operator => {
            let operators = accounts::list_operators(pool).await?;
            if operators.is_empty() {
                println!("運営者はいません（add operator で招待のリンクを作る）");
            }
            for o in operators {
                println!(
                    "{}  登録: {}  パスキー {} 個",
                    o.id,
                    o.registered_at.as_deref().unwrap_or("-"),
                    o.passkeys
                );
            }
        }
        Target::Queue => {
            let slug = required(&args.municipality, "list queue -m <slug>")?;
            let id = municipality_id(pool, slug).await?;
            let queue = publishing::queue(pool, &id).await?;
            let slots = publishing::slots(pool, Some(&id)).await?;
            if queue.is_empty() {
                println!("公開待ちはありません（add queue --id <topic_id> で入れる）");
            }
            if slots.is_empty() && !queue.is_empty() {
                println!("公開の時刻がありません（add slot -m {slug} --at 07:00 で足す）");
            }
            let planned = process::publishing::planned(&slots, queue.len(), chrono::Utc::now());
            for (i, q) in queue.iter().enumerate() {
                println!(
                    "{}  予定 {}  {}{}",
                    q.id,
                    planned.get(i).map(String::as_str).unwrap_or("-"),
                    q.title,
                    topics::source_mark(&q.source_type)
                );
            }
        }
        Target::Slot => {
            let id = match &args.municipality {
                Some(slug) => Some(municipality_id(pool, slug).await?),
                None => None,
            };
            let slots = publishing::slots(pool, id.as_deref()).await?;
            if slots.is_empty() {
                println!("公開の時刻はありません（add slot -m <slug> --at 07:00 で足す）");
            }
            for s in slots {
                println!(
                    "{}  [{}]  {}  最後に動いた日: {}",
                    s.id,
                    s.municipality_id,
                    s.time_of_day,
                    s.last_run_on.as_deref().unwrap_or("-")
                );
            }
        }
        Target::Source => {
            let sources: Vec<_> = collect::sources(pool)
                .await?
                .into_iter()
                .filter(|s| by_slug(&s.slug))
                .collect();
            if sources.is_empty() {
                println!("収集先はありません");
            }
            for s in sources {
                let terms = s
                    .config_json()
                    .ok()
                    .and_then(|c| c.get("terms_checked_at")?.as_str().map(str::to_owned));
                println!(
                    "{}  [{}] {} / {}  {}  規約の確認: {}  最後の取得: {}",
                    s.id,
                    s.municipality_id,
                    s.kind,
                    s.adapter,
                    if s.is_enabled { "有効" } else { "無効" },
                    terms.as_deref().unwrap_or("なし"),
                    s.last_crawled_at.as_deref().unwrap_or("-"),
                );
            }
        }
    }
    Ok(())
}

/// 意見の一覧。状態もトピックも指定しなければ、確認待ち（flagged）と判定待ち（pending）
async fn list_statements(ctx: &Ctx, args: &ListArgs) -> anyhow::Result<()> {
    let statuses: Vec<String> = if args.status.is_empty() {
        vec!["flagged".into(), "pending".into()]
    } else {
        args.status.clone()
    };
    let items = records::statements::list(
        &ctx.pool,
        &statuses,
        args.topic.as_deref(),
        args.municipality.as_deref(),
    )
    .await?;
    if items.is_empty() {
        if args.status.is_empty() {
            println!("確認待ちの意見はありません");
        } else {
            println!("{} の意見はありません", statuses.join("・"));
        }
    }
    for item in items {
        println!(
            "{}  [{}]  {}  topic={}\n    {}\n    理由: {}",
            item.id,
            item.status,
            item.created_at,
            item.topic_id,
            item.body,
            item.moderation_note.as_deref().unwrap_or("-"),
        );
    }
    Ok(())
}
