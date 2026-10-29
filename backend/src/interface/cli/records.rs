// SPDX-License-Identifier: AGPL-3.0-only

//! 段階2: DB を見る・直す（list / show / edit / add / remove / publish / archive）。
//! 中身は `records`（収集先は `ingest::collect`）。ここでは引数を読み、表示だけをする

use clap::{Args, ValueEnum};

use crate::ingest::{budget, collect};
use crate::process::{moderation, progress};
use crate::records::{self, accounts, topics};
use crate::shared::status::Publication;

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
    /// 意見の id（statement）・タグの id（tag）・運営者のアカウントの id（operator。list operator で見られる）
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
                    "{}  {}  [{}]  {}",
                    d.id, d.created_at, d.municipality_id, d.title
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
                    "{}  {}  {}  [{}]  {}",
                    r.id, r.created_at, r.slug, r.status, r.title
                );
            }
        }
        Target::Statement => list_statements(ctx, &args).await?,
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
    }
    Ok(())
}

pub async fn add(ctx: &Ctx, args: AddArgs) -> anyhow::Result<()> {
    let pool = &ctx.pool;
    match args.target {
        Target::Statement => {
            let usage = "add statement --topic <topic_id> --body <本文>";
            let topic = required(&args.topic, usage)?;
            let sid = topics::add_statement(pool, topic, required(&args.body, usage)?).await?;
            println!("{sid}: 足しました");
        }
        Target::Tag => {
            let usage = "add tag --topic <topic_id> --id <tag_id>";
            let topic = required(&args.topic, usage)?;
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
        Target::Statement | Target::Tag | Target::Operator => "",
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
    }
    Ok(())
}
