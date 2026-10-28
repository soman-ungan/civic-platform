// SPDX-License-Identifier: AGPL-3.0-only

/** データが無い欄の説明（中央寄せの薄い箱） */
export function EmptyState({ children }: { children: React.ReactNode }) {
  return (
    <p className="rounded-xl bg-zinc-50 px-4 py-10 text-center text-sm leading-6 text-zinc-500 dark:bg-zinc-900">
      {children}
    </p>
  );
}
