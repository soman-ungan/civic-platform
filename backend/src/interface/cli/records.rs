// SPDX-License-Identifier: AGPL-3.0-only

//! 段階2: DB を見る・直す（list / show / edit / add / remove / publish / archive）。
//! 中身は `records`（収集先は `ingest::collect`）。ここでは引数を読み、表示だけをする

use clap::{Args, ValueEnum};

use crate::ingest::{budget, collect};
use crate::process::{self, moderation, progress, similarity};
use crate::records::{self, accounts, publishing, topics};
use crate::shared::status::{ItemKind, Publication};

use super::{Ctx, municipality_id, required, skip};

/// 段階2の対象（画面やドキュメントで使っている、データのまとまり）
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Target {
    /// 予算（款別・性質別・歳入・主な事業）
    Budget,
    /// 決算カード（費目ごとの金額と人口）
    Settlement,
    /// 取り込んだ会議録と、そこから作ったトピック
    Minutes,
    /// トピック（出典の引用・タグ）
    Topic,
    /// 意見
    Statement,
    /// タグ（分野の語彙と地区）。トピックに付ける・外す
    Tag,
    /// 収集先（collector_sources）
    Source,
    /// 運営者（パスキーで登録したアカウントのうち、運営者にしたもの。ADR 0006）
    Operator,
    /// 公開待ちの下書き（決まった時刻に古い順に1件ずつ公開する。docs/plans/2026-10-30-timeline-flow.md）
    Queue,
    /// 自動公開の時刻（自治体ごと。1つで1日1件）
    Slot,
    /// 似ているトピックの組（-m）、またはトピックの中の似ている意見の組（--topic）。費用 0（process::similarity）
    Similar,
    /// 提言（締め切ったトピックの合意点と意見が分かれた点。ADR 0005）
    Recommendation,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    target: Target,
    /// 自治体の URL の名前（例: otawara）で絞る
    #[arg(short, long)]
    municipality: Option<String>,
    /// 下書きだけ（topic）
    #[arg(long)]
    drafts: bool,
    /// 意見の状態で絞る（statement。カンマ区切り。既定は確認待ち flagged,pending）
    #[arg(long, value_delimiter = ',')]
    status: Vec<String>,
    /// トピックで絞る（statement）
    #[arg(long)]
    topic: Option<String>,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    target: Target,
    /// 自治体の URL の名前（budget・settlement）
    #[arg(short, long)]
    municipality: Option<String>,
    /// 年度（西暦。budget・settlement）
    #[arg(short, long)]
    year: Option<i32>,
    /// id（minutes は document の id、topic・statement・source はそれぞれの id）
    #[arg(long)]
    id: Option<String>,
}

#[derive(Debug, Args)]
pub struct EditArgs {
    target: Target,
    /// トピック・意見・時刻（slot）の id
    #[arg(long)]
    id: Option<String>,
    /// 日本時間の時刻（slot。例: 07:30）
    #[arg(long)]
    at: Option<String>,
    /// タイトル（topic）
    #[arg(long)]
    title: Option<String>,
    /// 説明（topic）
    #[arg(long)]
    description: Option<String>,
    /// 本文（statement。140文字以内。投票がある意見は直せない）
    #[arg(long)]
    body: Option<String>,
    /// 前書き（recommendation。見出しは --title）
    #[arg(long)]
    preface: Option<String>,
    /// 見出しと前書きを AI に書き直してもらう（recommendation。API キーが要る。費用がかかる）
    #[arg(long)]
    ai: bool,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    target: Target,
    /// 対象のトピックの id（recommendation はカンマ区切りで複数）
    #[arg(long, value_delimiter = ',')]
    topic: Vec<String>,
    /// 子にする公開済みの提言の id（recommendation。カンマ区切り）
    #[arg(long, value_delimiter = ',')]
    child: Vec<String>,
    /// 元の公開済みの提言の id（topic。第2回のトピックの下書きを作る）
    #[arg(long)]
    from_recommendation: Option<String>,
    /// 意見の本文（statement。140文字以内）
    #[arg(long)]
    body: Option<String>,
    /// タグの id（tag。例: field-kosodate、092100-kurobane。list tag で見られる）・下書きのトピックの id（queue）
    #[arg(long)]
    id: Option<String>,
    /// 自治体の URL の名前（slot）
    #[arg(short, long)]
    municipality: Option<String>,
    /// 日本時間の時刻（slot。例: 07:30）
    #[arg(long)]
    at: Option<String>,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    target: Target,
    /// 意見の id（statement）・タグの id（tag）・運営者のアカウントの id（operator。list operator で見られる）・
    /// 下書きのトピックの id（queue）・時刻の id（slot。list slot で見られる）
    #[arg(long)]
    id: Option<String>,
    /// 対象のトピックの id（tag）
    #[arg(long)]
    topic: Option<String>,
}

#[derive(Debug, Args)]
pub struct PublishArgs {
    target: Target,
    /// 自治体の URL の名前（budget）
    #[arg(short, long)]
    municipality: Option<String>,
    /// 年度（西暦。budget）
    #[arg(short, long)]
    year: Option<i32>,
    /// トピックの id（topic）
    #[arg(long)]
    id: Option<String>,
}

// ===== list =====

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
        Target::Recommendation => list_recommendations(ctx, &args).await?,
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

// ===== show =====

pub async fn show(ctx: &Ctx, args: ShowArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    match args.target {
        Target::Budget => {
            let usage = "show budget -m <slug> -y <年度>";
            let slug = required(&args.municipality, usage)?;
            let year = *required(&args.year, usage)?;
            let id = municipality_id(pool, slug).await?;
            let Some((status, checked, url)) = records::budget::load(pool, &id, year).await? else {
                anyhow::bail!("{slug} の {year}年度の予算はありません");
            };
            println!("[{status}]");
            print!("{}", budget::render(&checked, url.as_deref()));
        }
        Target::Settlement => {
            let usage = "show settlement -m <slug> -y <年度>";
            let slug = required(&args.municipality, usage)?;
            let year = *required(&args.year, usage)?;
            let id = municipality_id(pool, slug).await?;
            let Some(card) = records::settlement::load(pool, &id, year).await? else {
                anyhow::bail!("{slug} の {year}年度の決算カードはありません");
            };
            println!(
                "{slug} {year}年度  人口 {} 人",
                card.population.map_or("-".into(), |p| p.to_string())
            );
            if let Some(url) = &card.source_url {
                println!("出典: {url}");
            }
            let mut category = String::new();
            for item in card.items {
                if item.category != category {
                    println!("\n{}", item.category);
                    category = item.category;
                }
                println!("  {}: {}（{}）", item.item_key, item.value, item.unit);
            }
        }
        Target::Minutes => {
            let id = required(
                &args.id,
                "show minutes --id <document の id>（list minutes で見られる）",
            )?;
            let Some((doc, topics)) = records::minutes::load(pool, id).await? else {
                anyhow::bail!("会議録が見つかりません: {id}");
            };
            println!("{}（{}）", doc.title, doc.held_on.as_deref().unwrap_or("-"));
            println!(
                "出典: {}  作り方: {}  取り込み: {}",
                doc.url.as_deref().unwrap_or("-"),
                doc.extraction.as_deref().unwrap_or("-"),
                doc.imported_at
            );
            println!("作ったトピック {} 件", topics.len());
            for t in topics {
                println!("  {}  [{}]  {}", t.id, t.status, t.title);
            }
        }
        Target::Topic => {
            let id = required(&args.id, "show topic --id <id>")?;
            match topics::show(pool, id).await? {
                Some(detail) => print!("{}", detail.render()),
                None => anyhow::bail!("トピックが見つかりません: {id}"),
            }
        }
        Target::Statement => {
            let id = required(&args.id, "show statement --id <id>")?;
            let Some(d) = records::statements::load(pool, id).await? else {
                anyhow::bail!("意見が見つかりません: {id}");
            };
            let s = &d.statement;
            // 投稿者が誰かは出さない（運営のコマンドでも、最初の意見か市民の投稿かだけ）
            println!(
                "{id}  [{}]  {}  topic={}  {}",
                s.status,
                s.created_at,
                s.topic_id,
                match (d.seed, d.posted) {
                    (true, _) => "最初の意見",
                    (false, true) => "市民の投稿",
                    (false, false) => "あとから足した議会の主張・運営者の意見",
                }
            );
            println!("  {}", s.body);
            println!("  理由: {}", s.moderation_note.as_deref().unwrap_or("-"));
            println!(
                "  投票: 賛成 {}・反対 {}・パス {}",
                d.agree, d.disagree, d.pass
            );
        }
        Target::Tag => {
            return skip(
                "show",
                args.target,
                "タグは名前だけで、中身が無い（list tag で一覧を見る）",
            );
        }
        Target::Operator => {
            return skip(
                "show",
                args.target,
                "運営者のアカウントは ID と登録日だけで、中身が無い（list operator で一覧を見る）",
            );
        }
        Target::Recommendation => {
            let id = required(&args.id, "show recommendation --id <id>")?;
            show_recommendation(ctx, id).await?;
        }
        Target::Similar => skip(
            "show",
            args.target,
            "似ているかは計算するだけで、保存しない（list similar で見る）",
        )?,
        Target::Queue | Target::Slot => {
            return skip(
                "show",
                args.target,
                "一覧に中身がすべて出る（list queue -m <slug> / list slot）",
            );
        }
        Target::Source => {
            let id = required(&args.id, "show source --id <id>（list source で見られる）")?;
            let Some(s) = collect::sources(pool)
                .await?
                .into_iter()
                .find(|s| s.id == *id)
            else {
                anyhow::bail!("収集先が見つかりません: {id}");
            };
            println!(
                "{}  [{}] {}  {} / {}  {}",
                s.id,
                s.municipality_id,
                s.slug,
                s.kind,
                s.adapter,
                if s.is_enabled { "有効" } else { "無効" }
            );
            println!("  base_url: {}", s.base_url);
            println!(
                "  最後の取得: {}",
                s.last_crawled_at.as_deref().unwrap_or("-")
            );
            println!(
                "  config: {}",
                serde_json::to_string_pretty(&s.config_json()?)?
            );
        }
    }
    Ok(())
}

// ===== edit / add / remove =====

pub async fn edit(ctx: &Ctx, args: EditArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    match args.target {
        Target::Topic => {
            let usage = "edit topic --id <id> [--title <タイトル>] [--description <説明>]";
            let id = required(&args.id, usage)?;
            if args.title.is_none() && args.description.is_none() {
                anyhow::bail!("使い方: {usage}");
            }
            topics::edit_topic(pool, id, args.title.as_deref(), args.description.as_deref())
                .await?;
            println!("{id}: 直しました");
        }
        Target::Statement => {
            let usage = "edit statement --id <id> --body <本文>";
            let id = required(&args.id, usage)?;
            topics::edit_statement(pool, id, required(&args.body, usage)?).await?;
            println!("{id}: 直しました");
        }
        Target::Budget | Target::Settlement | Target::Minutes => skip(
            "edit",
            args.target,
            "資料から取り込んだ数字・原文は手で直さない（直すときは import --again で作り直す）",
        )?,
        Target::Tag => skip(
            "edit",
            args.target,
            "タグの語彙は seed で管理する（data/seeds/020_field_tags.sql）",
        )?,
        Target::Source => skip(
            "edit",
            args.target,
            "収集先は seed で管理する（data/seeds/010_municipality_*.sql）",
        )?,
        Target::Operator => skip(
            "edit",
            args.target,
            "運営者は足すか外すだけ（add / remove operator）",
        )?,
        Target::Recommendation => edit_recommendation(ctx, &args).await?,
        Target::Similar => skip(
            "edit",
            args.target,
            "似ているかは計算するだけで、保存しない（list similar で見る）",
        )?,
        Target::Queue => skip(
            "edit",
            args.target,
            "公開待ちは入れた順（入れるか外すだけ。add / remove queue）",
        )?,
        Target::Slot => {
            let usage = "edit slot --id <id> --at <HH:MM>";
            let id = required(&args.id, usage)?;
            let Some(slot) = publishing::slots(pool, None)
                .await?
                .into_iter()
                .find(|s| s.id == *id)
            else {
                anyhow::bail!("時刻が見つかりません: {id}");
            };
            let at = required(&args.at, usage)?;
            process::publishing::update_slot(
                pool,
                &slot.municipality_id,
                id,
                at,
                chrono::Utc::now(),
            )
            .await?;
            println!("{id}: {at} にしました");
        }
    }
    Ok(())
}

pub async fn add(ctx: &Ctx, args: AddArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    match args.target {
        Target::Statement => {
            let usage = "add statement --topic <topic_id> --body <本文>";
            let first = args.topic.first().cloned();
            let topic = required(&first, usage)?;
            let sid = topics::add_statement(pool, topic, required(&args.body, usage)?).await?;
            println!("{sid}: 足しました");
        }
        Target::Tag => {
            let usage = "add tag --topic <topic_id> --id <tag_id>";
            let first = args.topic.first().cloned();
            let topic = required(&first, usage)?;
            let tag = required(&args.id, usage)?;
            topics::set_tag(pool, topic, tag, true).await?;
            println!("{topic}: {tag} を付けました");
        }
        Target::Operator => {
            // 運営者は画面からは増やせない。1回だけ使える招待のリンクを作り、そこからパスキーを登録してもらう
            let token = accounts::create_operator_invite(pool).await?;
            println!(
                "運営者の招待のリンクを作りました（{} 時間有効・1回だけ使える）:",
                accounts::OPERATOR_INVITE_TTL_HOURS
            );
            println!(
                "  {}/operator/join?token={token}",
                ctx.config.webauthn_origin.trim_end_matches('/')
            );
            println!(
                "このリンクは本人にだけ渡してください。リンクからパスキーを登録したアカウントが運営者になります"
            );
        }
        Target::Queue => {
            let id = required(&args.id, "add queue --id <topic_id>（下書きだけ）")?;
            if !publishing::enqueue(pool, None, id).await? {
                anyhow::bail!("下書きのトピックが見つかりません: {id}");
            }
            println!("{id}: 公開待ちに入れました");
        }
        Target::Slot => {
            let usage = "add slot -m <slug> --at <HH:MM>（日本時間）";
            let slug = required(&args.municipality, usage)?;
            let at = required(&args.at, usage)?;
            let mid = municipality_id(pool, slug).await?;
            let id = process::publishing::add_slot(pool, &mid, at, chrono::Utc::now()).await?;
            println!("{id}: {slug} に {at} を足しました");
        }
        Target::Recommendation => add_recommendation(ctx, &args).await?,
        Target::Topic if args.from_recommendation.is_some() => {
            let id = args.from_recommendation.as_deref().unwrap_or_default();
            match records::recommendations::second_round(pool, id).await? {
                Ok(topic) => println!(
                    "{topic}: 第2回のトピックの下書きを作りました（show topic --id {topic} で確かめ、最初の意見を減らしてから公開）"
                ),
                Err(e) => anyhow::bail!("{e}: {id}"),
            }
        }
        other => skip("add", other, not_editable(other))?,
    }
    Ok(())
}

pub async fn remove(ctx: &Ctx, args: RemoveArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    match args.target {
        Target::Statement => {
            let id = required(&args.id, "remove statement --id <id>")?;
            if !moderation::decide(pool, id, false).await? {
                anyhow::bail!("意見が見つかりません: {id}");
            }
            println!("{id}: 取り下げました");
        }
        Target::Tag => {
            let usage = "remove tag --topic <topic_id> --id <tag_id>";
            let topic = required(&args.topic, usage)?;
            let tag = required(&args.id, usage)?;
            topics::set_tag(pool, topic, tag, false).await?;
            println!("{topic}: {tag} を外しました");
        }
        Target::Operator => {
            let id = required(
                &args.id,
                "remove operator --id <id>（list operator で見られる）",
            )?;
            if !accounts::remove_operator(pool, id).await? {
                anyhow::bail!("運営者が見つかりません: {id}");
            }
            println!("{id}: 運営者から外しました（アカウントとパスキーは残る）");
        }
        Target::Recommendation => {
            let id = required(&args.id, "remove recommendation --id <id>（下書きだけ）")?;
            if !records::recommendations::remove_draft(pool, id).await? {
                anyhow::bail!(
                    "下書きの提言が見つかりません（公開したものは archive で取り下げる）: {id}"
                );
            }
            println!("{id}: 下書きを消しました");
        }
        Target::Queue => {
            let id = required(&args.id, "remove queue --id <topic_id>")?;
            if !publishing::dequeue(pool, None, id).await? {
                anyhow::bail!("下書きのトピックが見つかりません: {id}");
            }
            println!("{id}: 公開待ちから外しました（下書きのまま）");
        }
        Target::Slot => {
            let id = required(&args.id, "remove slot --id <id>（list slot で見られる）")?;
            if !publishing::remove_slot(pool, None, id).await? {
                anyhow::bail!("時刻が見つかりません: {id}");
            }
            println!("{id}: 消しました");
        }
        other => skip("remove", other, not_editable(other))?,
    }
    Ok(())
}

/// add / remove が何もしない対象の理由
fn not_editable(target: Target) -> &'static str {
    match target {
        Target::Budget | Target::Settlement | Target::Minutes => {
            "資料から取り込んだものは手で足し引きしない（import で作り直す）"
        }
        Target::Topic => {
            "トピックは会議録（import minutes）か、市民の提案（画面）、公開した提言（add topic --from-recommendation <id>）から作る"
        }
        Target::Source => "収集先は seed で管理する（data/seeds/010_municipality_*.sql）",
        Target::Similar => "似ているかは計算するだけで、保存しない（list similar で見る）",
        Target::Recommendation => "",
        Target::Statement | Target::Tag | Target::Operator | Target::Queue | Target::Slot => "",
    }
}

// ===== publish / archive =====

pub async fn publish(ctx: &Ctx, args: PublishArgs, publish: bool) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    let (verb, status) = if publish {
        ("publish", Publication::Published)
    } else {
        ("archive", Publication::Archived)
    };
    match args.target {
        Target::Budget => {
            let usage = format!("{verb} budget -m <slug> -y <年度>");
            let slug = required(&args.municipality, &usage)?;
            let year = *required(&args.year, &usage)?;
            let id = municipality_id(pool, slug).await?;
            if !records::budget::set_status(pool, &id, year, status).await? {
                anyhow::bail!("{slug} の {year}年度の予算はありません");
            }
            println!("{slug} {year}年度: {}", status.as_str());
        }
        Target::Topic => {
            let id = required(&args.id, &format!("{verb} topic --id <id>"))?;
            if !topics::set_status(pool, id, status, &progress::default_closes_at()).await? {
                anyhow::bail!("トピックが見つかりません: {id}");
            }
            println!("{id}: {}", status.as_str());
        }
        Target::Settlement => skip(
            verb,
            args.target,
            "決算カードは下書きを持たない（import settlement で取り込んだら公開される）",
        )?,
        Target::Minutes => skip(
            verb,
            args.target,
            "会議録から作ったトピックは1つずつ公開・取り下げする（publish topic --id。show minutes で一覧）",
        )?,
        Target::Statement => skip(
            verb,
            args.target,
            "意見の公開・非公開は判定として行う（moderate statement --id <id> --approve | --reject）",
        )?,
        Target::Tag | Target::Source | Target::Operator => {
            skip(verb, args.target, "公開・取り下げの状態を持たない")?
        }
        Target::Recommendation => {
            let id = required(&args.id, &format!("{verb} recommendation --id <id>"))?;
            match records::recommendations::set_status(pool, id, status).await? {
                Ok(()) => println!("{id}: {}", status.as_str()),
                Err(e) => anyhow::bail!("{e}: {id}"),
            }
        }
        Target::Similar => skip(
            verb,
            args.target,
            "似ているかは計算するだけで、保存しない（list similar で見る）",
        )?,
        Target::Queue | Target::Slot => skip(
            verb,
            args.target,
            "公開待ちは決まった時刻に公開される。急ぐものは publish topic --id で今すぐ公開する",
        )?,
    }
    Ok(())
}

// ===== 提言（ADR 0005。docs/plans/2026-10-30-opinion-synthesis.md） =====

/// 提言（公開済み・下書き・取り下げ）と、提言にできるトピック・まとまりの候補
async fn list_recommendations(ctx: &Ctx, args: &ListArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    let slug = required(&args.municipality, "list recommendation -m <slug>")?;
    let mid = municipality_id(pool, slug).await?;
    for status in Publication::ALL {
        for r in records::recommendations::list(pool, &mid, status).await? {
            println!(
                "{}  [{}]  {}  合意点 {} 件・分かれた点 {} 件",
                r.id,
                r.status,
                if r.title.is_empty() {
                    "（見出し未定）"
                } else {
                    &r.title
                },
                r.consensus,
                r.divided
            );
        }
    }
    let (topics, groups) = process::recommendation::candidates(pool, &mid).await?;
    println!("\n提言にできるトピック（締め切ったもの）:");
    if topics.is_empty() {
        println!("  ありません");
    }
    for t in &topics {
        println!(
            "  {}  {}人  {}  [{}]",
            t.id,
            t.participants,
            t.title,
            t.fields.join("・")
        );
    }
    if !groups.is_empty() {
        println!("\nまとまりの候補（同じ分野で似ているもの）:");
        for g in groups {
            println!("  --topic {}", g.join(","));
        }
    }
    Ok(())
}

async fn show_recommendation(ctx: &Ctx, id: &str) -> anyhow::Result<()> {
    let Some(d) = records::recommendations::show(&ctx.pool, id).await? else {
        anyhow::bail!("提言が見つかりません: {id}");
    };
    println!("{}  [{}]", d.id, d.status);
    println!("見出し: {}", d.title);
    println!("前書き: {}", d.preface);
    println!(
        "基準: 合意点 {} 以上・分かれた点 {} 以上",
        d.consensus_threshold, d.divided_threshold
    );
    for t in &d.topics {
        println!("元のトピック: {}  {}  {}人", t.id, t.title, t.participants);
    }
    for c in &d.children {
        println!("子の提言: {}  {}", c.id, c.title);
    }
    for item in &d.items {
        let kind = match item.kind {
            ItemKind::Consensus => "合意点",
            ItemKind::Divided => "分かれた点",
        };
        let groups: Vec<String> = item
            .groups
            .iter()
            .map(|g| format!("{} 賛成{}/反対{}/{}", g.label, g.agree, g.disagree, g.seen))
            .collect();
        println!("[{kind}] {}  （{}）", item.body, groups.join("、"));
    }
    Ok(())
}

/// 下書きの提言を作る。API キーがあれば、見出しと前書きの下書きを AI に頼む
async fn add_recommendation(ctx: &Ctx, args: &AddArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    let slug = required(
        &args.municipality,
        "add recommendation -m <slug> --topic <id,id> [--child <id,id>]",
    )?;
    let mid = municipality_id(pool, slug).await?;
    let id = process::recommendation::create(
        pool,
        &mid,
        &args.topic,
        &args.child,
        ctx.config.recommendation_divided_threshold,
    )
    .await?;
    println!("{id}: 提言の下書きを作りました");
    write_text_with_ai(ctx, &id).await;
    println!("show recommendation --id {id} で確かめ、publish recommendation --id {id} で公開");
    Ok(())
}

/// API キーがあれば、見出しと前書きの下書きを AI に頼む（失敗しても提言はそのまま。手で書ける）
async fn write_text_with_ai(ctx: &Ctx, id: &str) {
    let Some(llm) = &ctx.llm else {
        println!(
            "API キーが無いので、見出しと前書きは edit recommendation --id {id} --title … --preface … で書いてください"
        );
        return;
    };
    match process::recommendation::draft_text(&ctx.pool, llm, &ctx.config.moderation_model, id)
        .await
    {
        Ok((title, preface)) => println!("AI の下書き:\n  見出し: {title}\n  前書き: {preface}"),
        Err(e) => println!("見出しと前書きの下書きを作れませんでした（手で書いてください）: {e}"),
    }
}

async fn edit_recommendation(ctx: &Ctx, args: &EditArgs) -> anyhow::Result<()> {
    let id = required(
        &args.id,
        "edit recommendation --id <id> [--title …] [--preface …] [--ai]",
    )?;
    if args.ai {
        write_text_with_ai(ctx, id).await;
        return Ok(());
    }
    match records::recommendations::edit(
        &ctx.pool,
        id,
        args.title.as_deref(),
        args.preface.as_deref(),
    )
    .await?
    {
        Ok(()) => println!("{id}: 直しました"),
        Err(e) => anyhow::bail!("{e}: {id}"),
    }
    Ok(())
}
