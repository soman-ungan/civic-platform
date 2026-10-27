// SPDX-License-Identifier: AGPL-3.0-only

//! 決算カードの読み取り。実物の Excel は data/private/（git の管理外）にあるときだけ確かめる

use backend::collect::settlement_card::{
    Category, Cell, Grid, Unit, excel_links, find_card, fiscal_year_of, parse_card, year_pages,
};

fn private(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("../data/private/soumu/{name}")).ok()
}

fn value(
    card: &backend::collect::settlement_card::Card,
    category: Category,
    key: &str,
) -> Option<i64> {
    card.items
        .iter()
        .find(|i| i.category == category && i.key == key)
        .and_then(|i| i.value)
}

#[test]
#[ignore = "data/private/soumu/ に実物の Excel が要る"]
fn real_card_r5() {
    let Some(bytes) = private("tochigi-r5.xlsx") else {
        return;
    };
    let card = find_card(&bytes, "092100")
        .unwrap()
        .expect("大田原市のシート");
    assert_eq!(
        (card.prefecture.as_str(), card.name.as_str()),
        ("栃木県", "大田原市")
    );
    assert_eq!(card.fiscal_year, 2023);
    assert_eq!(card.population, Some(68873));
    assert_eq!(card.total(Category::Revenue), Some(35317540));
    assert_eq!(card.total(Category::ExpenditurePurpose), Some(33694873));
    assert_eq!(card.total(Category::ExpenditureNature), Some(33694873));
    assert_eq!(
        value(&card, Category::ExpenditurePurpose, "民生費"),
        Some(12751516)
    );
    assert_eq!(
        value(&card, Category::ExpenditureNature, "公債費"),
        Some(3440220)
    );
    assert_eq!(value(&card, Category::Revenue, "財産収入"), Some(83673));
    // 振り仮名が混ざらないこと
    assert_eq!(value(&card, Category::Revenue, "利子割交付金"), Some(2188));
    assert_eq!(value(&card, Category::Indicator, "財政力指数"), Some(630));
    assert_eq!(
        value(&card, Category::Indicator, "経常収支比率"),
        Some(97700)
    );
    assert_eq!(
        value(&card, Category::Indicator, "実質公債費比率"),
        Some(6700)
    );
    assert_eq!(
        value(&card, Category::Indicator, "将来負担比率"),
        Some(21000)
    );
    assert_eq!(
        value(&card, Category::Indicator, "地方債現在高"),
        Some(25455249)
    );
    for item in &card.items {
        println!("{:?} {} {:?}", item.category, item.key, item.value);
    }
    // 別の自治体を取り違えない
    let other = find_card(&bytes, "092134")
        .unwrap()
        .expect("那須塩原市のシート");
    assert_eq!(other.name, "那須塩原市");
}

#[test]
#[ignore = "data/private/soumu/ に実物の Excel が要る"]
fn real_card_h27() {
    let Some(bytes) = private("tochigi-h27.xlsx") else {
        return;
    };
    let card = find_card(&bytes, "092100")
        .unwrap()
        .expect("大田原市のシート");
    assert_eq!(card.fiscal_year, 2015);
    assert_eq!(card.population, Some(72892));
    assert_eq!(card.total(Category::Revenue), Some(35565299));
    assert_eq!(card.total(Category::ExpenditurePurpose), Some(34287172));
    assert_eq!(
        value(&card, Category::Indicator, "経常収支比率"),
        Some(94600)
    );
}

#[test]
#[ignore = "data/private/soumu/ に実物の HTML が要る"]
fn real_pages() {
    let (Some(index), Some(year)) = (private("card.html"), private("card-24.html")) else {
        return;
    };
    let base = reqwest::Url::parse("https://www.soumu.go.jp/iken/zaisei/card.html").unwrap();
    let pages = year_pages(
        &backend::collect::settlement_card::decode_html(&index),
        &base,
    );
    assert!(pages.contains(&(
        2023,
        "https://www.soumu.go.jp/iken/zaisei/card-24.html".into()
    )));
    assert!(pages.contains(&(
        2019,
        "https://www.soumu.go.jp/iken/zaisei/card-20.html".into()
    )));
    let html = backend::collect::settlement_card::decode_html(&year);
    assert_eq!(
        excel_links(&html, "栃木県", &base),
        ["https://www.soumu.go.jp/main_content/000998938.xlsx"]
    );
    assert_eq!(excel_links(&html, "北海道", &base).len(), 2);
    assert_eq!(excel_links(&html, "京都府", &base).len(), 1);
}

#[test]
fn era_years() {
    assert_eq!(fiscal_year_of("令和5年度"), Some(2023));
    assert_eq!(fiscal_year_of("令和元年度"), Some(2019));
    assert_eq!(fiscal_year_of("平成27年度"), Some(2015));
    assert_eq!(fiscal_year_of("令和5年度(千円)"), None);
}

/// 実物と同じ並びの、小さな決算カード（架空の値）
fn sample(code_muni: &str, name: &str, revenue_total: f64) -> Grid {
    let t = |r, c, s: &str| (r, c, Cell::Text(s.into()));
    let n = |r, c, v: f64| (r, c, Cell::Number(v));
    Grid::new([
        t(1, 3, "令和５年度"),
        t(1, 41, "住民基本台帳人口"),
        n(3, 41, 1000.0),
        t(1, 77, "都道府県名"),
        t(1, 88, "団体名"),
        t(4, 77, "09"),
        t(4, 88, code_muni),
        t(6, 77, "栃木県"),
        t(6, 88, name),
        // 歳入
        t(6, 5, "歳入の状況"),
        t(10, 1, "地方税"),
        n(10, 12, 600.0),
        t(11, 1, "地方交付税"),
        n(11, 12, 300.0),
        t(12, 1, "内訳"),
        t(12, 2, "普通交付税"),
        n(12, 12, 250.0),
        t(13, 1, "(一般財源計)"),
        n(13, 12, 900.0),
        t(14, 1, "地方債"),
        t(14, 12, "-"),
        t(15, 1, "財 産 収 入"),
        n(15, 12, 100.0),
        t(16, 1, "歳入合計"),
        n(16, 12, revenue_total),
        // 性質別・目的別
        t(48, 4, "性質別歳出の状況"),
        t(48, 51, "目的別歳出の状況"),
        t(50, 42, "経常収支比率"),
        t(51, 1, "人件費"),
        n(51, 12, 400.0),
        n(51, 42, 24.1),
        t(51, 48, "民生費"),
        n(51, 57, 500.0),
        t(52, 1, "公債費"),
        n(52, 12, 500.0),
        t(52, 48, "公債費"),
        n(52, 57, 400.0),
        t(52, 84, "財政力指数"),
        n(52, 96, 0.63),
        n(52, 104, 0.64),
        t(53, 84, "実質公債費比率"),
        t(53, 94, "(％)"),
        n(53, 96, 6.7),
        t(54, 1, "歳出合計"),
        n(54, 12, 900.0),
        t(54, 48, "歳出合計"),
        n(54, 57, 900.0),
        t(60, 33, "経 常 収 支 比 率"),
        n(61, 36, 97.7),
        n(61, 43, 98.5),
    ])
}

#[test]
fn parses_sections_by_heading() {
    let card = parse_card(&sample("2100", "大田原市", 1000.0)).unwrap();
    assert_eq!(card.code, "092100");
    assert_eq!(card.fiscal_year, 2023);
    assert_eq!(card.population, Some(1000));
    let keys = |c: Category| -> Vec<(String, Option<i64>)> {
        card.items
            .iter()
            .filter(|i| i.category == c)
            .map(|i| (i.key.clone(), i.value))
            .collect()
    };
    // 「内訳」と小計（計）は費目にしない。「-」は値なし。空白は除く
    assert_eq!(
        keys(Category::Revenue),
        [
            ("地方税".into(), Some(600)),
            ("地方交付税".into(), Some(300)),
            ("地方債".into(), None),
            ("財産収入".into(), Some(100)),
        ]
    );
    // 同じ「公債費」でも、性質別と目的別を取り違えない
    assert_eq!(
        keys(Category::ExpenditureNature),
        [("人件費".into(), Some(400)), ("公債費".into(), Some(500))]
    );
    assert_eq!(
        keys(Category::ExpenditurePurpose),
        [("民生費".into(), Some(500)), ("公債費".into(), Some(400))]
    );
    let indicator = |k: &str| {
        card.items
            .iter()
            .find(|i| i.category == Category::Indicator && i.key == k)
            .map(|i| (i.value, i.unit))
    };
    assert_eq!(indicator("財政力指数"), Some((Some(630), Unit::Milli)));
    assert_eq!(indicator("実質公債費比率"), Some((Some(6700), Unit::Milli)));
    assert_eq!(indicator("経常収支比率"), Some((Some(97700), Unit::Milli)));
}

#[test]
fn mismatched_totals_are_rejected() {
    let err = parse_card(&sample("2100", "大田原市", 999.0)).unwrap_err();
    assert!(err.contains("一致しません"), "{err}");
}
