// SPDX-License-Identifier: AGPL-3.0-only

//! 段階2: DB を見る・直す（list / show / edit / add / remove / publish / archive）。
//! 下書きは API のどこにも出ない。運営者がここで確かめてから公開する（ADR 0004）

use backend::status::Publication;
use backend::{budget, collect, drafts, moderation};
use clap::{Args, ValueEnum};

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
    /// トピック・意見の id
    #[arg(long)]
    id: Option<String>,
    /// タイトル（topic）
    #[arg(long)]
    title: Option<String>,
    /// 説明（topic）
    #[arg(long)]
    description: Option<String>,
    /// 本文（statement。140文字以内。投票がある意見は直せない）
    #[arg(long)]
    body: Option<String>,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    target: Target,
    /// 対象のトピックの id
    #[arg(long)]
    topic: Option<String>,
    /// 意見の本文（statement。140文字以内）
    #[arg(long)]
    body: Option<String>,
    /// タグの id（tag。例: field-kosodate、092100-kurobane。list tag で見られる）
    #[arg(long)]
    id: Option<String>,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    target: Target,
    /// 意見の id（statement）・タグの id（tag）
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

#[derive(sqlx::FromRow)]
struct MinutesRow {
    id: String,
    slug: String,
    title: String,
    held_on: Option<String>,
    extraction: Option<String>,
    topics: i64,
}

#[derive(sqlx::FromRow)]
struct StatementRow {
    #[sqlx(flatten)]
    item: moderation::QueueItem,
    slug: String,
}

#[derive(sqlx::FromRow)]
struct MinutesDocument {
    title: String,
    held_on: Option<String>,
    url: Option<String>,
    extraction: Option<String>,
    imported_at: String,
}

pub async fn list(ctx: &Ctx, args: ListArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    let by_slug = |slug: &str| args.municipality.as_deref().is_none_or(|m| m == slug);
    match args.target {
        Target::Budget => {
            let rows: Vec<_> = budget::list(pool)
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
            let rows: Vec<(String, i32, i64, Option<i64>)> = sqlx::query_as(
                "SELECT m.slug, s.fiscal_year, SUM(s.value), st.population
                 FROM settlement_items s
                 JOIN municipalities m ON m.id = s.municipality_id
                 LEFT JOIN municipality_stats st
                   ON st.municipality_id = s.municipality_id AND st.fiscal_year = s.fiscal_year
                 WHERE s.category = 'expenditure_purpose'
                 GROUP BY m.slug, s.fiscal_year, st.population
                 ORDER BY m.slug, s.fiscal_year",
            )
            .fetch_all(pool)
            .await?;
            let rows: Vec<_> = rows
                .into_iter()
                .filter(|(slug, ..)| by_slug(slug))
                .collect();
            if rows.is_empty() {
                println!("決算カードはまだありません");
            }
            for (slug, year, total, population) in rows {
                println!(
                    "{slug}  {year}年度  歳出 {total} 千円  人口 {} 人",
                    population.map_or("-".into(), |p| p.to_string())
                );
            }
        }
        Target::Minutes => {
            let rows: Vec<MinutesRow> = sqlx::query_as(
                    "SELECT d.id, m.slug, d.title, d.held_on, d.extraction,
                            (SELECT COUNT(DISTINCT ts.topic_id) FROM topic_sources ts WHERE ts.document_id = d.id) AS topics
                     FROM documents d JOIN municipalities m ON m.id = d.municipality_id
                     WHERE d.kind = 'minutes' ORDER BY m.slug, d.held_on, d.title",
                )
                .fetch_all(pool)
                .await?;
            let rows: Vec<_> = rows.into_iter().filter(|r| by_slug(&r.slug)).collect();
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
            let items = drafts::list(pool).await?;
            let filter = match &args.municipality {
                Some(slug) => Some(municipality_id(pool, slug).await?),
                None => None,
            };
            let items: Vec<_> = items
                .into_iter()
                .filter(|d| filter.as_ref().is_none_or(|m| *m == d.municipality_id))
                .collect();
            if items.is_empty() {
                println!("下書きのトピックはありません");
            }
            for d in items {
                println!(
                    "{}  {}  [{}]  {}",
                    d.id, d.created_at, d.municipality_id, d.title
                );
            }
        }
        Target::Topic => {
            let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
                "SELECT t.id, t.created_at, m.slug, t.status, t.title
                 FROM topics t JOIN municipalities m ON m.id = t.municipality_id
                 ORDER BY m.slug, t.created_at",
            )
            .fetch_all(pool)
            .await?;
            let rows: Vec<_> = rows.into_iter().filter(|r| by_slug(&r.2)).collect();
            if rows.is_empty() {
                println!("トピックはありません");
            }
            for (id, created_at, slug, status, title) in rows {
                println!("{id}  {created_at}  {slug}  [{status}]  {title}");
            }
        }
        Target::Statement => list_statements(ctx, &args).await?,
        Target::Tag => {
            let filter = match &args.municipality {
                Some(slug) => Some(municipality_id(pool, slug).await?),
                None => None,
            };
            let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
                "SELECT t.id, t.kind, t.name, d.municipality_id
                 FROM tags t LEFT JOIN districts d ON d.id = t.district_id
                 ORDER BY t.kind DESC, d.municipality_id, t.id",
            )
            .fetch_all(pool)
            .await?;
            for (id, kind, name, municipality) in rows {
                // 地区のタグは、その自治体のものだけ（自治体を選ばなければすべて）
                if let (Some(want), Some(m)) = (&filter, &municipality)
                    && want != m
                {
                    continue;
                }
                let kind = if kind == "field" { "分野" } else { "地区" };
                println!("{id}  [{kind}]  {name}");
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
    let pool = &ctx.pool;
    let statuses: Vec<String> = if args.status.is_empty() {
        vec!["flagged".into(), "pending".into()]
    } else {
        args.status.clone()
    };
    // 絞り込みは手元で行う（両方の DB で動く SQL にするため）
    let rows: Vec<StatementRow> = sqlx::query_as(
        "SELECT s.id, s.topic_id, s.status, s.body, s.moderation_note, s.created_at, m.slug
             FROM statements s JOIN topics t ON t.id = s.topic_id
             JOIN municipalities m ON m.id = t.municipality_id
             ORDER BY s.created_at",
    )
    .fetch_all(pool)
    .await?;
    let items: Vec<moderation::QueueItem> = rows
        .into_iter()
        .filter(|r| statuses.contains(&r.item.status))
        .filter(|r| args.topic.as_ref().is_none_or(|t| *t == r.item.topic_id))
        .filter(|r| args.municipality.as_ref().is_none_or(|m| *m == r.slug))
        .map(|r| r.item)
        .collect();
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
            let Some((status, checked, url)) = budget::load(pool, &id, year).await? else {
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
            let items: Vec<(String, String, i64, String, Option<String>)> = sqlx::query_as(
                "SELECT s.category, s.item_key, s.value, s.unit, d.url
                 FROM settlement_items s LEFT JOIN documents d ON d.id = s.document_id
                 WHERE s.municipality_id = $1 AND s.fiscal_year = $2
                 ORDER BY s.category, s.position",
            )
            .bind(&id)
            .bind(year)
            .fetch_all(pool)
            .await?;
            if items.is_empty() {
                anyhow::bail!("{slug} の {year}年度の決算カードはありません");
            }
            let population: Option<i64> = sqlx::query_scalar(
                "SELECT population FROM municipality_stats WHERE municipality_id = $1 AND fiscal_year = $2",
            )
            .bind(&id)
            .bind(year)
            .fetch_optional(pool)
            .await?;
            println!(
                "{slug} {year}年度  人口 {} 人",
                population.map_or("-".into(), |p| p.to_string())
            );
            if let Some(url) = items.first().and_then(|i| i.4.clone()) {
                println!("出典: {url}");
            }
            let mut category = String::new();
            for (c, key, value, unit, _) in items {
                if c != category {
                    println!("\n{c}");
                    category = c;
                }
                println!("  {key}: {value}（{unit}）");
            }
        }
        Target::Minutes => {
            let id = required(
                &args.id,
                "show minutes --id <document の id>（list minutes で見られる）",
            )?;
            let doc: Option<MinutesDocument> = sqlx::query_as(
                "SELECT title, held_on, url, extraction, imported_at FROM documents
                     WHERE id = $1 AND kind = 'minutes'",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?;
            let Some(doc) = doc else {
                anyhow::bail!("会議録が見つかりません: {id}");
            };
            println!("{}（{}）", doc.title, doc.held_on.as_deref().unwrap_or("-"));
            println!(
                "出典: {}  作り方: {}  取り込み: {}",
                doc.url.as_deref().unwrap_or("-"),
                doc.extraction.as_deref().unwrap_or("-"),
                doc.imported_at
            );
            let topics: Vec<(String, String, String)> = sqlx::query_as(
                "SELECT DISTINCT t.id, t.status, t.title FROM topics t
                 JOIN topic_sources ts ON ts.topic_id = t.id
                 WHERE ts.document_id = $1 ORDER BY t.id",
            )
            .bind(id)
            .fetch_all(pool)
            .await?;
            println!("作ったトピック {} 件", topics.len());
            for (id, status, title) in topics {
                println!("  {id}  [{status}]  {title}");
            }
        }
        Target::Topic => {
            let id = required(&args.id, "show topic --id <id>")?;
            match drafts::show(pool, id).await? {
                Some(detail) => print!("{}", detail.render()),
                None => anyhow::bail!("トピックが見つかりません: {id}"),
            }
        }
        Target::Statement => {
            let id = required(&args.id, "show statement --id <id>")?;
            let row: Option<(String, String, String, Option<String>, String, bool)> =
                sqlx::query_as(
                    "SELECT topic_id, status, body, moderation_note, created_at, user_id IS NULL
                     FROM statements WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;
            let Some((topic_id, status, body, note, created_at, seed)) = row else {
                anyhow::bail!("意見が見つかりません: {id}");
            };
            let votes: Vec<(i64, i64)> = sqlx::query_as(
                "SELECT vote_value, COUNT(*) FROM votes WHERE statement_id = $1 GROUP BY vote_value ORDER BY vote_value DESC",
            )
            .bind(id)
            .fetch_all(pool)
            .await?;
            let count = |v: i64| votes.iter().find(|(x, _)| *x == v).map_or(0, |(_, n)| *n);
            // 投稿者が誰かは出さない（運営のコマンドでも、最初の意見か市民の投稿かだけ）
            println!(
                "{id}  [{status}]  {created_at}  topic={topic_id}  {}",
                if seed {
                    "最初の意見"
                } else {
                    "市民の投稿"
                }
            );
            println!("  {body}");
            println!("  理由: {}", note.as_deref().unwrap_or("-"));
            println!(
                "  投票: 賛成 {}・反対 {}・パス {}",
                count(1),
                count(-1),
                count(0)
            );
        }
        Target::Tag => {
            return skip(
                "show",
                args.target,
                "タグは名前だけで、中身が無い（list tag で一覧を見る）",
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
            drafts::edit_topic(pool, id, args.title.as_deref(), args.description.as_deref())
                .await?;
            println!("{id}: 直しました");
        }
        Target::Statement => {
            let usage = "edit statement --id <id> --body <本文>";
            let id = required(&args.id, usage)?;
            drafts::edit_statement(pool, id, required(&args.body, usage)?).await?;
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
    }
    Ok(())
}

pub async fn add(ctx: &Ctx, args: AddArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    match args.target {
        Target::Statement => {
            let usage = "add statement --topic <topic_id> --body <本文>";
            let topic = required(&args.topic, usage)?;
            let sid = drafts::add_statement(pool, topic, required(&args.body, usage)?).await?;
            println!("{sid}: 足しました");
        }
        Target::Tag => {
            let usage = "add tag --topic <topic_id> --id <tag_id>";
            let topic = required(&args.topic, usage)?;
            let tag = required(&args.id, usage)?;
            drafts::set_tag(pool, topic, tag, true).await?;
            println!("{topic}: {tag} を付けました");
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
            drafts::set_tag(pool, topic, tag, false).await?;
            println!("{topic}: {tag} を外しました");
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
            "トピックは会議録から作る（import minutes。市民からの提案はフェーズ6のあと）"
        }
        Target::Source => "収集先は seed で管理する（data/seeds/010_municipality_*.sql）",
        Target::Statement | Target::Tag => "",
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
            if !budget::set_status(pool, &id, year, status).await? {
                anyhow::bail!("{slug} の {year}年度の予算はありません");
            }
            println!("{slug} {year}年度: {}", status.as_str());
        }
        Target::Topic => {
            let id = required(&args.id, &format!("{verb} topic --id <id>"))?;
            if !drafts::set_status(pool, id, status).await? {
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
        Target::Tag | Target::Source => skip(verb, args.target, "公開・取り下げの状態を持たない")?,
    }
    Ok(())
}
