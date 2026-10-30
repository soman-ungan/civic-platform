// SPDX-License-Identifier: AGPL-3.0-only

// 自治体の画面のタブ（タブの表示と、横フリックでの切り替えの両方で使う）

/**
 * 自治体のトップのタブ（URL の最初の部分が自治体の slug）。
 * 並び順がスワイプの向きになる（左のタブ → 右のタブ）
 */
export function tabsFor(pathname: string) {
  const slug = pathname.split("/")[1] ?? "";
  // swipe: その画面の横フリックで隣のタブへ移るか。まちのお金は、横フリックを欄の切り替えに使う（ユーザーと決定）
  const tabs = [
    { href: `/${slug}`, label: "トピック", swipe: true },
    { href: `/${slug}/finances`, label: "まちのお金", swipe: false },
  ];
  return { tabs, current: tabs.findIndex((t) => t.href === pathname) };
}

/** 自治体名とタブの固定のヘッダーの id（トピック詳細のレイヤの上端をそろえる） */
export const TABS_HEADER_ID = "tabs-header";

/** 右のタブへは nav-forward、左のタブへは nav-back（globals.css のスライドの向き） */
export function transitionTo(from: number, to: number): string[] {
  return [to > from ? "nav-forward" : "nav-back"];
}
