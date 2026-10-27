// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 開閉できる欄（まちのお金）。見出しの右の ▼ と「タップで開く」で、開けることがひと目でわかるようにする
 */
export function FinanceSection({
  title,
  lead,
  defaultOpen = false,
  children,
}: {
  title: string;
  /** 見出しの下の短い説明（開いたときだけ出す） */
  lead?: React.ReactNode;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  return (
    <details
      open={defaultOpen}
      className="group border-b border-zinc-200 dark:border-zinc-800"
    >
      <summary className="flex min-h-12 cursor-pointer list-none items-center justify-between gap-2 py-2 [&::-webkit-details-marker]:hidden">
        <h2 className="font-semibold">{title}</h2>
        <span className="flex items-center gap-1.5 text-xs text-zinc-500">
          <span className="group-open:hidden">タップで開く</span>
          <span
            aria-hidden
            className="inline-block transition-transform group-open:rotate-180"
          >
            ▼
          </span>
        </span>
      </summary>
      <div className="pb-6">
        {lead && <p className="mb-3 text-xs leading-5 text-zinc-500">{lead}</p>}
        {children}
      </div>
    </details>
  );
}
