// SPDX-License-Identifier: AGPL-3.0-only

//! show: 1件の中身を出す

use crate::ingest::{budget, collect};
use crate::interface::cli::{Ctx, municipality_id, required, skip};
use crate::records::{self, topics};

use super::{ShowArgs, Target};

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
