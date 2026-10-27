// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

/**
 * 自治体のトップのタブ（URL の最初の部分が自治体の slug）。
 * 並び順がスワイプの向きになる（左のタブ → 右のタブ）
 */
export function tabsFor(pathname: string) {
  const slug = pathname.split("/")[1] ?? "";
  const tabs = [
    { href: `/${slug}`, label: "トピック" },
    { href: `/${slug}/finances`, label: "まちのお金" },
  ];
  return { tabs, current: tabs.findIndex((t) => t.href === pathname) };
}

/** 右のタブへは nav-forward、左のタブへは nav-back（globals.css のスライドの向き） */
export function transitionTo(from: number, to: number): string[] {
  return [to > from ? "nav-forward" : "nav-back"];
}

/** タブ「トピック｜まちのお金」。URL を読むので、レイアウトでは Suspense の中に置く */
export function TabNav() {
  return <Tabs pathname={usePathname()} />;
}

/** URL を読む前に一瞬だけ出す、同じ形のタブ（Suspense の fallback。まだ押せない） */
export function TabNavFallback() {
  return (
    <div aria-hidden className="flex">
      {["トピック", "まちのお金"].map((label) => (
        <span
          key={label}
          className="flex h-11 flex-1 items-center justify-center border-b-2 border-transparent text-sm text-zinc-500"
        >
          {label}
        </span>
      ))}
    </div>
  );
}

function Tabs({ pathname }: { pathname: string }) {
  const { tabs, current } = tabsFor(pathname);
  return (
    <nav aria-label="ページ" className="flex">
      {tabs.map((tab, i) => {
        const active = i === current;
        return (
          <Link
            key={tab.href}
            href={tab.href}
            aria-current={active ? "page" : undefined}
            transitionTypes={
              current >= 0 && !active ? transitionTo(current, i) : undefined
            }
            className={`flex h-11 flex-1 items-center justify-center border-b-2 text-sm ${
              active
                ? "border-zinc-900 font-semibold dark:border-zinc-100"
                : "border-transparent text-zinc-500"
            }`}
          >
            {tab.label}
          </Link>
        );
      })}
    </nav>
  );
}
