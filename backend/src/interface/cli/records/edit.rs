// SPDX-License-Identifier: AGPL-3.0-only

//! edit / add / remove: 直す・足す・外す（その対象に要らないものは「何もしません」）

use crate::ingest::collect;
use crate::interface::cli::{Ctx, municipality_id, required, skip};
use crate::process::{self, moderation};
use crate::records::{accounts, publishing, topics};

use super::{AddArgs, EditArgs, RemoveArgs, Target};

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
        Target::Source => {
            // 収集先を足す・外すのは seed で。直せるのは、どの年度から取るかだけ
            let usage = "edit source --id <id> --from-year <年度>";
            let id = required(&args.id, usage)?;
            let year = *required(&args.from_year, usage)?;
            if !collect::set_from_fiscal_year(pool, id, year).await? {
                anyhow::bail!("収集先が見つかりません: {id}（list source で見られる）");
            }
            println!("{id}: {year}年度から取ります");
        }
        Target::Operator => skip(
            "edit",
            args.target,
            "運営者は足すか外すだけ（add / remove operator）",
        )?,
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
        Target::Topic => "トピックは会議録（import minutes）か、市民の提案（画面）から作る",
        Target::Source => "収集先は seed で管理する（data/seeds/010_municipality_*.sql）",
        Target::Similar => "似ているかは計算するだけで、保存しない（list similar で見る）",
        Target::Statement | Target::Tag | Target::Operator | Target::Queue | Target::Slot => "",
    }
}
