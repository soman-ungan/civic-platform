// SPDX-License-Identifier: AGPL-3.0-only

//! まちのお金の API（決算カード）

mod common;

use axum::http::StatusCode;
use backend::AppState;
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
    assert_eq!(j["years"], serde_json::json!([2022, 2023]));
    assert_eq!(j["fiscal_year"], 2023);
    assert_eq!(j["population"], 100);
    assert_eq!(j["expenditure_total"], 1000);
    // 750 千円 ÷ 100 人 = 7,500 円
    assert_eq!(j["expenditure_purpose"][0]["key"], "民生費");
    assert_eq!(j["expenditure_purpose"][0]["per_capita"], 7500);
    assert_eq!(j["expenditure_purpose"][0]["share"], 0.75);
    // 「-」は null のまま
    assert!(j["expenditure_purpose"][2]["value"].is_null());
    assert_eq!(j["indicators"][0]["value"], 0.63);
    assert_eq!(j["source"]["page_url"], "https://example.jp/card-24.html");
    assert_eq!(j["trend"][0]["expenditure_purpose"]["民生費"], 600);
    assert_eq!(j["trend"][0]["population"], 110);
}

#[tokio::test]
async fn other_years_and_municipalities() {
    let (pool, state) = setup().await;
    let res = get(&state, "/api/municipalities/otawara/finances?year=2022").await;
    assert_eq!(res.json["expenditure_purpose"][0]["value"], 600);
    let res = get(&state, "/api/municipalities/otawara/finances?year=2010").await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    // 公開前の自治体は 404。公開すると、その自治体の数字だけが返る
    let res = get(&state, "/api/municipalities/nasushiobara/finances").await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    common::activate_second_municipality(&pool).await;
    let res = get(&state, "/api/municipalities/nasushiobara/finances").await;
    assert_eq!(res.json["expenditure_total"], 99000);
    assert_eq!(res.json["years"], serde_json::json!([2023]));
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
