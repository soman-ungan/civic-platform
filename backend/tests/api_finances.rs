// SPDX-License-Identifier: AGPL-3.0-only

//! まちのお金の API（決算カード）

mod common;

use axum::http::StatusCode;
use backend::interface::api::AppState;
use common::request;

async fn setup() -> (sqlx::SqlitePool, AppState) {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    // 2自治体・2年度。那須塩原市の値は、取り違えたら分かるように桁を変える
    sqlx::raw_sql(
        "
        INSERT INTO collector_sources (id, municipality_id, kind, adapter, base_url)
            VALUES ('src', '092100', 'settlement_card', 'soumu_settlement_card', 'https://example.jp/');
        INSERT INTO documents (id, municipality_id, kind, title, url, sha256, char_count, imported_at) VALUES
            ('d22', '092100', 'settlement_card', '市町村決算カード 2022年度（栃木県）', 'https://example.jp/r4.xlsx', 'h22', 1, '2026-10-27T00:00:00Z'),
            ('d23', '092100', 'settlement_card', '市町村決算カード 2023年度（栃木県）', 'https://example.jp/r5.xlsx', 'h23', 1, '2026-10-27T00:00:00Z'),
            ('n23', '092134', 'settlement_card', '市町村決算カード 2023年度（栃木県）', 'https://example.jp/r5.xlsx', 'h23', 1, '2026-10-27T00:00:00Z');
        INSERT INTO settlement_items (municipality_id, fiscal_year, category, item_key, position, value, unit, document_id) VALUES
            ('092100', 2022, 'expenditure_purpose', '民生費', 1, 600, 'thousand_yen', 'd22'),
            ('092100', 2022, 'expenditure_purpose', '教育費', 2, 400, 'thousand_yen', 'd22'),
            ('092100', 2023, 'revenue', '地方税', 1, 1000, 'thousand_yen', 'd23'),
            ('092100', 2023, 'expenditure_purpose', '民生費', 1, 750, 'thousand_yen', 'd23'),
            ('092100', 2023, 'expenditure_purpose', '教育費', 2, 250, 'thousand_yen', 'd23'),
            ('092100', 2023, 'expenditure_purpose', '諸支出金', 3, NULL, 'thousand_yen', 'd23'),
            ('092100', 2023, 'expenditure_nature', '人件費', 1, 1000, 'thousand_yen', 'd23'),
            ('092100', 2023, 'indicator', '財政力指数', 1, 630, 'milli', 'd23'),
            ('092134', 2023, 'expenditure_purpose', '民生費', 1, 99000, 'thousand_yen', 'n23');
        INSERT INTO municipality_stats (municipality_id, fiscal_year, population, source_url) VALUES
            ('092100', 2022, 110, 'https://example.jp/card-23.html'),
            ('092100', 2023, 100, 'https://example.jp/card-24.html'),
            ('092134', 2023, 50, 'https://example.jp/card-24.html');
        INSERT INTO tags (id, kind, name) VALUES ('field-fukushi', 'field', '福祉・介護');
        INSERT INTO settlement_item_tags (category, item_key, tag_id) VALUES
            ('expenditure_purpose', '民生費', 'field-fukushi');
        INSERT INTO topic_tags (topic_id, tag_id) VALUES ('t1', 'field-fukushi');
        ",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = common::test_state(pool.clone());
    (pool, state)
}

async fn get(state: &AppState, path: &str) -> common::TestResponse {
    request(state, "GET", path, None, None).await
}

#[tokio::test]
async fn latest_year_with_per_capita_and_share() {
    let (_pool, state) = setup().await;
    let res = get(&state, "/api/municipalities/otawara/finances").await;
    assert_eq!(res.status, StatusCode::OK);
    let j = &res.json;
    assert_eq!(
        j["years"],
        serde_json::json!([
            {"fiscal_year": 2022, "settlement": true, "budget": false},
            {"fiscal_year": 2023, "settlement": true, "budget": false},
        ])
    );
    assert_eq!(j["fiscal_year"], 2023);
    assert!(j["budget"].is_null());
    let s = &j["settlement"];
    assert_eq!(s["population"], 100);
    assert_eq!(s["expenditure_total"], 1000);
    // 750 千円 ÷ 100 人 = 7,500 円
    assert_eq!(s["expenditure_purpose"][0]["key"], "民生費");
    assert_eq!(s["expenditure_purpose"][0]["per_capita"], 7500);
    assert_eq!(s["expenditure_purpose"][0]["share"], 0.75);
    // 「-」は null のまま
    assert!(s["expenditure_purpose"][2]["value"].is_null());
    assert_eq!(s["indicators"][0]["value"], 0.63);
    assert_eq!(s["source"]["page_url"], "https://example.jp/card-24.html");
    assert_eq!(j["trend"][0]["expenditure_purpose"]["民生費"], 600);
    assert_eq!(j["trend"][0]["population"], 110);
}

#[tokio::test]
async fn other_years_and_municipalities() {
    let (pool, state) = setup().await;
    let res = get(&state, "/api/municipalities/otawara/finances?year=2022").await;
    assert_eq!(
        res.json["settlement"]["expenditure_purpose"][0]["value"],
        600
    );
    let res = get(&state, "/api/municipalities/otawara/finances?year=2010").await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    // 公開前の自治体は 404。公開すると、その自治体の数字だけが返る
    let res = get(&state, "/api/municipalities/nasushiobara/finances").await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    common::activate_second_municipality(&pool).await;
    let res = get(&state, "/api/municipalities/nasushiobara/finances").await;
    assert_eq!(res.json["settlement"]["expenditure_total"], 99000);
    assert_eq!(res.json["years"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn topic_detail_shows_related_items() {
    let (_pool, state) = setup().await;
    let res = get(&state, "/api/municipalities/otawara/topics/t1").await;
    assert_eq!(res.status, StatusCode::OK);
    let f = &res.json["finances"];
    assert_eq!(f["fiscal_year"], 2023);
    assert_eq!(f["items"].as_array().unwrap().len(), 1);
    assert_eq!(f["items"][0]["key"], "民生費");
    assert_eq!(f["items"][0]["per_capita"], 7500);
}

#[tokio::test]
async fn no_finances_without_data() {
    let pool = common::test_pool().await;
    common::seed_fixture(&pool).await;
    let state = common::test_state(pool);
    assert_eq!(
        get(&state, "/api/municipalities/otawara/finances")
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let res = get(&state, "/api/municipalities/otawara/topics/t1").await;
    assert!(res.json["finances"].is_null());
}

/// 2024年度の予算（2つの事業。1つは福祉の分野）を足す。`status` は draft か published
async fn add_budget(pool: &sqlx::SqlitePool, status: &str) {
    sqlx::raw_sql(&format!(
        "
        INSERT INTO documents (id, municipality_id, kind, title, url, sha256, char_count, imported_at) VALUES
            ('db24', '092100', 'budget', '当初予算の概要 2024年度', 'https://example.jp/R6gaiyou.pdf', 'hb24', 1, '2026-10-27T00:00:00Z');
        INSERT INTO budgets (id, municipality_id, fiscal_year, status, document_id, total, created_at) VALUES
            ('b24', '092100', 2024, '{status}', 'db24', 2000, '2026-10-27T00:00:00Z');
        INSERT INTO budget_items (budget_id, category, item_key, position, value) VALUES
            ('b24', 'expenditure_purpose', '民生費', 1, 1500),
            ('b24', 'expenditure_purpose', '教育費', 2, 500);
        INSERT INTO budget_projects (id, budget_id, position, name, amount, flag, plan_label, page) VALUES
            ('bp1', 'b24', 1, '子育て支援事業費', 300, 'new', '政策２ 子育て', 12),
            ('bp2', 'b24', 2, '道路維持費', 900, NULL, NULL, 14);
        INSERT INTO budget_project_tags (project_id, tag_id) VALUES ('bp1', 'field-fukushi');
        "
    ))
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn budget_years_use_the_latest_population() {
    let (pool, state) = setup().await;
    add_budget(&pool, "published").await;
    let res = get(&state, "/api/municipalities/otawara/finances").await;
    let j = &res.json;
    // 予算だけの年度が最新になる
    assert_eq!(j["fiscal_year"], 2024);
    assert_eq!(
        j["years"][2],
        serde_json::json!({"fiscal_year": 2024, "settlement": false, "budget": true})
    );
    assert!(j["settlement"].is_null());
    let b = &j["budget"];
    assert_eq!(b["total"], 2000);
    // 2024年度の人口は無いので、2023年度の人口（100人）で割る
    assert_eq!(b["population_year"], 2023);
    assert_eq!(b["expenditure_purpose"][0]["per_capita"], 15000);
    // 主な事業は金額の大きい順
    assert_eq!(b["projects"][0]["name"], "道路維持費");
    assert_eq!(b["projects"][1]["flag"], "new");
    assert_eq!(b["projects"][1]["tags"][0]["name"], "福祉・介護");
    assert_eq!(b["source"]["url"], "https://example.jp/R6gaiyou.pdf");
    // 推移の最後に、予算だけの年度が予算として入る（人口はそれ以前で最新）
    let trend = j["trend"].as_array().unwrap();
    assert_eq!(trend.len(), 3);
    assert_eq!(trend[1]["kind"], "settlement");
    assert_eq!(trend[2]["fiscal_year"], 2024);
    assert_eq!(trend[2]["kind"], "budget");
    assert_eq!(trend[2]["population"], 100);
    assert_eq!(trend[2]["expenditure_purpose"]["民生費"], 1500);
}

#[tokio::test]
async fn draft_budgets_are_hidden() {
    let (pool, state) = setup().await;
    add_budget(&pool, "draft").await;
    let res = get(&state, "/api/municipalities/otawara/finances").await;
    assert_eq!(res.json["fiscal_year"], 2023);
    assert!(res.json["budget"].is_null());
    assert_eq!(res.json["trend"].as_array().unwrap().len(), 2);
    let res = get(&state, "/api/municipalities/otawara/finances?year=2024").await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let res = get(&state, "/api/municipalities/otawara/topics/t1").await;
    assert!(res.json["finances"]["budget"].is_null());
}

#[tokio::test]
async fn topic_detail_shows_projects_of_its_field() {
    let (pool, state) = setup().await;
    add_budget(&pool, "published").await;
    let res = get(&state, "/api/municipalities/otawara/topics/t1").await;
    let b = &res.json["finances"]["budget"];
    assert_eq!(b["fiscal_year"], 2024);
    assert_eq!(b["project_count"], 1);
    assert_eq!(b["projects"][0]["name"], "子育て支援事業費");
}
