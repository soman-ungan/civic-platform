// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import { ChipLink } from "@/components/ui/chip";
import { SegmentedSwitch } from "@/components/ui/segmented-switch";
import type { Municipality } from "@/lib/api/types/municipalities";
import { OPERATOR_TABS, type OperatorTab, operatorHref } from "@/lib/operator";

import { Drafts } from "./drafts";
import { Published } from "./published";
import { Queue } from "./queue";

/**
 * 管理画面の枠: 見出し・自治体の切り替え（2つ以上のとき）・タブは固定のヘッダーにし、
 * その下だけをスクロールする（自治体のトップと同じ形）
 */
export function OperatorConsole({
  municipalities,
  municipality,
  tab,
}: {
  municipalities: Municipality[];
  municipality: Municipality;
  tab: OperatorTab;
}) {
  const slug = municipality.slug;
  return (
    <>
      <header className="shrink-0 border-b border-zinc-200 dark:border-zinc-800">
        <div className="mx-auto w-full max-w-md px-4 pt-3 pb-3">
          <div className="flex items-baseline justify-between gap-4">
            <h1 className="text-lg font-semibold">管理画面</h1>
            {/* 準備中の自治体には公開の画面が無いので、トップ（公開中の自治体へ振り分ける）に戻る */}
            <Link
              href="/"
              className="text-sm text-zinc-600 underline underline-offset-2 dark:text-zinc-400"
            >
              通常の画面へ
            </Link>
          </div>
          {municipalities.length > 1 ? (
            <nav aria-label="自治体" className="mt-2 flex flex-wrap gap-2">
              {municipalities.map((m) => (
                <ChipLink
                  key={m.slug}
                  active={m.slug === slug}
                  href={operatorHref(m.slug, tab)}
                >
                  {m.name}
                </ChipLink>
              ))}
            </nav>
          ) : (
            <p className="text-sm text-zinc-500">{municipality.name}</p>
          )}
          <div className="mt-3 text-sm">
            <SegmentedSwitch
              label="管理する内容"
              current={tab}
              options={OPERATOR_TABS.map((t) => ({
                key: t.key,
                label: t.label,
                href: operatorHref(slug, t.key),
              }))}
            />
          </div>
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto w-full max-w-md px-4 pt-4 pb-8">
          {tab === "drafts" && <Drafts key={slug} slug={slug} />}
          {tab === "queue" && <Queue key={slug} slug={slug} />}
          {tab === "published" && <Published key={slug} slug={slug} />}
        </div>
      </div>
    </>
  );
}
