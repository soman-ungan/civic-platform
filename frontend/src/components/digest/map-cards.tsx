// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";

import { BUTTON_SECONDARY } from "@/components/ui/styles";
import type { MapPoint } from "@/lib/api/types/digest";
import { formatDate } from "@/lib/dates";

/** 点を押したときの概要。概要を押すとトピック詳細（合意点のページの上に重ねるレイヤ） */
export function PointPopup({
  slug,
  point,
  isAnchor,
  onAnchor,
  onClose,
}: {
  slug: string;
  point: MapPoint;
  isAnchor: boolean;
  onAnchor: () => void;
  onClose: () => void;
}) {
  return (
    <div className="absolute inset-x-2 bottom-2 rounded-2xl border border-zinc-200 bg-white p-3 shadow-lg dark:border-zinc-700 dark:bg-zinc-900">
      <div className="flex items-start gap-2">
        <Link href={`/${slug}/topics/${point.id}`} className="min-w-0 flex-1">
          <span className="block text-sm leading-snug font-medium underline-offset-2 hover:underline">
            {point.title}
          </span>
          <span className="mt-0.5 block text-xs text-zinc-500">
            {formatDate(point.closes_at)}に締め切り ・ 参加 {point.participants}{" "}
            人 ・ 押すと詳細
          </span>
        </Link>
        <button
          type="button"
          aria-label="閉じる"
          onClick={onClose}
          className="h-6 w-6 shrink-0 text-zinc-500"
        >
          ×
        </button>
      </div>
      <button
        type="button"
        onClick={onAnchor}
        className={`mt-2 h-8 w-full text-xs ${BUTTON_SECONDARY}`}
      >
        {isAnchor ? "起点を外す" : "ここを起点にする"}
      </button>
    </div>
  );
}

export function TopicCard({
  slug,
  point,
  role,
  onPress,
  onClearAnchor,
}: {
  slug: string;
  point: MapPoint;
  role: "anchor" | "selected" | "neighbor" | "plain";
  onPress: () => void;
  onClearAnchor: () => void;
}) {
  const label = {
    anchor: "起点",
    selected: "まとめに入れた",
    neighbor: "似ているトピック（押すとまとめに入れる）",
    plain: `${formatDate(point.closes_at)}に締め切り`,
  }[role];
  const border = {
    anchor: "border-zinc-900 dark:border-zinc-100",
    selected: "border-sky-600 bg-sky-50 dark:bg-sky-950/40",
    neighbor: "border-amber-400",
    plain: "border-zinc-200 dark:border-zinc-800",
  }[role];
  return (
    <li className={`flex items-stretch rounded-2xl border ${border}`}>
      <button
        type="button"
        onClick={onPress}
        aria-pressed={role === "anchor" || role === "selected"}
        className="min-w-0 flex-1 p-3 text-left"
      >
        <span className="text-xs text-zinc-500">
          {label} ・ 参加 {point.participants} 人
        </span>
        <span className="mt-0.5 block text-sm leading-snug font-medium">
          {point.title}
        </span>
      </button>
      <div className="flex shrink-0 flex-col items-end justify-center gap-1 pr-3">
        <Link
          href={`/${slug}/topics/${point.id}`}
          className="text-xs text-zinc-600 underline underline-offset-2 dark:text-zinc-400"
        >
          詳細
        </Link>
        {role === "anchor" && (
          <button
            type="button"
            onClick={onClearAnchor}
            className="text-xs text-zinc-500"
          >
            外す
          </button>
        )}
      </div>
    </li>
  );
}
