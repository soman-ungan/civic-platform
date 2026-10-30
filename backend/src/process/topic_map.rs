// SPDX-License-Identifier: AGPL-3.0-only

//! トピックマップ（締め切ったトピックの地図。docs/plans/2026-10-30-topic-map.md）。
//!
//! - トピック1件を点1つにし、トピックどうしの似ている度合い（`process::similarity`）で配置する
//! - 地図に載せるのはタグで絞ったものだけ。配置は、似ている度合いの表の各行を特徴として
//!   Polis と同じ PCA（`analysis::pca::powerit_pca`）で2次元にする（多次元尺度構成法とほぼ同じ）。決定的で、乱数を使わない
//! - 近傍は、起点のトピックとの似ている度合いが目安（`SIMILAR_TOPIC_SCORE`）以上の、締め切ったトピック
//! - 比べるのは同じ自治体の締め切ったトピックだけ。API キーも費用も要らない

use std::collections::HashMap;

use serde::Serialize;
use sqlx::SqlitePool;

use crate::process::analysis::pca::powerit_pca;
use crate::process::similarity::{Index, SIMILAR_TOPIC_SCORE};
use crate::shared::util::now_iso;

/// PCA のべき乗法の回数（分析と同じ）
const PCA_ITERS: usize = 100;

/// 締め切ったトピック（公開済みで、期限を過ぎたもの）
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClosedTopic {
    pub id: String,
    pub title: String,
    pub description: String,
    pub closes_at: String,
    /// 投票した人数
    pub participants: i64,
}

/// 自治体の締め切ったトピックと、それらで作った似ている度合いのコーパス
pub struct Closed {
    pub topics: Vec<ClosedTopic>,
    pub index: Index,
}

impl Closed {
    pub fn get(&self, id: &str) -> Option<&ClosedTopic> {
        self.topics.iter().find(|t| t.id == id)
    }

    /// 起点の近傍（起点を除く、目安以上のもの。似ている順）
    pub fn neighbors(&self, anchor: &str) -> Vec<(String, f64)> {
        self.index
            .similar_to(anchor, self.topics.len())
            .into_iter()
            .filter(|m| m.score >= SIMILAR_TOPIC_SCORE)
            .map(|m| (m.id, m.score))
            .collect()
    }

    /// `id` が起点そのものか、起点の近傍か
    pub fn in_neighborhood(&self, anchor: &str, id: &str) -> bool {
        self.get(anchor).is_some()
            && self.get(id).is_some()
            && (id == anchor || self.index.score(anchor, id) >= SIMILAR_TOPIC_SCORE)
    }
}

/// 自治体の締め切ったトピック（締め切った新しい順）
pub async fn closed(pool: &SqlitePool, municipality_id: &str) -> Result<Closed, sqlx::Error> {
    let topics: Vec<ClosedTopic> = sqlx::query_as(
        "SELECT t.id, t.title, t.description, t.closes_at,
                (SELECT count(DISTINCT v.user_id) FROM votes v
                   JOIN statements s ON s.id = v.statement_id
                  WHERE s.topic_id = t.id) AS participants
         FROM topics t
         WHERE t.municipality_id = $1 AND t.status = 'published'
           AND t.closes_at IS NOT NULL AND t.closes_at <= $2
         ORDER BY t.closes_at DESC, t.id",
    )
    .bind(municipality_id)
    .bind(now_iso())
    .fetch_all(pool)
    .await?;
    let index = Index::build(
        topics
            .iter()
            .map(|t| (t.id.clone(), format!("{} {}", t.title, t.description))),
    );
    Ok(Closed { topics, index })
}

/// 地図の点
#[derive(Debug, Clone, Serialize)]
pub struct Point {
    pub id: String,
    pub title: String,
    pub closes_at: String,
    pub participants: i64,
    /// -1〜1 に収めた座標
    pub x: f64,
    pub y: f64,
}

/// 似ている組（地図の線）
#[derive(Debug, Clone, Serialize)]
pub struct Edge {
    pub a: String,
    pub b: String,
    pub score: f64,
}

/// 地図で絞れるタグ（締め切ったトピックに付いているもの）
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MapTag {
    pub id: String,
    pub kind: String,
    pub name: String,
    /// そのタグの付いた締め切ったトピックの数
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopicMap {
    pub tags: Vec<MapTag>,
    /// 選んだタグ（無ければ点も線も空）
    pub tag: Option<String>,
    pub points: Vec<Point>,
    pub edges: Vec<Edge>,
}

/// 似ている度合いの表を2次元にする（各行を特徴として PCA）。-1〜1 に収め、向きをそろえる
pub fn layout(matrix: &[Vec<f64>]) -> Vec<[f64; 2]> {
    let n = matrix.len();
    if n < 2 {
        return vec![[0.0, 0.0]; n];
    }
    let pca = powerit_pca(matrix, 2, PCA_ITERS, &[]);
    let mut axes: Vec<Vec<f64>> = pca
        .comps
        .iter()
        .map(|comp| {
            matrix
                .iter()
                .map(|row| {
                    row.iter()
                        .zip(&pca.center)
                        .zip(comp)
                        .map(|((x, c), w)| (x - c) * w)
                        .sum()
                })
                .collect()
        })
        .collect();
    axes.resize(2, vec![0.0; n]);
    for axis in &mut axes {
        // 固有ベクトルの正負はデータで入れ替わるので、偏り（3乗の和）が正になる向きにそろえる。
        // 偏りが無ければ、最初の0でない点が正になる向き
        let skew: f64 = axis.iter().map(|v| v * v * v).sum();
        let first = axis
            .iter()
            .copied()
            .find(|v| v.abs() > 1e-12)
            .unwrap_or(0.0);
        let sign = if skew.abs() > 1e-12 {
            skew.signum()
        } else {
            first.signum()
        };
        let max = axis.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
        for v in axis.iter_mut() {
            *v = if max > 0.0 { *v * sign / max } else { 0.0 };
        }
    }
    // 表示には十分な桁に丸める（計算の誤差で、同じ地図の座標が最後の桁だけ揺れないように）
    let round = |v: f64| (v * 1e4).round() / 1e4 + 0.0;
    (0..n)
        .map(|i| [round(axes[0][i]), round(axes[1][i])])
        .collect()
}

/// タグの地図（`tag` が無ければ、タグの一覧だけ）
pub async fn topic_map(
    pool: &SqlitePool,
    municipality_id: &str,
    tag: Option<&str>,
) -> Result<TopicMap, sqlx::Error> {
    let closed = closed(pool, municipality_id).await?;
    let ids: Vec<String> = closed.topics.iter().map(|t| t.id.clone()).collect();
    let mut tags: Vec<MapTag> = Vec::new();
    let mut tagged: HashMap<String, Vec<String>> = HashMap::new();
    for id in &ids {
        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT g.id, g.kind, g.name FROM topic_tags tt JOIN tags g ON g.id = tt.tag_id
             WHERE tt.topic_id = $1",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;
        for (tag_id, kind, name) in rows {
            match tags.iter_mut().find(|t| t.id == tag_id) {
                Some(t) => t.count += 1,
                None => tags.push(MapTag {
                    id: tag_id.clone(),
                    kind,
                    name,
                    count: 1,
                }),
            }
            tagged.entry(tag_id).or_default().push(id.clone());
        }
    }
    // 分野を先に、多い順
    tags.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .reverse()
            .then(b.count.cmp(&a.count))
            .then(a.name.cmp(&b.name))
    });

    let Some(tag) = tag.filter(|t| tagged.contains_key(*t)) else {
        return Ok(TopicMap {
            tags,
            tag: None,
            points: Vec::new(),
            edges: Vec::new(),
        });
    };
    let members = &tagged[tag];
    let matrix = closed.index.matrix(members);
    let coords = layout(&matrix);
    let points = members
        .iter()
        .zip(coords)
        .filter_map(|(id, [x, y])| {
            let t = closed.get(id)?;
            Some(Point {
                id: t.id.clone(),
                title: t.title.clone(),
                closes_at: t.closes_at.clone(),
                participants: t.participants,
                x,
                y,
            })
        })
        .collect();
    let mut edges = Vec::new();
    for (i, a) in members.iter().enumerate() {
        for (j, b) in members.iter().enumerate().skip(i + 1) {
            if matrix[i][j] >= SIMILAR_TOPIC_SCORE {
                edges.push(Edge {
                    a: a.clone(),
                    b: b.clone(),
                    score: (matrix[i][j] * 1e4).round() / 1e4,
                });
            }
        }
    }
    Ok(TopicMap {
        tags,
        tag: Some(tag.to_owned()),
        points,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_deterministic_and_separates_clusters() {
        let index = Index::build([
            ("bus1", "路線バスの本数を増やす"),
            ("bus2", "路線バスの便を増やして通学を便利に"),
            ("lib1", "図書館の開館時間を夜まで延ばす"),
            ("lib2", "図書館を夜まで開館する"),
        ]);
        let ids: Vec<String> = ["bus1", "bus2", "lib1", "lib2"].map(String::from).to_vec();
        let a = layout(&index.matrix(&ids));
        let b = layout(&index.matrix(&ids));
        assert_eq!(a, b);
        // 同じ話題どうしは近く、違う話題とは遠い
        let d = |p: [f64; 2], q: [f64; 2]| ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt();
        assert!(d(a[0], a[1]) < d(a[0], a[2]));
        assert!(d(a[2], a[3]) < d(a[1], a[3]));
        assert!(a.iter().all(|p| p[0].abs() <= 1.0 && p[1].abs() <= 1.0));
    }

    #[test]
    fn layout_orientation_does_not_flip_with_order() {
        let index = Index::build([
            ("a", "路線バスの本数を増やす"),
            ("b", "路線バスの便を増やす"),
            ("c", "図書館の開館時間を延ばす"),
        ]);
        let forward: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
        let backward: Vec<String> = ["c", "b", "a"].map(String::from).to_vec();
        let f = layout(&index.matrix(&forward));
        let r = layout(&index.matrix(&backward));
        assert!((f[0][0] - r[2][0]).abs() < 1e-6, "{f:?} {r:?}");
    }

    #[test]
    fn tiny_maps() {
        assert!(layout(&[]).is_empty());
        assert_eq!(layout(&[vec![1.0]]), vec![[0.0, 0.0]]);
    }
}
