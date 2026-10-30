// SPDX-License-Identifier: AGPL-3.0-only

// トピックマップと市民のまとめの表示の決まり（ADR 0005「市民のまとめ」）

const enc = encodeURIComponent;

/** 締め切った日（日本時間の「2026年10月30日」） */
export function formatDate(iso: string): string {
  return new Intl.DateTimeFormat("ja-JP", {
    timeZone: "Asia/Tokyo",
    year: "numeric",
    month: "long",
    day: "numeric",
  }).format(new Date(iso));
}

/** 基準の割合の表記（0.6 → 「6割」） */
export function ratioLabel(value: number): string {
  return `${Math.round(value * 10)}割`;
}

/** 地図の欄の id（合意点のページの中の位置） */
export const MAP_ANCHOR = "topic-map";

/** 合意点のページの地図（タグを選ぶ、または合意点の元のトピックを起点にして開く） */
export function consensusMapHref(
  slug: string,
  by: { tag: string } | { focus: string },
): string {
  const q = "tag" in by ? `tag=${enc(by.tag)}` : `focus=${enc(by.focus)}`;
  return `/${slug}/consensus?${q}#${MAP_ANCHOR}`;
}

/** まとめのページ /{slug}/digest?anchor=&topics=（起点と、近傍から選んだトピック） */
export function digestHref(
  slug: string,
  anchor: string,
  topics: string[],
): string {
  return `/${slug}/digest?anchor=${enc(anchor)}&topics=${topics.map(enc).join(",")}`;
}

/** まとめから提案するページ（元のトピックと、最初の意見に使う原文の意見の ID を渡す） */
export function proposeFromDigestHref(
  slug: string,
  anchor: string,
  topics: string[],
  use: string[],
): string {
  return `/${slug}/proposals?anchor=${enc(anchor)}&topics=${topics.map(enc).join(",")}&use=${use.map(enc).join(",")}`;
}

/** URL の「a,b,c」を配列に */
export function idList(value: string | string[] | undefined): string[] {
  const s = Array.isArray(value) ? value.join(",") : (value ?? "");
  return s
    .split(",")
    .map((x) => x.trim())
    .filter((x) => x !== "");
}
