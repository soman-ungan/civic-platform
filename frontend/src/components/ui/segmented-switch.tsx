// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

/** 2〜3個の並びの切り替え（URL を変える。履歴は増やさない） */
export function SegmentedSwitch<K extends string>({
  label,
  options,
  current,
}: {
  /** role="group" の aria-label */
  label: string;
  options: { key: K; label: string; href: string }[];
  current: K;
}) {
  return (
    <span
      role="group"
      aria-label={label}
      className="inline-flex rounded-full border border-zinc-300 p-0.5 dark:border-zinc-700"
    >
      {options.map((o) => (
        <Link
          key={o.key}
          href={o.href}
          scroll={false}
          replace
          aria-current={o.key === current ? "true" : undefined}
          className={`inline-flex min-h-8 items-center rounded-full px-3 ${
            o.key === current
              ? "bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-900"
              : "text-zinc-600 dark:text-zinc-400"
          }`}
        >
          {o.label}
        </Link>
      ))}
    </span>
  );
}
