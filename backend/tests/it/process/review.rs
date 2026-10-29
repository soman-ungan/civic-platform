// SPDX-License-Identifier: AGPL-3.0-only

//! 意見の審査（ADR 0007）: 公人の氏名の確かめ、4つの観点のまとめ方、似た意見の案内、AI に送るもの。
//! 偽の LLM で確かめる（実際の API は呼ばない）

use std::sync::Arc;

use crate::common::{
    account_ready_to_post, request, review_reply, seed_fixture, test_pool, test_state,
};
use axum::http::StatusCode;
use backend::interface::api::AppState;
use backend::process::moderation::{self, Moderation};
use backend::shared::llm::{Fake, Llm};
use serde_json::{Value, json};
use sqlx::SqlitePool;

const T1: &str = "/api/municipalities/otawara/topics/t1";

/// t1 に説明と承認済みの意見2件、t2（別の自治体）に意見1件、大田原市の公人1人
async fn setup() -> (AppState, Arc<Fake>) {
    let pool = test_pool().await;
    seed_fixture(&pool).await;
    sqlx::raw_sql(
        "UPDATE topics SET description = 'バスの路線と本数の見直し' WHERE id = 't1';
         UPDATE statements SET status = 'approved', created_at = '2026-10-26T00:00:01Z' WHERE id = 's1';
         INSERT INTO statements (id, topic_id, body, status, is_seed, created_at) VALUES
            ('e1', 't1', '朝の通学の便を増やしてほしい', 'approved', TRUE, '2026-10-26T00:00:02Z'),
            ('x1', 't2', '別の自治体の意見', 'approved', TRUE, '2026-10-26T00:00:00Z');
         INSERT INTO public_officials (id, municipality_id, name, role, term_start) VALUES
            ('o1', '092100', '山田 一郎', 'mayor', '2022-03-20');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let (llm, fake) = Llm::fake(1.0);
    let state = test_state(pool).with_moderation(Moderation::Llm {
        llm,
        model: "claude-sonnet-5-5".into(),
    });
    (state, fake)
}

async fn post(state: &AppState, cookie: &str, body: &str) -> crate::common::TestResponse {
    request(
        state,
        "POST",
        &format!("{T1}/statements"),
        Some(cookie),
        Some(json!({ "body": body })),
    )
    .await
}

async fn count(pool: &SqlitePool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

fn reply(harm: &str, person: &str, votable: &str, duplicate_of: i64, suggestion: &str) -> Value {
    let mut v = review_reply(harm, person, votable, duplicate_of);
    v["suggestion"] = suggestion.into();
    v
}

#[tokio::test]
async fn official_names_are_rejected_without_ai() {
    let (state, fake) = setup().await;
    let cookie = account_ready_to_post(&state.pool, &state, "t1").await;
    // 空白の有無にかかわらず、氏名がそのまま含まれていれば AI に送らずに非公開
    let res = post(&state, &cookie, "山田一郎はいいやつだ").await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.json["status"], "rejected");
    assert_eq!(res.json["rejection"], "official_name");
    assert!(fake.requests().is_empty());

    // API キーの無い環境でも効く
    let pool = test_pool().await;
    seed_fixture(&pool).await;
    sqlx::query(
        "INSERT INTO public_officials (id, municipality_id, name, role, term_start)
         VALUES ('o1', '092100', '山田 一郎', 'mayor', '2022-03-20')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = test_state(pool);
    let cookie = account_ready_to_post(&state.pool, &state, "t1").await;
    let res = post(&state, &cookie, "山田 一郎 市長の方針に反対").await;
    assert_eq!(res.json["rejection"], "official_name");
}

#[tokio::test]
async fn aspects_are_combined_in_order() {
    let (state, fake) = setup().await;
    let cases = [
        // 非公開の理由は 害 → 人物 → 投票できない の順に選ぶ
        (
            reply("reject", "reject", "reject", 0, ""),
            "rejected",
            Some("harm"),
        ),
        (
            reply("ok", "reject", "reject", 0, "路線の見直しの方針に反対です"),
            "rejected",
            Some("person"),
        ),
        (
            reply("ok", "ok", "reject", 0, ""),
            "rejected",
            Some("not_votable"),
        ),
        // 非公開が迷いより先
        (
            reply("unsure", "reject", "ok", 0, ""),
            "rejected",
            Some("person"),
        ),
        (reply("ok", "unsure", "ok", 1, ""), "flagged", None),
        // 範囲外の番号は重複とみなさない
        (reply("ok", "ok", "ok", 9, ""), "approved", None),
    ];
    for (answer, status, rejection) in cases {
        fake.push_json(answer);
        let cookie = account_ready_to_post(&state.pool, &state, "t1").await;
        let res = post(&state, &cookie, "投稿").await;
        assert_eq!(res.json["status"], status, "{:?}", res.json);
        assert_eq!(res.json["rejection"].as_str(), rejection, "{:?}", res.json);
    }
    // 理由の文は運営者向けで、API では返さない
    fake.push_json(reply("ok", "reject", "ok", 0, "路線の見直しに反対"));
    let cookie = account_ready_to_post(&state.pool, &state, "t1").await;
    let res = post(&state, &cookie, "市長には辞任してほしい").await;
    assert_eq!(res.json["suggestion"], "路線の見直しに反対");
    crate::common::assert_no_key(&res.json, "reason");
}

#[tokio::test]
async fn duplicates_are_not_saved_and_point_to_the_existing_statement() {
    let (state, fake) = setup().await;
    let cookie = account_ready_to_post(&state.pool, &state, "t1").await;
    let before = count(&state.pool, "SELECT count(*) FROM statements").await;

    // 既存の意見は新しい順に番号が振られる（1 = e1）
    fake.push_json(reply("ok", "ok", "ok", 1, ""));
    let res = post(&state, &cookie, "通学の時間帯のバスを増やして").await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["status"], "duplicate");
    // 投稿できるのは投票を終えてからなので、案内する意見にはふつう投票済み。その票を返す
    assert_eq!(
        res.json["similar"],
        json!({"id": "e1", "body": "朝の通学の便を増やしてほしい", "your_vote": 0})
    );
    // 保存しないので、新しい意見の ID は無い
    assert!(res.json.get("id").is_none());
    assert_eq!(
        count(&state.pool, "SELECT count(*) FROM statements").await,
        before
    );

    // 上限に数えない（言い直して出し直せる）
    fake.push_json(crate::common::approved_reply());
    let res = post(&state, &cookie, "夜の便も増やしてほしい").await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.json["status"], "approved");
}

#[tokio::test]
async fn authors_agree_with_their_saved_statements() {
    let (state, fake) = setup().await;
    let author_votes = |id: String| {
        let pool = state.pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT v.vote_value FROM votes v
                 JOIN statements s ON s.id = v.statement_id AND s.user_id = v.user_id
                 WHERE s.id = $1",
            )
            .bind(id)
            .fetch_all(&pool)
            .await
            .unwrap()
        }
    };
    // 保存した意見（承認・確認待ち）には、書いた人の賛成を入れる
    for (answer, status) in [
        (crate::common::approved_reply(), "approved"),
        (reply("ok", "unsure", "ok", 0, ""), "flagged"),
    ] {
        fake.push_json(answer);
        let cookie = account_ready_to_post(&state.pool, &state, "t1").await;
        let res = post(&state, &cookie, "夜の便も増やしてほしい").await;
        assert_eq!(res.json["status"], status);
        assert_eq!(
            author_votes(res.json["id"].as_str().unwrap().to_owned()).await,
            [1]
        );
    }
    // 非公開には入れない
    fake.push_json(reply("reject", "ok", "ok", 0, ""));
    let cookie = account_ready_to_post(&state.pool, &state, "t1").await;
    let res = post(&state, &cookie, "投稿").await;
    assert_eq!(res.json["status"], "rejected");
    assert!(
        author_votes(res.json["id"].as_str().unwrap().to_owned())
            .await
            .is_empty()
    );
    // 似た意見は保存しないので、票も増えない
    fake.push_json(reply("ok", "ok", "ok", 1, ""));
    let cookie = account_ready_to_post(&state.pool, &state, "t1").await;
    let before = count(&state.pool, "SELECT count(*) FROM votes").await;
    let res = post(&state, &cookie, "通学の時間帯のバスを増やして").await;
    assert_eq!(res.json["status"], "duplicate");
    assert_eq!(
        count(&state.pool, "SELECT count(*) FROM votes").await,
        before
    );
}

#[tokio::test]
async fn ai_receives_the_topic_and_its_statements_but_no_ids() {
    let (state, fake) = setup().await;
    let cookie = account_ready_to_post(&state.pool, &state, "t1").await;
    fake.push_json(crate::common::approved_reply());
    let res = post(&state, &cookie, "送る本文").await;
    let id = res.json["id"].as_str().unwrap();
    let user_id: String = sqlx::query_scalar("SELECT user_id FROM statements WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .unwrap();

    let sent = fake.requests()[0].to_string();
    for expected in [
        "送る本文",
        "大田原のテスト",
        "バスの路線と本数の見直し",
        "朝の通学の便を増やしてほしい",
        "1. 朝の通学の便を増やしてほしい",
    ] {
        assert!(sent.contains(expected), "{expected} が送られていない");
    }
    assert!(
        !sent.contains("別の自治体の意見"),
        "別のトピックの意見が送られている"
    );
    for secret in [user_id.as_str(), "t1", "e1", "s1"] {
        assert!(
            !sent.contains(&format!("\"{secret}\"")) && !sent.contains(&format!("{secret}.")),
            "ID（{secret}）が送られている"
        );
    }
    assert!(!sent.contains(&user_id), "参加者 ID が送られている");
}

#[tokio::test]
async fn retry_marks_duplicates_as_rejected() {
    let (state, fake) = setup().await;
    sqlx::query(
        "INSERT INTO statements (id, topic_id, user_id, body, status, created_at)
         VALUES ('p1', 't1', 'u2', '朝のバスを増やして', 'pending', '2026-01-01T00:00:00.000Z')",
    )
    .execute(&state.pool)
    .await
    .unwrap();
    fake.push_json(reply("ok", "ok", "ok", 1, ""));
    assert_eq!(
        moderation::retry_pending(&state.pool, &state.moderation)
            .await
            .unwrap(),
        1
    );
    let (status, note): (String, Option<String>) =
        sqlx::query_as("SELECT status, moderation_note FROM statements WHERE id = 'p1'")
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(status, "rejected");
    assert!(note.unwrap().contains("e1"));
    // 再判定でも同じトピックの意見と比べる（自分自身は含めない）
    let sent = fake.requests()[0].to_string();
    assert!(sent.contains("朝の通学の便を増やしてほしい"));
    assert_eq!(sent.matches("朝のバスを増やして").count(), 1);
}

#[tokio::test]
async fn duplicates_of_unvoted_statements_have_no_vote() {
    let (state, fake) = setup().await;
    // 意見が15件より多いトピックでは、投票していない意見が残っていても投稿できる
    for i in 0..16 {
        sqlx::query(
            "INSERT INTO statements (id, topic_id, body, status, created_at)
             VALUES ($1, 't1', $2, 'approved', '2026-10-26T00:00:03Z')",
        )
        .bind(format!("m{i}"))
        .bind(format!("ほかの意見 {i}"))
        .execute(&state.pool)
        .await
        .unwrap();
    }
    let cookie = account_ready_to_post(&state.pool, &state, "t1").await;
    // 投票を終えたあとに承認された意見（いちばん新しいので 1 番）
    sqlx::query(
        "INSERT INTO statements (id, topic_id, body, status, created_at)
         VALUES ('n1', 't1', 'バス停に屋根を付けてほしい', 'approved', '2026-10-27T00:00:00Z')",
    )
    .execute(&state.pool)
    .await
    .unwrap();
    fake.push_json(reply("ok", "ok", "ok", 1, ""));
    let res = post(&state, &cookie, "バス停に雨よけを").await;
    assert_eq!(res.json["similar"]["id"], "n1");
    assert!(res.json["similar"].get("your_vote").is_none());
}
