// SPDX-License-Identifier: AGPL-3.0-only

//! 文の似ている度合い（トピックと意見に共通の基盤。docs/plans/2026-10-30-opinion-synthesis.md）。
//!
//! - 類似の判定は2段階。ここで候補を絞り、同じ論点か・同じ意見かは AI か運営者が決める
//! - 文字の2-gram の TF-IDF とコサイン類似度。API キーも費用も要らず、保存もしない（その場で計算する）
//! - 言い換え（別の語で同じ意味）には弱い。件数が少ないうちは絞らずに全部を判定に回すこと
//! - 比べるのは同じ自治体の中だけ（呼ぶ側がコーパスを `municipality_id` で絞る）

use std::collections::HashMap;

use serde::Serialize;
use sqlx::SqlitePool;

/// 似ているトピック（タイトル＋説明）とみなす目安。運営者に候補として見せる・まとまりを作るのに使う。
/// 2026-10-30 の手元の DB（大田原市の75件）で、同じ論点の組はすべて 0.30 以上、別の論点は 0.19 以下だった
pub const SIMILAR_TOPIC_SCORE: f64 = 0.25;

/// 似ている意見（本文）とみなす目安（`list similar --topic` で見せる組）
pub const SIMILAR_STATEMENT_SCORE: f64 = 0.25;

/// 候補とその似ている度合い（0〜1）
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub id: String,
    pub score: f64,
}

/// 文の集まり（コーパス）。IDF はこの集まりから計算する
#[derive(Debug, Default)]
pub struct Index {
    ids: Vec<String>,
    vectors: Vec<Vec<(usize, f64)>>,
    grams: HashMap<(char, char), usize>,
    idf: Vec<f64>,
}

/// 全角の英数字・記号を半角に、大文字を小文字にし、文字（かな・漢字・英数字）だけを残す
pub fn normalize(text: &str) -> Vec<char> {
    text.chars()
        .map(|c| match c {
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
            _ => c,
        })
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// 文字の2-gram の出現回数。1文字だけの文は、その1文字を2-gram として扱う
fn bigrams(text: &str) -> HashMap<(char, char), f64> {
    let chars = normalize(text);
    let mut out = HashMap::new();
    if chars.len() == 1 {
        out.insert((chars[0], chars[0]), 1.0);
    }
    for w in chars.windows(2) {
        *out.entry((w[0], w[1])).or_insert(0.0) += 1.0;
    }
    out
}

fn normalized(mut v: Vec<(usize, f64)>) -> Vec<(usize, f64)> {
    let norm = v.iter().map(|(_, w)| w * w).sum::<f64>().sqrt();
    if norm > 0.0 {
        for (_, w) in &mut v {
            *w /= norm;
        }
    }
    v.sort_by_key(|(i, _)| *i);
    v
}

/// 並びが添字の順の疎ベクトルどうしの内積
fn dot(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
    let (mut i, mut j, mut sum) = (0, 0, 0.0);
    while i < a.len() && j < b.len() {
        match a[i].0.cmp(&b[j].0) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                sum += a[i].1 * b[j].1;
                i += 1;
                j += 1;
            }
        }
    }
    sum
}

impl Index {
    /// `(id, 本文)` の集まりから作る
    pub fn build<I, S, T>(docs: I) -> Self
    where
        I: IntoIterator<Item = (S, T)>,
        S: Into<String>,
        T: AsRef<str>,
    {
        let mut ids = Vec::new();
        let mut counts = Vec::new();
        let mut grams: HashMap<(char, char), usize> = HashMap::new();
        let mut df: Vec<f64> = Vec::new();
        for (id, text) in docs {
            let c = bigrams(text.as_ref());
            for g in c.keys() {
                let next = grams.len();
                let i = *grams.entry(*g).or_insert(next);
                if i == df.len() {
                    df.push(0.0);
                }
                df[i] += 1.0;
            }
            ids.push(id.into());
            counts.push(c);
        }
        let n = ids.len() as f64;
        // 滑らかにした IDF（どの文にも出る語も 0 にしない）
        let idf: Vec<f64> = df
            .iter()
            .map(|d| ((n + 1.0) / (d + 1.0)).ln() + 1.0)
            .collect();
        let vectors = counts
            .into_iter()
            .map(|c| {
                normalized(
                    c.into_iter()
                        .map(|(g, tf)| (grams[&g], tf * idf[grams[&g]]))
                        .collect(),
                )
            })
            .collect();
        Self {
            ids,
            vectors,
            grams,
            idf,
        }
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// コーパスの外の文をベクトルにする（コーパスに無い 2-gram は数えない）
    fn vectorize(&self, text: &str) -> Vec<(usize, f64)> {
        normalized(
            bigrams(text)
                .into_iter()
                .filter_map(|(g, tf)| self.grams.get(&g).map(|&i| (i, tf * self.idf[i])))
                .collect(),
        )
    }

    fn ranked(&self, scores: impl Iterator<Item = (usize, f64)>, k: usize) -> Vec<Match> {
        let mut v: Vec<(usize, f64)> = scores.filter(|(_, s)| *s > 0.0).collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(k);
        v.into_iter()
            .map(|(i, score)| Match {
                id: self.ids[i].clone(),
                score,
            })
            .collect()
    }

    /// `query` に似ている順の上位 `k` 件（似ている度合いが 0 のものは除く）
    pub fn top_k(&self, query: &str, k: usize) -> Vec<Match> {
        let q = self.vectorize(query);
        self.ranked(
            self.vectors
                .iter()
                .enumerate()
                .map(|(i, v)| (i, dot(&q, v))),
            k,
        )
    }

    /// コーパスの中の文 `id` に似ている順の上位 `k` 件（自分は除く）
    pub fn similar_to(&self, id: &str, k: usize) -> Vec<Match> {
        let Some(me) = self.ids.iter().position(|x| x == id) else {
            return Vec::new();
        };
        let q = &self.vectors[me];
        self.ranked(
            self.vectors
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != me)
                .map(|(i, v)| (i, dot(q, v))),
            k,
        )
    }

    /// 似ている度合いが `min_score` 以上の組（似ている順）
    pub fn pairs(&self, min_score: f64) -> Vec<(String, String, f64)> {
        let mut out = Vec::new();
        for i in 0..self.vectors.len() {
            for j in i + 1..self.vectors.len() {
                let s = dot(&self.vectors[i], &self.vectors[j]);
                if s >= min_score {
                    out.push((self.ids[i].clone(), self.ids[j].clone(), s));
                }
            }
        }
        out.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
        out
    }

    /// 似ている組をつないだまとまり（2件以上のものだけ。まとまりの中はコーパスの順、まとまりは大きい順）
    pub fn groups(&self, min_score: f64) -> Vec<Vec<String>> {
        let index: HashMap<&str, usize> = self
            .ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();
        let mut parent: Vec<usize> = (0..self.ids.len()).collect();
        fn find(p: &mut [usize], x: usize) -> usize {
            let mut r = x;
            while p[r] != r {
                r = p[r];
            }
            p[x] = r;
            r
        }
        for (a, b, _) in self.pairs(min_score) {
            let (ra, rb) = (
                find(&mut parent, index[a.as_str()]),
                find(&mut parent, index[b.as_str()]),
            );
            if ra != rb {
                parent[ra.max(rb)] = ra.min(rb);
            }
        }
        let mut by_root: Vec<Vec<String>> = vec![Vec::new(); self.ids.len()];
        for i in 0..self.ids.len() {
            let r = find(&mut parent, i);
            by_root[r].push(self.ids[i].clone());
        }
        let mut out: Vec<Vec<String>> = by_root.into_iter().filter(|g| g.len() >= 2).collect();
        out.sort_by_key(|g| std::cmp::Reverse(g.len()));
        out
    }
}

/// 似ているトピック（運営者に見せる候補）
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimilarTopic {
    pub id: String,
    pub title: String,
    /// draft / published
    pub status: String,
    pub score: f64,
}

#[derive(sqlx::FromRow)]
struct TopicRow {
    id: String,
    title: String,
    description: String,
    status: String,
}

/// 自治体の、取り下げていないトピック（下書き・公開中・締め切ったもの）
async fn topic_rows(
    pool: &SqlitePool,
    municipality_id: &str,
) -> Result<Vec<TopicRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, title, description, status FROM topics
         WHERE municipality_id = $1 AND status IN ('draft', 'published') ORDER BY created_at, id",
    )
    .bind(municipality_id)
    .fetch_all(pool)
    .await
}

fn topic_index(rows: &[TopicRow]) -> Index {
    Index::build(
        rows.iter()
            .map(|t| (t.id.clone(), format!("{} {}", t.title, t.description))),
    )
}

/// 自治体の中で、似ているトピックの組（`SIMILAR_TOPIC_SCORE` 以上。似ている順）
pub async fn topic_pairs(
    pool: &SqlitePool,
    municipality_id: &str,
) -> Result<Vec<(SimilarTopic, SimilarTopic)>, sqlx::Error> {
    let rows = topic_rows(pool, municipality_id).await?;
    let index = topic_index(&rows);
    let find = |id: &str, score: f64| {
        let t = rows.iter().find(|t| t.id == id).expect("コーパスの id");
        SimilarTopic {
            id: t.id.clone(),
            title: t.title.clone(),
            status: t.status.clone(),
            score,
        }
    };
    Ok(index
        .pairs(SIMILAR_TOPIC_SCORE)
        .into_iter()
        .map(|(a, b, s)| (find(&a, s), find(&b, s)))
        .collect())
}

/// 各トピックに似ている、同じ自治体のほかのトピック（`SIMILAR_TOPIC_SCORE` 以上を、最大 `k` 件ずつ）。
/// 管理画面の下書きの確認で、同じ論点の重なりを確かめるのに使う
pub async fn similar_topics(
    pool: &SqlitePool,
    municipality_id: &str,
    topic_ids: &[String],
    k: usize,
) -> Result<HashMap<String, Vec<SimilarTopic>>, sqlx::Error> {
    let rows = topic_rows(pool, municipality_id).await?;
    let index = topic_index(&rows);
    Ok(topic_ids
        .iter()
        .map(|id| {
            let similar = index
                .similar_to(id, k)
                .into_iter()
                .filter(|m| m.score >= SIMILAR_TOPIC_SCORE)
                .filter_map(|m| {
                    rows.iter().find(|t| t.id == m.id).map(|t| SimilarTopic {
                        id: t.id.clone(),
                        title: t.title.clone(),
                        status: t.status.clone(),
                        score: m.score,
                    })
                })
                .collect();
            (id.clone(), similar)
        })
        .collect())
}

/// トピックの中で、似ている意見の組（承認済み・確認待ち。`min_score` 以上、似ている順）。
/// 返すのは (id, 本文) の組と度合い
pub async fn statement_pairs(
    pool: &SqlitePool,
    topic_id: &str,
    min_score: f64,
) -> Result<Vec<((String, String), (String, String), f64)>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, body FROM statements
         WHERE topic_id = $1 AND status IN ('approved', 'flagged', 'pending') ORDER BY created_at, id",
    )
    .bind(topic_id)
    .fetch_all(pool)
    .await?;
    let index = Index::build(rows.iter().map(|(id, body)| (id.clone(), body.as_str())));
    let body: HashMap<&str, &str> = rows
        .iter()
        .map(|(id, b)| (id.as_str(), b.as_str()))
        .collect();
    Ok(index
        .pairs(min_score)
        .into_iter()
        .map(|(a, b, s)| {
            let (ba, bb) = (body[a.as_str()].to_owned(), body[b.as_str()].to_owned());
            ((a, ba), (b, bb), s)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Index {
        Index::build([
            ("bus", "市内の路線バスを増便し、高校生の通学を便利にする"),
            ("bus2", "路線バスの本数を増やして、通学しやすくする"),
            ("lib", "図書館の開館時間を夜まで延ばす"),
            ("park", "公園に子どもの遊具を増やす"),
            ("tax", "ふるさと納税の返礼品を見直す"),
        ])
    }

    #[test]
    fn normalizes_width_and_case_and_drops_symbols() {
        assert_eq!(
            normalize("ＡＢＣ１２３、ｄｅｆ！ abc"),
            "abc123defabc".chars().collect::<Vec<_>>()
        );
    }

    #[test]
    fn ranks_the_same_issue_first() {
        let index = corpus();
        let top = index.top_k("バスを増便して通学を楽に", 2);
        assert_eq!(top.len(), 2);
        assert!(top.iter().all(|m| m.id.starts_with("bus")));
        assert!(
            index
                .top_k("まったく関係のない話題", 5)
                .iter()
                .all(|m| m.score < 0.2)
        );
    }

    #[test]
    fn similar_to_excludes_itself() {
        let index = corpus();
        let top = index.similar_to("bus", 1);
        assert_eq!(top[0].id, "bus2");
        assert!(index.similar_to("missing", 3).is_empty());
    }

    #[test]
    fn identical_texts_score_one() {
        let index = Index::build([
            ("a", "図書館を夜まで開ける"),
            ("b", "図書館を夜まで開ける。"),
        ]);
        let pairs = index.pairs(0.99);
        assert_eq!(pairs.len(), 1);
        assert!((pairs[0].2 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn groups_connect_similar_pairs() {
        let index = corpus();
        let groups = index.groups(0.2);
        assert_eq!(groups, vec![vec!["bus".to_string(), "bus2".to_string()]]);
    }

    #[test]
    fn empty_and_tiny_texts_do_not_panic() {
        let index = Index::build([("a", ""), ("b", "市")]);
        assert_eq!(index.len(), 2);
        assert!(index.top_k("", 3).is_empty());
        assert_eq!(index.top_k("市", 3)[0].id, "b");
    }
}
