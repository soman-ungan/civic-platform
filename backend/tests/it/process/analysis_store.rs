// SPDX-License-Identifier: AGPL-3.0-only

//! 分析の計算と保存（`process::analysis::store`）: 2つのグループに分かれる投票を保存し、入力が変わらなければ計算し直さない、
//! 投票・判定・アカウントのまとめで計算し直す、人数が足りなければ insufficient

use backend::process::analysis::store::{RunOutcome, Stored, load, run_all, run_topic};

use crate::common::{insert_vote, seed_polarized as polarized};

#[tokio::test]
async fn computes_and_stores_two_groups() {
    let pool = crate::common::test_pool().await;
    polarized(&pool).await;

    assert_eq!(
        run_topic(&pool, "t1", false).await.unwrap(),
        RunOutcome::Computed
    );
    let Some(Stored::Ready { result, state, .. }) = load(&pool, "t1").await.unwrap() else {
        panic!("ready のはず");
    };
    assert_eq!(result.groups.len(), 2);
    let mut sizes: Vec<usize> = result.groups.iter().map(|g| g.n_members).collect();
    sizes.sort();
    assert_eq!(sizes, [10, 10]);
    // DB の「賛成 = 1」は Polis の「賛成 = -1」として扱われ、common は合意点になる
    assert!(result.consensus.agree.iter().any(|c| c.tid == "common"));
    // 保存される結果に参加者 ID は含まれない（状態には含まれる）
    let stored: String =
        sqlx::query_scalar("SELECT result FROM topic_analyses WHERE topic_id = 't1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!stored.contains("v00"));
    assert!(state.in_conv.contains(&"v00".to_owned()));
}

#[tokio::test]
async fn unchanged_input_is_not_recomputed_and_new_votes_carry_state() {
    let pool = crate::common::test_pool().await;
    polarized(&pool).await;

    assert_eq!(
        run_topic(&pool, "t1", false).await.unwrap(),
        RunOutcome::Computed
    );
    assert_eq!(
        run_topic(&pool, "t1", false).await.unwrap(),
        RunOutcome::Unchanged
    );
    assert_eq!(
        run_topic(&pool, "t1", true).await.unwrap(),
        RunOutcome::Computed
    );

    // 投票が増えたら再計算し、smoother は前回から続く（強制実行を含めて3回目）
    insert_vote(&pool, "u1", "a0", 1, "2026-10-26T01:00:00.000Z").await;
    assert_eq!(
        run_topic(&pool, "t1", false).await.unwrap(),
        RunOutcome::Computed
    );
    let Some(Stored::Ready { state, .. }) = load(&pool, "t1").await.unwrap() else {
        panic!()
    };
    assert_eq!(state.group_k_smoother.unwrap().last_k_count, 3);
}

#[tokio::test]
async fn moderation_change_triggers_recompute() {
    let pool = crate::common::test_pool().await;
    polarized(&pool).await;
    run_topic(&pool, "t1", false).await.unwrap();

    sqlx::query("UPDATE statements SET status = 'rejected' WHERE id = 'common'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        run_topic(&pool, "t1", false).await.unwrap(),
        RunOutcome::Computed
    );
    let Some(Stored::Ready { result, .. }) = load(&pool, "t1").await.unwrap() else {
        panic!()
    };
    assert!(result.consensus.agree.iter().all(|c| c.tid != "common"));
}

#[tokio::test]
async fn insufficient_data_is_stored() {
    let pool = crate::common::test_pool().await;
    crate::common::seed_fixture(&pool).await;
    // fixture には意見 s1（未承認）があるだけで、投票はない
    assert_eq!(
        run_topic(&pool, "t1", false).await.unwrap(),
        RunOutcome::Computed
    );
    let Some(Stored::Insufficient { detail, .. }) = load(&pool, "t1").await.unwrap() else {
        panic!("insufficient のはず");
    };
    assert_eq!((detail.participants, detail.statements), (0, 0));
}

#[tokio::test]
async fn run_all_only_computes_changed_topics() {
    let pool = crate::common::test_pool().await;
    polarized(&pool).await;
    // t1 と t2 の2トピック
    assert_eq!(run_all(&pool, false).await.unwrap(), 2);
    assert_eq!(run_all(&pool, false).await.unwrap(), 0);
}

#[tokio::test]
async fn insufficient_then_enough_votes_is_computed() {
    let pool = crate::common::test_pool().await;
    crate::common::seed_fixture(&pool).await;
    run_topic(&pool, "t1", false).await.unwrap();
    assert!(matches!(
        load(&pool, "t1").await.unwrap(),
        Some(Stored::Insufficient { .. })
    ));

    // 投票が集まったら、前回が insufficient でも計算できる
    sqlx::raw_sql("DELETE FROM statements WHERE id = 's1'")
        .execute(&pool)
        .await
        .unwrap();
    // 意見と投票を足す
    for i in 0..3 {
        sqlx::query("INSERT INTO statements (id, topic_id, body, status, created_at) VALUES ($1, 't1', 'x', 'approved', '2026-10-26T00:00:00Z')")
            .bind(format!("b{i}"))
            .execute(&pool)
            .await
            .unwrap();
    }
    for (n, uid) in ["u1", "u2"].iter().enumerate() {
        for i in 0..3 {
            insert_vote(
                &pool,
                uid,
                &format!("b{i}"),
                1,
                &format!("2026-10-26T01:0{n}:0{i}.000Z"),
            )
            .await;
        }
    }
    assert_eq!(
        run_topic(&pool, "t1", false).await.unwrap(),
        RunOutcome::Computed
    );
    assert!(matches!(
        load(&pool, "t1").await.unwrap(),
        Some(Stored::Ready { .. })
    ));
}

#[tokio::test]
async fn merging_an_anonymous_voter_into_an_account_triggers_recompute() {
    let pool = crate::common::test_pool().await;
    polarized(&pool).await;
    run_topic(&pool, "t1", false).await.unwrap();

    // このトピックに投票していないアカウントへ、匿名の v00 の投票がすべて移る（数も時刻も変わらない）
    sqlx::query("INSERT INTO users (id, created_at, registered_at) VALUES ('acct', $1, $1)")
        .bind("2026-10-26T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();
    backend::records::accounts::merge_anonymous(&pool, "v00", "acct")
        .await
        .unwrap();
    assert_eq!(
        run_topic(&pool, "t1", false).await.unwrap(),
        RunOutcome::Computed
    );
    let Some(Stored::Ready { state, .. }) = load(&pool, "t1").await.unwrap() else {
        panic!()
    };
    assert!(state.in_conv.contains(&"acct".to_owned()));
    assert!(!state.in_conv.contains(&"v00".to_owned()));
}
