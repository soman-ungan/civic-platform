// SPDX-License-Identifier: AGPL-3.0-only

//! 予算の概要からの抽出の確かめる処理と、下書きの保存

mod common;

use std::collections::HashMap;

use backend::budget::{self, Input, Raw, RawItem, RawProject};

fn item(key: &str, value: i64) -> RawItem {
    RawItem {
        key: key.into(),
        value,
    }
}

fn project(name: &str, amount: i64, page: i64, tags: &[&str]) -> RawProject {
    RawProject {
        name: name.into(),
        amount_text: format_thousands(amount),
        amount,
        flag: "none".into(),
        page,
        plan_label: Some("政策１ 豊かな自然と調和する".into()),
        tags: tags.iter().map(|t| t.to_string()).collect(),
    }
}

fn format_thousands(n: i64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 実物の並びを模した、架空の概要のページ
fn pages() -> Vec<String> {
    vec![
        "表紙".into(),
        "令和８年度における主な事業\n公害防止対策費                     8,937   水質検査業務委託等\nとちぎの元気な森づくり市町村交付金\n                           11,914   里山林管理事業補助金等\n事業費\n".into(),
        "火葬場運営費                    105,325   指定管理料等\n".into(),
    ]
}

fn tags() -> HashMap<String, String> {
    HashMap::from([
        ("環境・ごみ".to_string(), "field-kankyou".to_string()),
        ("農林水産業".to_string(), "field-nourin".to_string()),
    ])
}

fn raw() -> Raw {
    Raw {
        general_account_total: 1000,
        revenue: vec![item("市税", 600), item("地方交付税", 400)],
        expenditure_purpose: vec![item("民生費", 700), item("教育費", 300)],
        expenditure_nature: vec![item("人件費", 500), item("投資的経費", 500)],
        projects: vec![
            project("公害防止対策費", 8937, 2, &["環境・ごみ"]),
            // 名前が折り返され、間に金額が入っている
            project(
                "とちぎの元気な森づくり市町村交付金事業費",
                11914,
                2,
                &["農林水産業", "架空の分野"],
            ),
            // ページの数え方が1つずれている
            project("火葬場運営費", 105325, 2, &[]),
            // 作られた事業（どのページにも無い）
            project("架空の事業費", 5000, 2, &[]),
        ],
    }
}

#[test]
fn keeps_only_projects_found_on_the_page() {
    let checked = budget::check(raw(), &pages(), &tags());
    let names: Vec<_> = checked.projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "公害防止対策費",
            "とちぎの元気な森づくり市町村交付金事業費",
            "火葬場運営費"
        ]
    );
    // ずれたページは、見つかったページに直す
    assert_eq!(checked.projects[2].page, 3);
    // 語彙に無い分野は使わない
    assert_eq!(checked.projects[1].tag_ids, ["field-nourin"]);
    assert!(checked.notes.iter().any(|n| n.contains("架空の事業費")));
    assert!(checked.notes.iter().any(|n| n.contains("架空の分野")));
    assert_eq!(checked.items.len(), 6);
}

#[test]
fn categories_that_do_not_add_up_are_dropped() {
    let mut r = raw();
    r.expenditure_purpose = vec![item("民生費", 700), item("教育費", 299)];
    r.expenditure_nature = vec![item("人件費", 500), item("架空の性質", 500)];
    let checked = budget::check(r, &pages(), &tags());
    // 目的別は合計が合わないので捨てる
    assert!(
        !checked
            .items
            .iter()
            .any(|(c, _, _)| *c == "expenditure_purpose")
    );
    // 性質別の一覧に無い費目は「その他」にまとめる（合計は合う）
    assert!(
        checked
            .items
            .contains(&("expenditure_nature", "その他".to_string(), 500))
    );
    assert!(checked.notes.iter().any(|n| n.contains("合わない")));
    assert!(checked.notes.iter().any(|n| n.contains("架空の性質")));
}

#[test]
fn amount_text_must_match_the_value() {
    let mut r = raw();
    r.projects = vec![RawProject {
        amount_text: "8,937".into(),
        amount: 8_937_000,
        ..project("公害防止対策費", 8937, 2, &[])
    }];
    let checked = budget::check(r, &pages(), &tags());
    assert!(checked.projects.is_empty());
}

#[test]
fn revenue_keys_follow_the_settlement_card() {
    let mut r = raw();
    r.revenue = vec![
        item("市税", 400),
        item("県支出金", 200),
        item("使用料及び手数料", 100),
        // 対応の無い名前は残す
        item("自治体独自の収入", 50),
        // 名前をそろえて同じになったら足す
        item("地方税", 250),
    ];
    let checked = budget::check(r, &pages(), &tags());
    let revenue: Vec<_> = checked
        .items
        .iter()
        .filter(|(c, _, _)| *c == "revenue")
        .map(|(_, k, v)| (k.as_str(), *v))
        .collect();
    assert_eq!(
        revenue,
        [
            ("地方税", 650),
            ("都道府県支出金", 200),
            ("使用料・手数料", 100),
            ("自治体独自の収入", 50),
        ]
    );
    assert!(
        checked
            .notes
            .iter()
            .any(|n| n.contains("「市税」は決算カードの名前「地方税」"))
    );
}

#[test]
fn same_project_on_a_detail_page_is_dropped() {
    let mut pages = pages();
    // 後ろの、重点事業を1件ずつ説明するページ
    pages.push("１ 公害防止対策事業費\n事業費 8,937 千円\n".into());
    pages.push("歴史民俗資料館運営費 8,937\n".into());
    let mut r = raw();
    r.projects = vec![
        project("公害防止対策費", 8937, 2, &["環境・ごみ"]),
        project("公害防止対策事業費", 8937, 4, &["環境・ごみ"]),
        // 金額が同じでも、分野が違えば別の事業
        project("歴史民俗資料館運営費", 8937, 5, &["農林水産業"]),
    ];
    let checked = budget::check(r, &pages, &tags());
    let names: Vec<_> = checked.projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["公害防止対策費", "歴史民俗資料館運営費"]);
    assert!(
        checked
            .notes
            .iter()
            .any(|n| n.contains("同じ事業とみなして"))
    );
}

#[tokio::test]
async fn draft_is_saved_replaced_and_published() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    sqlx::raw_sql(
        "INSERT INTO tags (id, kind, name) VALUES ('field-kankyou', 'field', '環境・ごみ'), ('field-nourin', 'field', '農林水産業');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let input = Input {
        municipality_id: "092100".into(),
        fiscal_year: 2026,
        url: Some("https://example.jp/R8yosangaiyou.pdf".into()),
        collector_source_id: None,
        pdf: b"%PDF-1.7 fake".to_vec(),
    };
    let checked = budget::check(raw(), &pages(), &tags());
    budget::save_draft(&pool, &input, &checked, false, "manual")
        .await
        .unwrap();

    let (status, loaded, url) = budget::load(&pool, "092100", 2026).await.unwrap().unwrap();
    assert_eq!(status, "draft");
    assert_eq!(loaded.total, 1000);
    assert_eq!(loaded.projects.len(), 3);
    assert_eq!(loaded.projects[1].tag_ids, ["field-nourin"]);
    assert_eq!(url.as_deref(), Some("https://example.jp/R8yosangaiyou.pdf"));

    // 下書きは置き換えられる。公開済みは --again が要る
    budget::save_draft(&pool, &input, &checked, false, "manual")
        .await
        .unwrap();
    assert!(
        budget::set_status(
            &pool,
            "092100",
            2026,
            backend::status::Publication::Published
        )
        .await
        .unwrap()
    );
    assert!(
        budget::save_draft(&pool, &input, &checked, false, "manual")
            .await
            .is_err()
    );
    budget::save_draft(&pool, &input, &checked, true, "manual")
        .await
        .unwrap();
    let (status, _, _) = budget::load(&pool, "092100", 2026).await.unwrap().unwrap();
    assert_eq!(status, "draft");
    // 別の自治体には入っていない
    assert!(budget::load(&pool, "092134", 2026).await.unwrap().is_none());
}

/// 大田原市の令和8年度の概要の実物と、そのときの AI の応答（data/private/ に保存したもの）で確かめる
#[test]
#[ignore = "data/private/budget/ に実物の PDF と AI の応答が要る"]
fn real_otawara_r8() {
    check_real(
        "../data/private/budget/R8yosangaiyou.pdf",
        "../data/private/budget/r8-extraction.json",
    );
}

/// 那須町の令和8年度の概要（ほかの自治体でも使えるかの確かめ）
#[test]
#[ignore = "data/private/budget/neighbors/ に実物の PDF と AI の応答が要る"]
fn real_nasu_r8() {
    check_real(
        "../data/private/budget/neighbors/nasu-R8gaiyou.pdf",
        "../data/private/budget/neighbors/nasu-r8-extraction.json",
    );
}

/// 那須烏山市の令和8年度の概要（性質別に「投資・出資金」「貸付金」が分かれている）
#[test]
#[ignore = "data/private/budget/neighbors/ に実物の PDF と AI の応答が要る"]
fn real_nasukarasuyama_r8() {
    check_real(
        "../data/private/budget/neighbors/nasukarasuyama-R8gaiyou.pdf",
        "../data/private/budget/neighbors/nasukarasuyama-r8-extraction.json",
    );
}

/// 保存してある実物の応答を、共同作業の出力（manual）として取り込むと、API の時と同じ結果になる
#[tokio::test]
#[ignore = "data/private/budget/ に実物の PDF と AI の応答が要る"]
async fn real_manual_extraction_matches_check() {
    let (Ok(pdf), Ok(json)) = (
        std::fs::read("../data/private/budget/R8yosangaiyou.pdf"),
        std::fs::read_to_string("../data/private/budget/r8-extraction.json"),
    ) else {
        return;
    };
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    sqlx::raw_sql(include_str!("../../data/seeds/020_field_tags.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let input = Input {
        municipality_id: "092100".into(),
        fiscal_year: 2026,
        url: None,
        collector_source_id: None,
        pdf,
    };
    let checked = budget::extract(
        &pool,
        &backend::extraction::Extraction::Manual(value.clone()),
        &input,
    )
    .await
    .unwrap();
    let pages = budget::pdftext::page_texts(&input.pdf).unwrap();
    let tags: HashMap<String, String> =
        sqlx::query_as("SELECT name, id FROM tags WHERE kind = 'field'")
            .fetch_all(&pool)
            .await
            .unwrap()
            .into_iter()
            .collect();
    let direct = budget::check(serde_json::from_value(value).unwrap(), &pages, &tags);
    assert_eq!(checked, direct);
    println!("{} 件", checked.projects.len());
}

fn check_real(pdf: &str, json: &str) {
    let (Ok(pdf), Ok(json)) = (std::fs::read(pdf), std::fs::read_to_string(json)) else {
        return;
    };
    let raw: Raw = serde_json::from_str(&json).unwrap();
    let total = raw.projects.len();
    let tags: HashMap<String, String> = raw
        .projects
        .iter()
        .flat_map(|p| p.tags.clone())
        .map(|t| (t.clone(), t))
        .collect();
    let pages = budget::pdftext::page_texts(&pdf).unwrap();
    let checked = budget::check(raw, &pages, &tags);
    for n in &checked.notes {
        println!("{n}");
    }
    println!("{} / {total}", checked.projects.len());
    assert_eq!(checked.projects.len(), total);
    for category in ["revenue", "expenditure_purpose", "expenditure_nature"] {
        assert!(
            checked.items.iter().any(|(c, _, _)| *c == category),
            "{category} が無い"
        );
    }
}
