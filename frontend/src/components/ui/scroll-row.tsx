// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 横にスクロールするチップなどの行（画面の端まで広げる）。
 * 横のスクロールをタブやカルーセルのフリックと取り違えないように `data-swipe-ignore`。
 * 中身は `<li>` で渡す
 */
export function ScrollRow({
  label,
  className = "",
  listClassName = "gap-1.5",
  children,
}: {
  /** nav の aria-label */
  label: string;
  className?: string;
  /** ul に足すクラス（間隔など） */
  listClassName?: string;
  children: React.ReactNode;
}) {
  return (
    <nav
      aria-label={label}
      data-swipe-ignore
      className={`-mx-4 overflow-x-auto px-4 [scrollbar-width:none] ${className}`}
    >
      <ul className={`flex w-max ${listClassName}`}>{children}</ul>
    </nav>
  );
}
