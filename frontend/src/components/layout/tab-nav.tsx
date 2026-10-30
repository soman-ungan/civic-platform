// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

import { tabsFor, transitionTo } from "@/lib/tabs";

/** タブ「トピック｜提言｜まちのお金」。URL を読むので、レイアウトでは Suspense の中に置く */
export function TabNav() {
  return <Tabs pathname={usePathname()} />;
}

/** URL を読む前に一瞬だけ出す、同じ形のタブ（Suspense の fallback。まだ押せない） */
export function TabNavFallback() {
  return (
    <div aria-hidden className="flex">
      {tabsFor("").tabs.map(({ label }) => (
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
