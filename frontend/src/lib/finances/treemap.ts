// SPDX-License-Identifier: AGPL-3.0-only

// ツリーマップの配置（squarified。Bruls, Huizing, van Wijk 2000）。
// 外部のライブラリを使わず、四角がなるべく正方形に近くなるように並べる

export type Rect = { x: number; y: number; w: number; h: number };

/**
 * `values`（大きい順に並べておく）を、幅 `width`・高さ `height` の中に面積の比で並べる。
 * 返す四角は `values` と同じ順
 */
export function squarify(
  values: number[],
  width: number,
  height: number,
): Rect[] {
  const total = values.reduce((a, v) => a + v, 0);
  if (total <= 0 || width <= 0 || height <= 0) {
    return values.map(() => ({ x: 0, y: 0, w: 0, h: 0 }));
  }
  const scale = (width * height) / total;
  const areas = values.map((v) => v * scale);
  const out: Rect[] = [];
  let free: Rect = { x: 0, y: 0, w: width, h: height };
  let row: number[] = [];

  // 短い辺に沿って `row` を並べたときの、いちばん細長い四角の縦横比
  const worst = (r: number[], side: number) => {
    const sum = r.reduce((a, v) => a + v, 0);
    const max = Math.max(...r);
    const min = Math.min(...r);
    return Math.max(
      (side * side * max) / (sum * sum),
      (sum * sum) / (side * side * min),
    );
  };

  const layRow = (r: number[]) => {
    const sum = r.reduce((a, v) => a + v, 0);
    if (free.w >= free.h) {
      // 左に縦の列として並べる
      const w = sum / free.h;
      let y = free.y;
      for (const a of r) {
        const h = a / w;
        out.push({ x: free.x, y, w, h });
        y += h;
      }
      free = { x: free.x + w, y: free.y, w: free.w - w, h: free.h };
    } else {
      // 上に横の行として並べる
      const h = sum / free.w;
      let x = free.x;
      for (const a of r) {
        const w = a / h;
        out.push({ x, y: free.y, w, h });
        x += w;
      }
      free = { x: free.x, y: free.y + h, w: free.w, h: free.h - h };
    }
  };

  for (const a of areas) {
    if (a <= 0) {
      continue;
    }
    const side = Math.min(free.w, free.h);
    if (row.length === 0 || worst([...row, a], side) <= worst(row, side)) {
      row.push(a);
    } else {
      layRow(row);
      row = [a];
    }
  }
  if (row.length > 0) layRow(row);

  // 0 の値には大きさの無い四角を返し、順番をそろえる
  let i = 0;
  return areas.map((a) => (a > 0 ? out[i++] : { x: 0, y: 0, w: 0, h: 0 }));
}
