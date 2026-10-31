// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

import type { MapTag } from "@/lib/api/types/digest";
import { consensusMapHref } from "@/lib/digest";

/** 地図を開く前のタグの一覧 */
export function MapTagList({ slug, tags }: { slug: string; tags: MapTag[] }) {
  if (tags.length === 0) {
    return (
      <p className="rounded-xl bg-zinc-50 px-4 py-10 text-center text-sm text-zinc-500 dark:bg-zinc-900">
        まだ締め切ったトピックはありません
      </p>
    );
  }
  return (
    <ul className="grid grid-cols-2 gap-2">
      {tags.map((t) => (
        <li key={t.id}>
          <Link
            href={consensusMapHref(slug, { tag: t.id })}
            className="flex h-full flex-col rounded-2xl border border-zinc-200 p-3 dark:border-zinc-800"
          >
            <span className="text-sm font-medium">
              {t.kind === "district" ? `#${t.name}` : t.name}
            </span>
            <span className="mt-0.5 text-xs text-zinc-500">
              締め切ったトピック {t.count} 件
            </span>
          </Link>
        </li>
      ))}
    </ul>
  );
}
