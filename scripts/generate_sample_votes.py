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
N_VOTERS = 120
# 傾向と逆の票を入れる確率。強すぎると、逆の票が重なった少数の人が別のグループになる
NOISE_FLIP = 0.03
# 投票しない確率。分析の対象（in-conv）になるには min(7, 意見数) 件以上の投票が必要
NOISE_SKIP = 0.03
AGREE, DISAGREE, PASS = 1, -1, 0

# このまちとの関わり方（120人分）。グループの構成や比較の図に出るよう、
# どのグループでも各区分が5人以上になりやすい人数にする。
# 地区は付けない（市民を地区で分けると分断を煽るため集めない。docs/adr/0002-participant-attributes.md）
RESIDENCY = (["resident"] * 60) + (["neighbor"] * 20) + (["commuter"] * 22) + (["related"] * 18)

# トピックごとの、グループの人数と各意見への傾向（A=賛成、D=反対、P=パス）
TOPICS = {
    # 放課後の居場所: 行政が無料で整備する派 / 地域と既存施設を生かす派。05（中高生の場所）は全員賛成
    "sample-092100-1": [
        (66, "AAADAD"),
        (54, "DADAAA"),
    ],
    # 公共交通: 定期バスを増やす派 / 効率化する派。01（本数が少ない）と 05（電話予約）は全員賛成
    "sample-092100-2": [
        (66, "ADADA"),
        (54, "AAPAA"),
    ],
    # 避難所: 地域の自助 / 行政の責任 / デジタル活用。02（更衣室）と 05（やさしい日本語）は全員賛成
    "sample-092100-3": [
        (42, "AAADAAP"),
        (42, "DAPPADA"),
        (36, "AAAAADA"),
    ],
    # 締め切った路線バス（提言を試す）: 公共サービスとして守る派 / 効率化する派。01（時刻表）と 04（運転手）は全員賛成
    "sample-092100-4": [
        (64, "ADAAP"),
        (56, "AADAA"),
    ],
    # 締め切った通学のバス: 補助する派 / 家庭に任せる派。04（雨の日）は全員賛成。
    # 違いが2件だけだと、分析のクラスタリングが片寄った分け方（98人と6人など）に落ち着くので、05 も分ける
    "sample-092100-5": [
        (62, "AADAA"),
        (58, "PDAAP"),
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
    relations = [
        f"    ('{v}', '092100', '{rtype}', NULL, '2026-10-20T00:00:00.000Z')"
        for v, rtype in zip(voters, RESIDENCY)
    ]
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
                    # 締め切ったトピック（4・5）は、公開していた7〜9月に投票したことにする
                    day = f"2026-10-2{1 + t_index}" if t_index < 3 else f"2026-08-0{t_index}"
                    at = f"{day}T{minute // 60:02d}:{minute % 60:02d}:00.000Z"
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
