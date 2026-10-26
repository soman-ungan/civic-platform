#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""開発用サンプルの参加者と投票を生成する（data/dev-seeds/110_sample_votes_092100_otawara.sql）。

分析（Polis の PCA・クラスタリング）の動作を画面で確かめられるよう、
トピックごとに意見の傾向が異なるグループと、全員がおおむね賛成する意見を作る。
乱数のシードを固定しているので、何度実行しても同じ SQL になる。

    python3 scripts/generate_sample_votes.py > data/dev-seeds/110_sample_votes_092100_otawara.sql
"""

import random

SEED = 20261026
N_VOTERS = 40
# 傾向と逆の票を入れる確率。強すぎると、逆の票が重なった少数の人が別のグループになる
NOISE_FLIP = 0.03
# 投票しない確率。分析の対象（in-conv）になるには min(7, 意見数) 件以上の投票が必要
NOISE_SKIP = 0.03
AGREE, DISAGREE, PASS = 1, -1, 0

# 居住地属性（40人分）。住民には地区も付ける。比較表に出るよう、どの区分も5人以上にする
RESIDENCY = (["resident"] * 22) + (["neighbor"] * 6) + (["commuter"] * 7) + (["related"] * 5)
DISTRICTS = ["092100-otawara", "092100-kurobane", "092100-yuzukami", None]

# トピックごとの、グループの人数と各意見への傾向（A=賛成、D=反対、P=パス）
TOPICS = {
    # 放課後の居場所: 行政が無料で整備する派 / 地域と既存施設を生かす派。05（中高生の場所）は全員賛成
    "sample-092100-1": [
        (22, "AAADAD"),
        (18, "DADAAA"),
    ],
    # 公共交通: 定期バスを増やす派 / 効率化する派。01（本数が少ない）と 05（電話予約）は全員賛成
    "sample-092100-2": [
        (22, "ADADA"),
        (18, "AAPAA"),
    ],
    # 避難所: 地域の自助 / 行政の責任 / デジタル活用。02（更衣室）と 05（やさしい日本語）は全員賛成
    "sample-092100-3": [
        (14, "AAADAAP"),
        (14, "DAPPADA"),
        (12, "AAAAADA"),
    ],
}

VALUE = {"A": AGREE, "D": DISAGREE, "P": PASS}


def main() -> None:
    rng = random.Random(SEED)
    voters = [f"sample-voter-{i:02d}" for i in range(1, N_VOTERS + 1)]
    out = [
        "-- SPDX-License-Identifier: AGPL-3.0-only",
        "-- 開発・動作確認用のサンプル参加者と投票（本番には投入しない。`backend seed --dev` でのみ適用）",
        "-- すべて架空のデータ。scripts/generate_sample_votes.py で生成（手で編集しない）。",
        "-- 前提: 100_sample_092100_otawara.sql のトピックと意見が登録済みであること。",
        "",
        "INSERT INTO users (id, created_at) VALUES",
        ",\n".join(f"    ('{v}', '2026-10-20T00:00:00.000Z')" for v in voters),
        "ON CONFLICT DO NOTHING;",
        "",
        "INSERT INTO user_municipality_relations (user_id, municipality_id, residency_type, district_id, created_at) VALUES",
    ]
    relations = []
    for v, rtype in zip(voters, RESIDENCY):
        district = rng.choice(DISTRICTS) if rtype == "resident" else None
        district_sql = f"'{district}'" if district else "NULL"
        relations.append(f"    ('{v}', '092100', '{rtype}', {district_sql}, '2026-10-20T00:00:00.000Z')")
    out += [",\n".join(relations), "ON CONFLICT DO NOTHING;", ""]

    for t_index, (topic_id, groups) in enumerate(TOPICS.items()):
        # トピックごとに参加者をグループへ割り振る（毎回同じ並び）
        order = voters[:]
        rng.shuffle(order)
        rows = []
        cursor = 0
        minute = 0
        for size, pattern in groups:
            for voter in order[cursor : cursor + size]:
                for n, tendency in enumerate(pattern, start=1):
                    if rng.random() < NOISE_SKIP:
                        continue
                    value = VALUE[tendency]
                    if value != PASS and rng.random() < NOISE_FLIP:
                        value = -value
                    minute += 1
                    at = f"2026-10-2{1 + t_index}T{minute // 60:02d}:{minute % 60:02d}:00.000Z"
                    sid = f"{topic_id}-{n:02d}"
                    rows.append(f"    ('{sid}-{voter}', '{voter}', '{sid}', {value}, '{at}')")
            cursor += size
        out += [
            f"-- {topic_id}: グループの人数 {[size for size, _ in groups]}",
            "INSERT INTO votes (id, user_id, statement_id, vote_value, created_at) VALUES",
            ",\n".join(rows),
            "ON CONFLICT DO NOTHING;",
            "",
        ]
    print("\n".join(out))


if __name__ == "__main__":
    main()
