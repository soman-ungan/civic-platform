// SPDX-License-Identifier: AGPL-3.0-only

use std::path::Path;

use backend::llm::Llm;
use backend::llm::client::AnthropicClient;
use backend::moderation::{self, Moderation};
use backend::tagging::{self, Tagger};
use backend::{
    AppState, analysis, app, budget, collect, config::Config, db, drafts, eval, minutes, seed,
    status::Publication,
};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ルートの .env を親ディレクトリまで遡って読む（無くてもよい）
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env()?;
    let pool = db::connect(&config.database_url).await?;
    db::MIGRATOR.run(&pool).await?;
    tracing::info!("migrations applied");

    // `cargo run -- seed`: マスタ seed を投入して終了する
    // `cargo run -- seed --dev`: 続けて開発用サンプルデータも投入する（本番では使わない）
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("seed") {
        let mut dirs = vec![config.seeds_dir.as_str()];
        if args.iter().any(|a| a == "--dev") {
            dirs.push(config.dev_seeds_dir.as_str());
        }
        for dir in dirs {
            let files = seed::run(&pool, Path::new(dir)).await?;
            tracing::info!(dir, count = files.len(), "seeding finished");
        }
        return Ok(());
    }

    // `cargo run -- analyze [--topic <id>] [--force]`: 分析を更新して終了する（開発用）
    if args.first().map(String::as_str) == Some("analyze") {
        let force = args.iter().any(|a| a == "--force");
        let topic = args
            .iter()
            .position(|a| a == "--topic")
            .and_then(|i| args.get(i + 1));
        match topic {
            Some(topic_id) => {
                let outcome = analysis::store::run_topic(&pool, topic_id, force).await?;
                tracing::info!(topic_id, ?outcome, "analysis finished");
            }
            None => {
                let n = analysis::store::run_all(&pool, force).await?;
                tracing::info!(topics = n, "analysis finished");
            }
        }
        return Ok(());
    }

    // `cargo run -- topics drafts | show <id> | publish <id> | archive <id>`: 下書きの確認と公開
    if args.first().map(String::as_str) == Some("topics") {
        topics_command(&pool, &args[1..]).await?;
        return Ok(());
    }

    // API キーがあれば AI でモデレーションする（無ければ全件承認）
    let llm = match &config.anthropic_api_key {
        Some(key) => Some(
            Llm::anthropic(
                AnthropicClient::new(key.clone())?,
                config.llm_daily_budget_usd,
            )
            .with_extraction_budget(config.llm_extraction_daily_budget_usd),
        ),
        None => None,
    };
    let moderation = match &llm {
        Some(llm) => Moderation::Llm {
            llm: llm.clone(),
            model: config.moderation_model.clone(),
        },
        None => Moderation::AutoApprove,
    };

    let tagger = llm.as_ref().map(|llm| Tagger {
        llm: llm.clone(),
        model: config.tagging_model.clone(),
    });

    // `cargo run -- tag [--topic <id>] [--force]`: トピックに自動でタグを付けて終了する
    if args.first().map(String::as_str) == Some("tag") {
        let Some(tagger) = &tagger else {
            anyhow::bail!("ANTHROPIC_API_KEY が設定されていないため、タグ付けできません");
        };
        let force = args.iter().any(|a| a == "--force");
        let topic = args
            .iter()
            .position(|a| a == "--topic")
            .and_then(|i| args.get(i + 1));
        match topic {
            Some(topic_id) => {
                let outcome = tagging::tag_topic(&pool, tagger, topic_id, force).await?;
                println!("{topic_id}: {outcome:?}");
            }
            None => {
                let n = tagging::run_untagged(&pool, tagger).await?;
                println!("{n} 件のトピックをタグ付けしました");
            }
        }
        return Ok(());
    }

    // `cargo run -- collect list | run [--source <id>] [--year <n>] [--dry-run]`: 行政資料の収集
    // （予算の概要は AI で読むので、ANTHROPIC_API_KEY が要る。費用がかかる）
    if args.first().map(String::as_str) == Some("collect") {
        return collect_command(&pool, &config, llm.as_ref(), &args[1..]).await;
    }

    // `cargo run -- budget list | show <slug> <year> | publish <slug> <year> | archive <slug> <year>`:
    // 予算の下書きの確認と公開
    if args.first().map(String::as_str) == Some("budget") {
        return budget_command(&pool, &args[1..]).await;
    }

    // `cargo run -- import minutes --municipality <slug> --file <path> --title <会議名>
    //   --held-on <YYYY-MM-DD> [--url <URL>] [--dry-run] [--again] [--force]`: 会議録からトピック案を作る
    if args.first().map(String::as_str) == Some("import") {
        return import_command(&pool, &config, llm.as_ref(), &moderation, &args[1..]).await;
    }

    // `cargo run -- eval moderation|tagging [--model <id>] [--limit <n>] [--file <path>]`:
    // 評価データを実際の API で判定し、結果を出す（費用がかかる）
    if args.first().map(String::as_str) == Some("eval") {
        let Some(llm) = &llm else {
            anyhow::bail!("ANTHROPIC_API_KEY が設定されていないため、評価できません");
        };
        let option = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        let limit = option("--limit")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        match args.get(1).map(String::as_str) {
            Some("moderation") => {
                let model = option("--model").unwrap_or(config.moderation_model.clone());
                let file = option("--file").unwrap_or("../evals/moderation.jsonl".into());
                let cases: Vec<eval::ModerationCase> = eval::load(Path::new(&file))?;
                let cases = &cases[..cases.len().min(limit)];
                println!("モデレーションの評価: {model}、{} 件", cases.len());
                let moderation = Moderation::Llm {
                    llm: llm.clone(),
                    model,
                };
                let report = eval::run_moderation(&pool, &moderation, cases).await?;
                print!("{}", report.render());
            }
            Some("tagging") => {
                let model = option("--model").unwrap_or(config.tagging_model.clone());
                let file = option("--file").unwrap_or("../evals/tagging.jsonl".into());
                let cases: Vec<eval::TaggingCase> = eval::load(Path::new(&file))?;
                let cases = &cases[..cases.len().min(limit)];
                println!("タグ付けの評価: {model}、{} 件", cases.len());
                let tagger = Tagger {
                    llm: llm.clone(),
                    model,
                };
                let report = eval::run_tagging(&pool, &tagger, cases).await?;
                print!("{}", report.render());
            }
            _ => anyhow::bail!(
                "使い方: eval moderation|tagging [--model <id>] [--limit <n>] [--file <path>]"
            ),
        }
        return Ok(());
    }

    // `cargo run -- moderation list | approve <id> | reject <id> | retry`: 確認待ちの意見を扱う
    if args.first().map(String::as_str) == Some("moderation") {
        moderation_command(&pool, &moderation, &args[1..]).await?;
        return Ok(());
    }

    if config.analysis_interval_secs > 0 {
        analysis::store::spawn_worker(
            pool.clone(),
            std::time::Duration::from_secs(config.analysis_interval_secs),
        );
    }

    match &moderation {
        Moderation::Llm { model, .. } => {
            tracing::info!(model, "AI moderation enabled");
            moderation::spawn_retry_worker(
                pool.clone(),
                moderation.clone(),
                std::time::Duration::from_secs(60),
            );
        }
        Moderation::AutoApprove => {
            tracing::info!("ANTHROPIC_API_KEY が無いので、投稿は自動で承認し、タグ付けはしません");
        }
    }
    if let Some(tagger) = tagger {
        tagging::spawn_worker(pool.clone(), tagger, std::time::Duration::from_secs(60));
    }

    let state = AppState::new(pool, &config.session_secret, config.cookie_secure)?
        .with_moderation(moderation);
    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);

    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn moderation_command(
    pool: &sqlx::SqlitePool,
    moderation: &Moderation,
    args: &[String],
) -> anyhow::Result<()> {
    match (args.first().map(String::as_str), args.get(1)) {
        (Some("list") | None, _) => {
            let items = moderation::queue(pool).await?;
            if items.is_empty() {
                println!("確認待ちの意見はありません");
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
        }
        (Some(cmd @ ("approve" | "reject")), Some(id)) => {
            if moderation::decide(pool, id, cmd == "approve").await? {
                println!("{id}: {cmd}");
            } else {
                anyhow::bail!("意見が見つかりません: {id}");
            }
        }
        (Some("retry"), _) => {
            let n = moderation::retry_pending(pool, moderation).await?;
            println!("{n} 件を判定しました");
        }
        _ => anyhow::bail!("使い方: moderation [list | approve <id> | reject <id> | retry]"),
    }
    Ok(())
}

async fn import_command(
    pool: &sqlx::SqlitePool,
    config: &Config,
    llm: Option<&Llm>,
    moderation: &Moderation,
    args: &[String],
) -> anyhow::Result<()> {
    let option = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    if args.first().map(String::as_str) == Some("budget") {
        return import_budget(pool, config, llm, args).await;
    }
    let usage = "使い方: import minutes --municipality <slug> --file <path> --title <会議名> --held-on <YYYY-MM-DD> [--url <URL>] [--dry-run] [--again] [--force]\n       import budget --municipality <slug> --file <pdf> --year <n> [--url <URL>] [--dry-run] [--again]";
    if args.first().map(String::as_str) != Some("minutes") {
        anyhow::bail!(usage);
    }
    let (Some(slug), Some(file), Some(title), Some(held_on)) = (
        option("--municipality"),
        option("--file"),
        option("--title"),
        option("--held-on"),
    ) else {
        anyhow::bail!(usage);
    };
    let municipality_id: String =
        sqlx::query_scalar("SELECT id FROM municipalities WHERE slug = $1")
            .bind(&slug)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("自治体が見つかりません: {slug}"))?;
    let input = minutes::Input {
        municipality_id,
        title,
        held_on,
        url: option("--url"),
        text: std::fs::read_to_string(&file)?,
    };

    if args.iter().any(|a| a == "--dry-run") {
        let p = minutes::prepare(pool, &input).await?;
        println!(
            "発言 {} 件（{} 文字）",
            p.utterances.len(),
            input.text.chars().count()
        );
        println!(
            "議事日程: {}",
            if p.topical {
                "トピックを作る議事あり"
            } else {
                "形式的な議事だけ（取り込まない。取り込むときは --force）"
            }
        );
        for item in &p.agenda {
            println!("  {item}");
        }
        let mut speakers: Vec<(String, usize)> = Vec::new();
        for u in &p.utterances {
            let label = u.speaker_label();
            match speakers.iter_mut().find(|(l, _)| *l == label) {
                Some((_, n)) => *n += 1,
                None => speakers.push((label, 1)),
            }
        }
        for (label, n) in speakers {
            println!("  {label}: {n} 件");
        }
        if p.unmatched.is_empty() {
            println!("首長・議員はすべて名簿と対応付けられました");
        } else {
            println!("名簿に無い首長・議員（data/seeds/030_officials_*.sql に登録してください）:");
            for name in &p.unmatched {
                println!("  {name}");
            }
        }
        if let Some(id) = p.existing_document {
            println!("この会議録は取り込み済みです（document {id}）。作り直すときは --again");
        }
        return Ok(());
    }

    let Some(llm) = llm else {
        anyhow::bail!(
            "ANTHROPIC_API_KEY が設定されていないため、取り込めません（--dry-run は使えます）"
        );
    };
    let again = args.iter().any(|a| a == "--again");
    let force = args.iter().any(|a| a == "--force");
    match minutes::import(
        pool,
        llm,
        &config.extraction_model,
        moderation,
        &input,
        again,
        force,
    )
    .await?
    {
        minutes::Outcome::NotTopical(agenda) => {
            println!(
                "一般質問などトピックを作る議事が無いので、取り込みませんでした（取り込むときは --force）:"
            );
            for item in agenda {
                println!("  {item}");
            }
        }
        minutes::Outcome::NoUtterances => {
            anyhow::bail!("発言が見つかりません。会議録の見出しの形を確かめてください（--dry-run）")
        }
        minutes::Outcome::Unmatched(names) => {
            println!("名簿に無い首長・議員がいるので止めました。登録してからやり直してください:");
            for name in names {
                println!("  {name}");
            }
        }
        minutes::Outcome::Duplicate(id) => {
            println!("この会議録は取り込み済みです（document {id}）。作り直すときは --again");
        }
        minutes::Outcome::Imported(report) => {
            println!(
                "発言 {} 件から、下書きを {} 件作りました",
                report.utterances,
                report.topics.len()
            );
            for (id, title) in &report.topics {
                println!("  {id}  {title}");
            }
            if !report.notes.is_empty() {
                println!("捨てたもの:");
                for note in &report.notes {
                    println!("  {note}");
                }
            }
            println!("確認: cargo run -- topics show <id> → topics publish <id>");
        }
    }
    Ok(())
}

async fn collect_command(
    pool: &sqlx::SqlitePool,
    config: &Config,
    llm: Option<&Llm>,
    args: &[String],
) -> anyhow::Result<()> {
    let option = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let sources = collect::sources(pool).await?;
    match args.first().map(String::as_str) {
        Some("list") | None => {
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
        Some("run") => {
            let Some(contact) = &config.collector_contact else {
                anyhow::bail!(
                    "COLLECTOR_CONTACT が設定されていないため、収集できません（User-Agent に入れる運営者の連絡先）"
                );
            };
            let options = collect::Options {
                dry_run: args.iter().any(|a| a == "--dry-run"),
                fiscal_year: option("--year").map(|y| y.parse()).transpose()?,
            };
            let only = option("--source");
            // AI で資料を読む収集先（予算の概要）は費用がかかるので、--source で選ぶか --ai を付けたときだけ動かす
            let with_ai = args.iter().any(|a| a == "--ai");
            let uses_ai = |s: &collect::Source| s.adapter == "html_link_list";
            let (targets, skipped): (Vec<_>, Vec<_>) = sources
                .into_iter()
                .filter(|s| match &only {
                    Some(id) => s.id == *id,
                    // 手動の取り込み（会議録など）は自動では取得しない
                    None => s.is_enabled && s.adapter != "manual_upload",
                })
                .partition(|s| only.is_some() || with_ai || !uses_ai(s));
            for s in &skipped {
                println!(
                    "{}: AI で資料を読む（費用がかかる）ので飛ばしました。動かすときは --source {} か --ai",
                    s.id, s.id
                );
            }
            if targets.is_empty() {
                anyhow::bail!("処理する収集先がありません（collect list で確かめてください）");
            }
            for source in targets {
                println!("== {} ({})", source.id, source.adapter);
                let ai = llm.map(|llm| collect::Ai {
                    llm,
                    model: &config.extraction_model,
                });
                let report = collect::run_source(pool, &source, contact, &options, ai).await?;
                for line in &report.lines {
                    println!("  {line}");
                }
                for warning in &report.warnings {
                    println!("  警告: {warning}");
                }
            }
        }
        _ => anyhow::bail!(
            "使い方: collect [list | run [--source <id>] [--year <n>] [--dry-run] [--ai]]"
        ),
    }
    Ok(())
}

/// `import budget`: 手で取った当初予算の概要の PDF を取り込む（ほかの自治体の試しにも使う）。
/// AI で読むので費用がかかる。`--dry-run` は読んで確かめた結果を出すだけで、保存しない
async fn import_budget(
    pool: &sqlx::SqlitePool,
    config: &Config,
    llm: Option<&Llm>,
    args: &[String],
) -> anyhow::Result<()> {
    let option = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let (Some(slug), Some(file), Some(year)) =
        (option("--municipality"), option("--file"), option("--year"))
    else {
        anyhow::bail!(
            "使い方: import budget --municipality <slug> --file <pdf> --year <n> [--url <URL>] [--dry-run] [--again]"
        );
    };
    let Some(llm) = llm else {
        anyhow::bail!("ANTHROPIC_API_KEY が設定されていないため、予算の概要を読めません");
    };
    let municipality_id = municipality_id(pool, &slug).await?;
    let input = budget::Input {
        municipality_id,
        fiscal_year: year.parse()?,
        url: option("--url"),
        collector_source_id: None,
        pdf: std::fs::read(&file)?,
    };
    let checked = budget::extract(pool, llm, &config.extraction_model, &input).await?;
    print!("{}", budget::render(&checked, input.url.as_deref()));
    if args.iter().any(|a| a == "--dry-run") {
        println!("（--dry-run のため保存していません）");
        return Ok(());
    }
    let again = args.iter().any(|a| a == "--again");
    budget::save_draft(pool, &input, &checked, again).await?;
    println!("下書きを作りました。確認: budget show {slug} {year} → budget publish {slug} {year}");
    Ok(())
}

async fn municipality_id(pool: &sqlx::SqlitePool, slug: &str) -> anyhow::Result<String> {
    sqlx::query_scalar("SELECT id FROM municipalities WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("自治体が見つかりません: {slug}"))
}

async fn budget_command(pool: &sqlx::SqlitePool, args: &[String]) -> anyhow::Result<()> {
    let usage = "使い方: budget [list | show <slug> <year> | publish <slug> <year> | archive <slug> <year>]";
    match (args.first().map(String::as_str), args.get(1), args.get(2)) {
        (Some("list") | None, _, _) => {
            let rows = budget::list(pool).await?;
            if rows.is_empty() {
                println!("予算はまだありません");
            }
            for (slug, year, status, total) in rows {
                println!("{slug}  {year}年度  [{status}]  歳出 {total} 千円");
            }
        }
        (Some("show"), Some(slug), Some(year)) => {
            let id = municipality_id(pool, slug).await?;
            let Some((status, checked, url)) = budget::load(pool, &id, year.parse()?).await? else {
                anyhow::bail!("{slug} の {year}年度の予算はありません");
            };
            println!("[{status}]");
            print!("{}", budget::render(&checked, url.as_deref()));
        }
        (Some(cmd @ ("publish" | "archive")), Some(slug), Some(year)) => {
            let id = municipality_id(pool, slug).await?;
            let status = if cmd == "publish" {
                Publication::Published
            } else {
                Publication::Archived
            };
            if !budget::set_status(pool, &id, year.parse()?, status).await? {
                anyhow::bail!("{slug} の {year}年度の予算はありません");
            }
            println!("{slug} {year}年度: {}", status.as_str());
        }
        _ => anyhow::bail!(usage),
    }
    Ok(())
}

async fn topics_command(pool: &sqlx::SqlitePool, args: &[String]) -> anyhow::Result<()> {
    match (args.first().map(String::as_str), args.get(1)) {
        (Some("drafts") | None, _) => {
            let items = drafts::list(pool).await?;
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
        (Some("show"), Some(id)) => match drafts::show(pool, id).await? {
            Some(detail) => print!("{}", detail.render()),
            None => anyhow::bail!("トピックが見つかりません: {id}"),
        },
        (Some(cmd @ ("publish" | "archive")), Some(id)) => {
            let status = if cmd == "publish" {
                Publication::Published
            } else {
                Publication::Archived
            };
            if drafts::set_status(pool, id, status).await? {
                println!("{id}: {}", status.as_str());
            } else {
                anyhow::bail!("トピックが見つかりません: {id}");
            }
        }
        // topics edit <id> [--title <タイトル>] [--description <説明>]
        (Some("edit"), Some(id)) => {
            let option = |name: &str| {
                args.iter()
                    .position(|a| a == name)
                    .and_then(|i| args.get(i + 1))
                    .map(String::as_str)
            };
            let (title, description) = (option("--title"), option("--description"));
            if title.is_none() && description.is_none() {
                anyhow::bail!(
                    "使い方: topics edit <id> [--title <タイトル>] [--description <説明>]"
                );
            }
            drafts::edit_topic(pool, id, title, description).await?;
            println!("{id}: 直しました");
        }
        // topics statement add <topic_id> <本文> | edit <意見の ID> <本文> | remove <意見の ID>
        (Some("statement"), Some(cmd)) => match (cmd.as_str(), args.get(2), args.get(3)) {
            ("add", Some(topic_id), Some(body)) => {
                let sid = drafts::add_statement(pool, topic_id, body).await?;
                println!("{sid}: 足しました");
            }
            ("edit", Some(sid), Some(body)) => {
                drafts::edit_statement(pool, sid, body).await?;
                println!("{sid}: 直しました");
            }
            ("remove", Some(sid), None) => {
                if !moderation::decide(pool, sid, false).await? {
                    anyhow::bail!("意見が見つかりません: {sid}");
                }
                println!("{sid}: 取り下げました");
            }
            _ => anyhow::bail!(
                "使い方: topics statement add <topic_id> <本文> | edit <意見の ID> <本文> | remove <意見の ID>"
            ),
        },
        // topics tag add|remove <topic_id> <tag_id>
        (Some("tag"), Some(cmd)) if cmd == "add" || cmd == "remove" => {
            let (Some(topic_id), Some(tag_id)) = (args.get(2), args.get(3)) else {
                anyhow::bail!("使い方: topics tag add|remove <topic_id> <tag_id>");
            };
            drafts::set_tag(pool, topic_id, tag_id, cmd == "add").await?;
            println!(
                "{topic_id}: {tag_id} を{}",
                if cmd == "add" {
                    "付けました"
                } else {
                    "外しました"
                }
            );
        }
        _ => anyhow::bail!(
            "使い方: topics [drafts | show <id> | publish <id> | archive <id> | edit <id> ... | statement ... | tag ...]"
        ),
    }
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
    tracing::info!("shutting down");
}
