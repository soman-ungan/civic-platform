// SPDX-License-Identifier: AGPL-3.0-only

import { ChipLink } from "@/components/ui/chip";
import { SegmentedSwitch } from "@/components/ui/segmented-switch";
import type { Municipality } from "@/lib/api/types/municipalities";
import { OPERATOR_TABS, type OperatorTab, operatorHref } from "@/lib/operator";

import { Drafts } from "./drafts";
import { Published } from "./published";
import { Queue } from "./queue";
import { STICKY_TABS } from "./sticky";

/** 管理画面の枠: 自治体の切り替え（2つ以上のとき）とタブ */
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
    <div className="mx-auto w-full max-w-md px-4 pt-4 pb-8">
      <h1 className="text-lg font-semibold">管理画面</h1>
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
      <div className={`mt-1 text-sm ${STICKY_TABS}`}>
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
      <div className="mt-4">
        {tab === "drafts" && <Drafts key={slug} slug={slug} />}
        {tab === "queue" && <Queue key={slug} slug={slug} />}
        {tab === "published" && <Published key={slug} slug={slug} />}
      </div>
    </div>
  );
}
