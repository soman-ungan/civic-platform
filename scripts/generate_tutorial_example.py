#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""使い方のページ（/guide）に出す、架空の例題トピックと例の分析を作る
（frontend/src/components/guide/example.ts。手で編集しない）。

例は本番の DB に入れない（架空の票が本物の意見と混ざらないように。docs/plans/2026-11-20-tutorial.md）。
本物のアルゴリズムで計算した分析を見せるため、手元の使い捨ての DB で計算して、API の応答を固定の JSON にする。
乱数のシードを固定しているので、同じ手順なら同じ票になる。

手順（リポジトリのルートで。DB とポートは使い捨て）:

    export DATABASE_URL="sqlite://$PWD/tmp-guide.db?mode=rwc" PORT=8099
    (cd backend && cargo run -- seed)                       # マイグレーションと自治体のマスタ
    python3 scripts/generate_tutorial_example.py sql | sqlite3 tmp-guide.db
    (cd backend && cargo run) &                             # 分析はバックグラウンドで動く
    python3 scripts/generate_tutorial_example.py fetch http://127.0.0.1:8099 \\
        > frontend/src/components/guide/example.ts
    kill %1; rm tmp-guide.db*

fetch は、見る人として最初の意見すべてに投票してから（分析が開く条件）、分析ができるのを待って取る。
ID は ex-01 などに置き換え、あなたの位置（map.you）は消す（使い方のページの票と合わないため）。
型は frontend の AnalysisView に合わせて確かめる（satisfies。npm run typecheck）。
"""

import http.cookiejar
import json
import random
import re
import sys
import time
import urllib.request

SEED = 20261120
MUNICIPALITY_ID = "092100"  # 計算に使うだけ。JSON には自治体を残さない
SLUG = "otawara"
TOPIC_ID = "guide-example"
N_VOTERS = 120
NOISE_FLIP = 0.03
# 投票しない人は出さない（7件すべてに投票しないと分析の対象にならず、例の人数が減る）
NOISE_SKIP = 0.0
AGREE, DISAGREE, PASS = 1, -1, 0
VALUE = {"A": AGREE, "D": DISAGREE, "P": PASS}

TITLE = "【例】公園でボール遊びをできるようにするか"
DESCRIPTION = (
    "使い方を説明するための架空の例です（実際の議会・投票ではありません）。"
    "ある市の議会で、公園でのボール遊びの禁止を見直すかが話し合われた、という想定です。"
)
STATEMENTS = [
    "時間を区切って、ボール遊びができる時間帯を設けるべきだ。",
    "小さい子どもや高齢者の安全のため、ボール遊びは禁止のままでよい。",
    "広い公園には、フェンスで分けたボール遊びの区画を作るべきだ。",
    "近所の家への騒音や、ボールの飛び込みへの対策が必要だ。",
    "子どもが外で思い切り体を動かせる場所が足りない。",
    "ルールは市が一律に決めるより、公園ごとに近所の人と利用者で話し合って決めるべきだ。",
    "学校の校庭を、放課後や休日に開放するべきだ。",
]
# グループの人数と各意見への傾向（A=賛成、D=反対、P=パス）。4（騒音の対策）・5（場所が足りない）・7（校庭）は全員賛成。
# 例は分かりやすく2つのグループにする（3つに分けても、違いが小さいとアルゴリズムは2つにまとめる）
GROUPS = [
    (66, "ADAAADA"),  # 時間や区画を決めて遊べるようにする
    (54, "DADAAAA"),  # 安全を優先し、ルールは地域で決める
]
# 見る人（fetch）の票。分析が開く条件を満たすためだけ
VIEWER = "ADAAADA"
# 関わり方。どのグループでも各区分が5人以上になりやすい人数（人を属性で分けない。ADR 0002）
RESIDENCY = ["resident"] * 60 + ["neighbor"] * 20 + ["commuter"] * 22 + ["related"] * 18
AT = "2026-11-20T00:00:00.000Z"


def sid(n: int) -> str:
    return f"{TOPIC_ID}-{n:02d}"


def sql() -> None:
    rng = random.Random(SEED)
    voters = [f"guide-voter-{i:03d}" for i in range(1, N_VOTERS + 1)]
    out = [
        f"INSERT INTO topics (id, municipality_id, title, description, source_type, source_url, created_at) VALUES "
        f"('{TOPIC_ID}', '{MUNICIPALITY_ID}', '{TITLE}', '{DESCRIPTION}', 'citizen', NULL, '{AT}');",
        "INSERT INTO statements (id, topic_id, user_id, body, status, created_at, is_seed) VALUES",
        ",\n".join(
            f"('{sid(n)}', '{TOPIC_ID}', NULL, '{body}', 'approved', '{AT}', TRUE)"
            for n, body in enumerate(STATEMENTS, start=1)
        )
        + ";",
        "INSERT INTO users (id, created_at) VALUES",
        ",\n".join(f"('{v}', '{AT}')" for v in voters) + ";",
        "INSERT INTO user_municipality_relations (user_id, municipality_id, residency_type, district_id, created_at) VALUES",
        ",\n".join(
            f"('{v}', '{MUNICIPALITY_ID}', '{r}', NULL, '{AT}')"
            for v, r in zip(voters, RESIDENCY)
        )
        + ";",
    ]
    order = voters[:]
    rng.shuffle(order)
    rows = []
    cursor = 0
    for size, pattern in GROUPS:
        for voter in order[cursor : cursor + size]:
            for n, tendency in enumerate(pattern, start=1):
                if rng.random() < NOISE_SKIP:
                    continue
                value = VALUE[tendency]
                if value != PASS and rng.random() < NOISE_FLIP:
                    value = -value
                rows.append(f"('{sid(n)}-{voter}', '{voter}', '{sid(n)}', {value}, '{AT}')")
        cursor += size
    out += [
        "INSERT INTO votes (id, user_id, statement_id, vote_value, created_at) VALUES",
        ",\n".join(rows) + ";",
    ]
    print("\n".join(out))


def fetch(api: str) -> None:
    opener = urllib.request.build_opener(
        urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar())
    )
    base = f"{api}/api/municipalities/{SLUG}/topics/{TOPIC_ID}"

    def call(path: str, body: dict | None = None, method: str = "POST", url: str = base):
        req = urllib.request.Request(
            url + path,
            data=None if body is None else json.dumps(body).encode(),
            headers={"Content-Type": "application/json"},
            method="GET" if body is None else method,
        )
        with opener.open(req) as res:
            return json.load(res)

    # 投票の前に、関わり方の登録が要る
    call("/relation", {"residency_type": "resident"}, "PUT", f"{api}/api/municipalities/{SLUG}")
    for n, tendency in enumerate(VIEWER, start=1):
        call(f"/statements/{sid(n)}/votes", {"value": VALUE[tendency]})

    for _ in range(60):
        view = call("/analysis")
        if view["status"] == "ready":
            break
        time.sleep(5)
    else:
        sys.exit(f"分析ができませんでした: {view}")

    # ID を短い名前に置き換え、計算した時刻と見る人の位置を消す
    ids = {sid(n): f"ex-{n:02d}" for n in range(1, len(STATEMENTS) + 1)}
    text = json.dumps(view, ensure_ascii=False)
    for old, new in ids.items():
        text = text.replace(f'"{old}"', f'"{new}"')
    view = json.loads(text)
    view["computed_at"] = AT
    view["map"]["you"] = None
    example = {
        "title": TITLE,
        "description": DESCRIPTION,
        "statements": [
            {"id": ids[sid(n)], "body": body} for n, body in enumerate(STATEMENTS, start=1)
        ],
        "analysis": view,
    }
    print(
        "\n".join(
            [
                "// SPDX-License-Identifier: AGPL-3.0-only",
                "// 使い方のページの、架空の例題トピックと例の分析。",
                "// scripts/generate_tutorial_example.py で生成（手で編集しない）。本物のアルゴリズムで計算した結果",
                "",
                'import type { GuideExample } from "./types";',
                "",
                "export const EXAMPLE = "
                # キーの引用符を外す（frontend の書き方にそろえる）
                + re.sub(
                    r'^( *)"([A-Za-z_]\w*)": ',
                    r"\1\2: ",
                    json.dumps(example, ensure_ascii=False, indent=2),
                    flags=re.M,
                )
                + " satisfies GuideExample;",
            ]
        )
    )

if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "sql":
        sql()
    elif len(sys.argv) >= 3 and sys.argv[1] == "fetch":
        fetch(sys.argv[2].rstrip("/"))
    else:
        sys.exit("使い方: generate_tutorial_example.py sql | fetch <API の URL>")
