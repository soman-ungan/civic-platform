// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

/**
 * 絞り込みや選択のチップ（押した状態は黒）。`md` はトピックのタグ、`sm` は年度・分野など
 * （`sm` は色見本などを並べられるように inline-flex）
 */
export function chipClass(active: boolean, size: "sm" | "md" = "sm"): string {
  const shape =
    size === "md"
      ? "h-9 whitespace-nowrap rounded-full border px-3.5 text-sm transition-colors"
      : "inline-flex min-h-9 items-center rounded-full border px-3 text-xs whitespace-nowrap";
  const color = active
    ? "border-zinc-900 bg-zinc-900 text-white dark:border-zinc-100 dark:bg-zinc-100 dark:text-zinc-900"
    : size === "md"
      ? "border-zinc-300 bg-white text-zinc-700 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-300"
      : "border-zinc-300 text-zinc-700 dark:border-zinc-700 dark:text-zinc-300";
  return `${shape} ${color}`;
}

/** 押すと絞り込みが変わるチップ（aria-pressed） */
export function ChipButton({
  active,
  onClick,
  size,
  className = "",
  children,
}: {
  active: boolean;
  onClick: () => void;
  size?: "sm" | "md";
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={`${chipClass(active, size)} ${className}`}
    >
      {children}
    </button>
  );
}

/** URL を変えるチップ（aria-current。年度など） */
export function ChipLink({
  active,
  href,
  className = "",
  ref,
  children,
}: {
  active: boolean;
  href: string;
  className?: string;
  ref?: React.Ref<HTMLAnchorElement>;
  children: React.ReactNode;
}) {
  return (
    <Link
      ref={ref}
      href={href}
      scroll={false}
      aria-current={active ? "page" : undefined}
      className={`${chipClass(active)} ${className}`}
    >
      {children}
    </Link>
  );
}
