// SPDX-License-Identifier: AGPL-3.0-only
// 使い方のページの、架空の例題トピックと例の分析。
// scripts/generate_tutorial_example.py で生成（手で編集しない）。本物のアルゴリズムで計算した結果

import type { GuideExample } from "./types";

export const EXAMPLE = {
  title: "【例】公園でボール遊びをできるようにするか",
  description: "使い方を説明するための架空の例です（実際の議会・投票ではありません）。ある市の議会で、公園でのボール遊びの禁止を見直すかが話し合われた、という想定です。",
  statements: [
    {
      id: "ex-01",
      body: "時間を区切って、ボール遊びができる時間帯を設けるべきだ。"
    },
    {
      id: "ex-02",
      body: "小さい子どもや高齢者の安全のため、ボール遊びは禁止のままでよい。"
    },
    {
      id: "ex-03",
      body: "広い公園には、フェンスで分けたボール遊びの区画を作るべきだ。"
    },
    {
      id: "ex-04",
      body: "近所の家への騒音や、ボールの飛び込みへの対策が必要だ。"
    },
    {
      id: "ex-05",
      body: "子どもが外で思い切り体を動かせる場所が足りない。"
    },
    {
      id: "ex-06",
      body: "ルールは市が一律に決めるより、公園ごとに近所の人と利用者で話し合って決めるべきだ。"
    },
    {
      id: "ex-07",
      body: "学校の校庭を、放課後や休日に開放するべきだ。"
    }
  ],
  analysis: {
    status: "ready",
    computed_at: "2026-11-20T00:00:00.000Z",
    participant_count: 120,
    analyzed_count: 120,
    groups: [
      {
        id: 0,
        label: "A",
        size: 66,
        representative: [
          {
            statement_id: "ex-03",
            body: "広い公園には、フェンスで分けたボール遊びの区画を作るべきだ。",
            repful_for: "agree",
            n_success: 65.0,
            n_trials: 66.0
          },
          {
            statement_id: "ex-01",
            body: "時間を区切って、ボール遊びができる時間帯を設けるべきだ。",
            repful_for: "agree",
            n_success: 65.0,
            n_trials: 66.0
          },
          {
            statement_id: "ex-06",
            body: "ルールは市が一律に決めるより、公園ごとに近所の人と利用者で話し合って決めるべきだ。",
            repful_for: "disagree",
            n_success: 65.0,
            n_trials: 66.0
          },
          {
            statement_id: "ex-02",
            body: "小さい子どもや高齢者の安全のため、ボール遊びは禁止のままでよい。",
            repful_for: "disagree",
            n_success: 63.0,
            n_trials: 66.0
          }
        ]
      },
      {
        id: 1,
        label: "B",
        size: 54,
        representative: [
          {
            statement_id: "ex-06",
            body: "ルールは市が一律に決めるより、公園ごとに近所の人と利用者で話し合って決めるべきだ。",
            repful_for: "agree",
            n_success: 53.0,
            n_trials: 54.0
          },
          {
            statement_id: "ex-02",
            body: "小さい子どもや高齢者の安全のため、ボール遊びは禁止のままでよい。",
            repful_for: "agree",
            n_success: 51.0,
            n_trials: 54.0
          },
          {
            statement_id: "ex-05",
            body: "子どもが外で思い切り体を動かせる場所が足りない。",
            repful_for: "agree",
            n_success: 54.0,
            n_trials: 54.0
          },
          {
            statement_id: "ex-03",
            body: "広い公園には、フェンスで分けたボール遊びの区画を作るべきだ。",
            repful_for: "disagree",
            n_success: 54.0,
            n_trials: 54.0
          },
          {
            statement_id: "ex-01",
            body: "時間を区切って、ボール遊びができる時間帯を設けるべきだ。",
            repful_for: "disagree",
            n_success: 51.0,
            n_trials: 54.0
          }
        ]
      }
    ],
    consensus: [
      {
        statement_id: "ex-07",
        body: "学校の校庭を、放課後や休日に開放するべきだ。",
        gac: 0.9046743697478992,
        groups: [
          {
            group_id: 0,
            label: "A",
            agree: 64,
            seen: 66
          },
          {
            group_id: 1,
            label: "B",
            agree: 52,
            seen: 54
          }
        ]
      },
      {
        statement_id: "ex-05",
        body: "子どもが外で思い切り体を動かせる場所が足りない。",
        gac: 0.8954831932773109,
        groups: [
          {
            group_id: 0,
            label: "A",
            agree: 61,
            seen: 66
          },
          {
            group_id: 1,
            label: "B",
            agree: 54,
            seen: 54
          }
        ]
      },
      {
        statement_id: "ex-04",
        body: "近所の家への騒音や、ボールの飛び込みへの対策が必要だ。",
        gac: 0.8907563025210083,
        groups: [
          {
            group_id: 0,
            label: "A",
            agree: 63,
            seen: 66
          },
          {
            group_id: 1,
            label: "B",
            agree: 52,
            seen: 54
          }
        ]
      }
    ],
    statements_by_group: [
      {
        statement_id: "ex-01",
        body: "時間を区切って、ボール遊びができる時間帯を設けるべきだ。",
        groups: [
          {
            group_id: 0,
            agree: 65,
            disagree: 1,
            seen: 66
          },
          {
            group_id: 1,
            agree: 3,
            disagree: 51,
            seen: 54
          }
        ]
      },
      {
        statement_id: "ex-02",
        body: "小さい子どもや高齢者の安全のため、ボール遊びは禁止のままでよい。",
        groups: [
          {
            group_id: 0,
            agree: 3,
            disagree: 63,
            seen: 66
          },
          {
            group_id: 1,
            agree: 51,
            disagree: 3,
            seen: 54
          }
        ]
      },
      {
        statement_id: "ex-03",
        body: "広い公園には、フェンスで分けたボール遊びの区画を作るべきだ。",
        groups: [
          {
            group_id: 0,
            agree: 65,
            disagree: 1,
            seen: 66
          },
          {
            group_id: 1,
            agree: 0,
            disagree: 54,
            seen: 54
          }
        ]
      },
      {
        statement_id: "ex-04",
        body: "近所の家への騒音や、ボールの飛び込みへの対策が必要だ。",
        groups: [
          {
            group_id: 0,
            agree: 63,
            disagree: 3,
            seen: 66
          },
          {
            group_id: 1,
            agree: 52,
            disagree: 2,
            seen: 54
          }
        ]
      },
      {
        statement_id: "ex-05",
        body: "子どもが外で思い切り体を動かせる場所が足りない。",
        groups: [
          {
            group_id: 0,
            agree: 61,
            disagree: 5,
            seen: 66
          },
          {
            group_id: 1,
            agree: 54,
            disagree: 0,
            seen: 54
          }
        ]
      },
      {
        statement_id: "ex-06",
        body: "ルールは市が一律に決めるより、公園ごとに近所の人と利用者で話し合って決めるべきだ。",
        groups: [
          {
            group_id: 0,
            agree: 1,
            disagree: 65,
            seen: 66
          },
          {
            group_id: 1,
            agree: 53,
            disagree: 1,
            seen: 54
          }
        ]
      },
      {
        statement_id: "ex-07",
        body: "学校の校庭を、放課後や休日に開放するべきだ。",
        groups: [
          {
            group_id: 0,
            agree: 64,
            disagree: 2,
            seen: 66
          },
          {
            group_id: 1,
            agree: 52,
            disagree: 2,
            seen: 54
          }
        ]
      }
    ],
    group_composition: [
      {
        group_id: 0,
        by_type: {
          commuter: 13,
          neighbor: 12,
          related: 11,
          resident: 30
        }
      },
      {
        group_id: 1,
        by_type: {
          commuter: 9,
          neighbor: 8,
          related: 7,
          resident: 30
        }
      }
    ],
    map: {
      base_clusters: [
        {
          x: 1.7870140363153222,
          y: -0.1515081297460465,
          count: 52,
          group_id: 0
        },
        {
          x: -1.2202603668919538,
          y: 0.5094076660345838,
          count: 3,
          group_id: 1
        },
        {
          x: -2.209214468166849,
          y: -0.011847861441175616,
          count: 43,
          group_id: 1
        },
        {
          x: 0.8142767636820211,
          y: 0.9856346599962555,
          count: 2,
          group_id: 0
        },
        {
          x: -1.236477195533549,
          y: -1.1489906511834775,
          count: 3,
          group_id: 1
        },
        {
          x: 1.7789831384662782,
          y: -0.31880023202276997,
          count: 2,
          group_id: 0
        },
        {
          x: 1.8635630228773896,
          y: 1.115983081049127,
          count: 3,
          group_id: 0
        },
        {
          x: 0.7642089538537392,
          y: -0.36421533567740666,
          count: 1,
          group_id: 0
        },
        {
          x: 1.8733491074166424,
          y: 1.944228998483808,
          count: 1,
          group_id: 0
        },
        {
          x: -2.2172453660158933,
          y: -0.17913996371789925,
          count: 2,
          group_id: 1
        },
        {
          x: -2.199428383627597,
          y: 0.8163980559935057,
          count: 2,
          group_id: 1
        },
        {
          x: 1.7968001208545743,
          y: 0.676737787688635,
          count: 2,
          group_id: 0
        },
        {
          x: 0.8908257502440893,
          y: 2.253125870791429,
          count: 1,
          group_id: 0
        },
        {
          x: 0.7752819882029288,
          y: -0.4150279177763579,
          count: 1,
          group_id: 0
        },
        {
          x: 0.7980599350404259,
          y: -0.6727636572218054,
          count: 1,
          group_id: 0
        },
        {
          x: -1.197482420054457,
          y: 0.25167192658913595,
          count: 1,
          group_id: 1
        }
      ],
      statements: [
        {
          statement_id: "ex-01",
          x: 1.1338281976799955,
          y: 0.5976154145947505
        },
        {
          statement_id: "ex-02",
          x: -1.415491502915022,
          y: 1.654728364755588
        },
        {
          statement_id: "ex-03",
          x: 1.2402902819478685,
          y: 0.2579364191093169
        },
        {
          statement_id: "ex-04",
          x: -0.0010788144166632289,
          y: -0.09130553008069596
        },
        {
          statement_id: "ex-05",
          x: -0.008438732564885735,
          y: -0.1397277721968407
        },
        {
          statement_id: "ex-06",
          x: -1.4722352660066844,
          y: -0.38346430356996297
        },
        {
          statement_id: "ex-07",
          x: 0.0007082586171044017,
          y: 0.014753776630979768
        }
      ],
      you: null
    },
    residency: [
      {
        statement_id: "ex-01",
        body: "時間を区切って、ボール遊びができる時間帯を設けるべきだ。",
        by_type: {
          commuter: {
            agree: 13,
            seen: 22
          },
          neighbor: {
            agree: 12,
            seen: 20
          },
          related: {
            agree: 10,
            seen: 18
          },
          resident: {
            agree: 34,
            seen: 61
          }
        }
      },
      {
        statement_id: "ex-02",
        body: "小さい子どもや高齢者の安全のため、ボール遊びは禁止のままでよい。",
        by_type: {
          commuter: {
            agree: 9,
            seen: 22
          },
          neighbor: {
            agree: 9,
            seen: 20
          },
          related: {
            agree: 7,
            seen: 18
          },
          resident: {
            agree: 29,
            seen: 61
          }
        }
      },
      {
        statement_id: "ex-03",
        body: "広い公園には、フェンスで分けたボール遊びの区画を作るべきだ。",
        by_type: {
          commuter: {
            agree: 13,
            seen: 22
          },
          neighbor: {
            agree: 12,
            seen: 20
          },
          related: {
            agree: 11,
            seen: 18
          },
          resident: {
            agree: 30,
            seen: 61
          }
        }
      },
      {
        statement_id: "ex-04",
        body: "近所の家への騒音や、ボールの飛び込みへの対策が必要だ。",
        by_type: {
          commuter: {
            agree: 22,
            seen: 22
          },
          neighbor: {
            agree: 18,
            seen: 20
          },
          related: {
            agree: 17,
            seen: 18
          },
          resident: {
            agree: 59,
            seen: 61
          }
        }
      },
      {
        statement_id: "ex-05",
        body: "子どもが外で思い切り体を動かせる場所が足りない。",
        by_type: {
          commuter: {
            agree: 22,
            seen: 22
          },
          neighbor: {
            agree: 19,
            seen: 20
          },
          related: {
            agree: 17,
            seen: 18
          },
          resident: {
            agree: 58,
            seen: 61
          }
        }
      },
      {
        statement_id: "ex-06",
        body: "ルールは市が一律に決めるより、公園ごとに近所の人と利用者で話し合って決めるべきだ。",
        by_type: {
          commuter: {
            agree: 10,
            seen: 22
          },
          neighbor: {
            agree: 8,
            seen: 20
          },
          related: {
            agree: 6,
            seen: 18
          },
          resident: {
            agree: 30,
            seen: 61
          }
        }
      },
      {
        statement_id: "ex-07",
        body: "学校の校庭を、放課後や休日に開放するべきだ。",
        by_type: {
          commuter: {
            agree: 20,
            seen: 22
          },
          neighbor: {
            agree: 20,
            seen: 20
          },
          related: {
            agree: 18,
            seen: 18
          },
          resident: {
            agree: 59,
            seen: 61
          }
        }
      }
    ]
  }
} satisfies GuideExample;
