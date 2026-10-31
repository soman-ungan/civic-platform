// SPDX-License-Identifier: AGPL-3.0-only

//! publish / archive: 公開する・取り下げる

use crate::interface::cli::{Ctx, municipality_id, required, skip};
use crate::process::progress;
use crate::records::{self, topics};
use crate::shared::status::Publication;

use super::{PublishArgs, Target};

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
